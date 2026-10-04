//! Batch id minting and the worker's flush loop.
//!
//! `run_worker` is a free function taking its dependencies so it can
//! be driven directly from tests without spawning a thread; Task 6
//! wraps it in one.

use std::sync::Arc;
use std::time::Duration;

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender};
use ulid::Ulid;

use super::pending::PendingStore;
use super::sender::LogSender;

/// Mint a batch id: `{device_id}-{ULID}`.
///
/// The ULID half is what makes the id unique within a process (a
/// 48-bit millisecond timestamp plus an 80-bit monotonic counter and
/// random component) and lexicographically time-sortable, which is
/// how the pending dir finds the newest batch without reading mtimes.
///
/// Uniqueness is not cosmetic. The server rejects a repeated id with
/// `409 duplicate_batch_id`, and the drain stops at its first
/// failure — a collision would re-persist unchanged and wedge the
/// queue on every future cycle.
pub(crate) fn mint_batch_id(device_id: &str) -> String {
    // The batch id becomes a filename, so anything a filesystem
    // could read as a path separator (or as anything else special)
    // is folded to `_`.
    let safe: String = device_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let safe = if safe.is_empty() {
        "device".to_string()
    } else {
        safe
    };
    format!("{safe}-{}", Ulid::generate())
}

/// Everything the worker needs that is not the channel.
pub(crate) struct WorkerCtx {
    pub(crate) device_id: String,
    pub(crate) batch_size: usize,
    pub(crate) interval: Duration,
    pub(crate) pending: PendingStore,
    pub(crate) sender: Arc<dyn LogSender>,
}

/// Body of the worker task. Runs on the submitter's own
/// current-thread runtime.
///
/// The blocking `recv_timeout` is deliberate and safe here: this
/// runtime exists to run this one task, and the block is only
/// reached between `send().await` calls, so an in-flight submit
/// never stalls the receiver — it parks in the crossbeam buffer
/// until the loop comes back around.
pub(crate) async fn run_worker(rx: Receiver<String>, ctx: WorkerCtx, done_tx: Sender<()>) {
    let mut batch: Vec<String> = Vec::with_capacity(ctx.batch_size);
    loop {
        match rx.recv_timeout(ctx.interval) {
            Ok(entry) => {
                batch.push(entry);
                if batch.len() >= ctx.batch_size {
                    flush(&ctx, &mut batch).await;
                }
            }
            Err(RecvTimeoutError::Timeout) => flush(&ctx, &mut batch).await,
            Err(RecvTimeoutError::Disconnected) => {
                flush(&ctx, &mut batch).await;
                break;
            }
        }
    }
    let _ = done_tx.send(());
}

/// Drain the pending directory newest-first, then submit `batch`.
///
/// The drain stops at its first failure: newer-first ordering means
/// a batch the server will never accept would otherwise block every
/// older one and every future batch behind it. The consequence is
/// head-of-line blocking during a sustained outage, which the spec
/// accepts in exchange for the ordering guarantee.
async fn flush(ctx: &WorkerCtx, batch: &mut Vec<String>) {
    for batch_id in ctx.pending.list_newest_first() {
        let Ok(entries) = ctx.pending.load(&batch_id) else {
            // Unreadable or corrupt — it will never become
            // submittable, so drop it rather than retry it forever.
            ctx.pending.remove(&batch_id);
            continue;
        };
        match ctx.sender.send(&batch_id, entries).await {
            Ok(()) => ctx.pending.remove(&batch_id),
            Err(err) => {
                if ctx.sender.is_already_ingested(&err) {
                    // The sink already holds this batch, so the
                    // entries are not lost — the file is just
                    // stale. Delete it and keep draining.
                    ctx.pending.remove(&batch_id);
                    continue;
                }
                return;
            }
        }
    }

    if batch.is_empty() {
        return;
    }
    let batch_id = mint_batch_id(&ctx.device_id);
    let entries = std::mem::take(batch);
    if let Err(err) = ctx.sender.send(&batch_id, entries.clone()).await {
        if ctx.sender.is_already_ingested(&err) {
            return;
        }
        if let Err(persist_err) = ctx.pending.persist(&batch_id, &entries) {
            // The disk refused the batch. There is nowhere left to
            // keep it, and retrying in memory would grow without
            // bound, so it is dropped — loudly, because a silent
            // loss is the worst outcome here.
            tracing::warn!(
                batch_id = %batch_id,
                error = %persist_err,
                send_error = %err,
                "log batch could not be sent or persisted; dropping it"
            );
        }
    }
}

#[cfg(test)]
pub(crate) mod tests_support {
    use super::super::config::LogSubmitterConfig;
    use super::super::sender::SubmitterError;
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};

    /// Records every accepted batch, and can be told to fail for
    /// specific batch ids (or all of them).
    #[derive(Debug, Default)]
    pub(crate) struct RecordingSender {
        accepted: Mutex<Vec<(String, Vec<String>)>>,
        fail_ids: Mutex<Vec<String>>,
        fail_all: AtomicBool,
    }

    impl RecordingSender {
        pub(crate) fn shared() -> Arc<RecordingSender> {
            Arc::new(RecordingSender::default())
        }
        pub(crate) fn accepted(&self) -> Vec<(String, Vec<String>)> {
            self.accepted.lock().unwrap().clone()
        }
        pub(crate) fn fail_for(&self, ids: &[&str]) {
            *self.fail_ids.lock().unwrap() = ids.iter().map(|s| s.to_string()).collect();
        }
        pub(crate) fn fail_everything(&self) {
            self.fail_all.store(true, Ordering::Release);
        }
    }

    #[async_trait::async_trait]
    impl LogSender for RecordingSender {
        async fn send(
            &self,
            batch_id: &str,
            log_entries: Vec<String>,
        ) -> Result<(), SubmitterError> {
            if self.fail_all.load(Ordering::Acquire)
                || self.fail_ids.lock().unwrap().iter().any(|i| i == batch_id)
            {
                return Err(SubmitterError::Send {
                    batch_id: batch_id.to_string(),
                    source: Box::new(std::io::Error::other("scripted failure")),
                });
            }
            self.accepted
                .lock()
                .unwrap()
                .push((batch_id.to_string(), log_entries));
            Ok(())
        }
    }

    /// Fails every batch as "already ingested" — the shape a server
    /// takes when it has the batch but the response was lost.
    #[derive(Debug, Default)]
    pub(crate) struct AlreadyIngestedSender {
        pub(crate) seen: Mutex<Vec<String>>,
    }

    #[async_trait::async_trait]
    impl LogSender for AlreadyIngestedSender {
        async fn send(
            &self,
            batch_id: &str,
            _log_entries: Vec<String>,
        ) -> Result<(), SubmitterError> {
            self.seen.lock().unwrap().push(batch_id.to_string());
            Err(SubmitterError::Send {
                batch_id: batch_id.to_string(),
                source: Box::new(std::io::Error::other("409 duplicate_batch_id")),
            })
        }

        fn is_already_ingested(&self, _err: &SubmitterError) -> bool {
            true
        }
    }

    /// A `LogSubmitterConfig` wired to a fresh temp dir, with test-
    /// sized timings. The deadline is generous so a loaded CI box
    /// does not produce flakes.
    pub(crate) fn make_config(
        sender: Arc<dyn LogSender>,
        batch_size: usize,
        buffer_capacity: usize,
    ) -> LogSubmitterConfig {
        let dir = std::env::temp_dir().join(format!(
            "aegis-submitter-test-{}-{}",
            std::process::id(),
            ulid::Ulid::generate()
        ));
        let mut cfg = LogSubmitterConfig::new("dev-test".to_string(), sender, dir);
        cfg.batch_size = batch_size;
        cfg.interval = Duration::from_millis(50);
        cfg.buffer_capacity = buffer_capacity;
        cfg.shutdown_deadline = Duration::from_secs(5);
        cfg
    }
}

#[cfg(test)]
mod tests {
    use super::tests_support::{AlreadyIngestedSender, RecordingSender};
    use super::*;
    use crossbeam_channel::Receiver;

    fn temp_pending(max_files: usize) -> (tempfile::TempDir, PendingStore) {
        let tmp = tempfile::tempdir().unwrap();
        let store = PendingStore::new(tmp.path().join("pending"), max_files);
        store.create_dir().unwrap();
        (tmp, store)
    }

    fn ctx(sender: Arc<dyn LogSender>, pending: &PendingStore, batch_size: usize) -> WorkerCtx {
        WorkerCtx {
            device_id: "dev-test".to_string(),
            batch_size,
            interval: Duration::from_millis(50),
            pending: pending.clone_store(),
            sender,
        }
    }

    /// Run the worker to completion. It exits when `rx` is
    /// disconnected, which every test triggers by dropping its `tx`.
    ///
    /// Driven by direct `await` rather than `tokio::spawn` +
    /// `done_rx.recv_timeout`: on a current-thread test runtime the
    /// blocking `recv_timeout` inside `run_worker` would starve the
    /// spawned task and deadlock.
    async fn drive(rx: Receiver<String>, ctx: WorkerCtx) {
        let (done_tx, _done_rx) = crossbeam_channel::bounded(1);
        run_worker(rx, ctx, done_tx).await;
    }

    fn entries(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("line-{i}")).collect()
    }

    // ---- batch id ----

    #[test]
    fn batch_id_is_device_id_plus_ulid() {
        let id = mint_batch_id("dev-abc");
        assert!(id.starts_with("dev-abc-01"), "got {id}");
        let tail = id.rsplit('-').next().unwrap();
        assert_eq!(tail.len(), 26, "ULID tail should be 26 chars, got {tail:?}");
    }

    #[test]
    fn batch_ids_are_unique_within_a_process() {
        let mut ids = std::collections::HashSet::new();
        for _ in 0..10_000 {
            assert!(ids.insert(mint_batch_id("dev-abc")), "duplicate batch id");
        }
        assert_eq!(ids.len(), 10_000);
    }

    #[test]
    fn batch_id_sanitizes_device_id_for_filesystem_use() {
        let id = mint_batch_id("../../etc/passwd");
        assert!(!id.contains('/'), "separator leaked into the id: {id}");
        // Dots fold too — the allowlist is [A-Za-z0-9_-], so a
        // `..` traversal has nothing left to traverse with.
        assert!(!id.contains('.'), "dot leaked into the id: {id}");
        assert!(
            id.contains("etc_passwd"),
            "the id should stay recognisable, got {id}"
        );
    }

    #[test]
    fn batch_id_falls_back_when_device_id_is_empty() {
        let id = mint_batch_id("");
        assert!(id.starts_with("device-"), "got {id}");
    }

    // ---- batching ----

    #[tokio::test]
    async fn batch_size_triggers_a_flush() {
        let sender = RecordingSender::shared();
        let (_tmp, store) = temp_pending(4);
        let c = ctx(sender.clone(), &store, 3);
        let (tx, rx) = crossbeam_channel::bounded(8);
        for e in entries(3) {
            tx.send(e).unwrap();
        }
        drop(tx);
        drive(rx, c).await;
        let accepted = sender.accepted();
        assert_eq!(accepted.len(), 1, "expected one batch, got {accepted:?}");
        assert_eq!(accepted[0].1, entries(3));
    }

    #[tokio::test]
    async fn interval_flushes_a_partial_batch() {
        let sender = RecordingSender::shared();
        let (_tmp, store) = temp_pending(4);
        let c = ctx(sender.clone(), &store, 100);
        let (tx, rx) = crossbeam_channel::bounded(8);
        tx.send("only-one".to_string()).unwrap();
        drop(tx);
        drive(rx, c).await;
        assert_eq!(sender.accepted().len(), 1);
    }

    #[tokio::test]
    async fn interval_with_empty_buffer_sends_nothing() {
        let sender = RecordingSender::shared();
        let (_tmp, store) = temp_pending(4);
        let c = ctx(sender.clone(), &store, 100);
        let (tx, rx) = crossbeam_channel::bounded(8);
        drop(tx);
        drive(rx, c).await;
        assert!(
            sender.accepted().is_empty(),
            "an empty batch would earn a 400 and wedge the drain: {:?}",
            sender.accepted()
        );
    }

    // ---- failure + pending ----

    #[tokio::test]
    async fn failed_fresh_batch_is_persisted_under_its_batch_id() {
        let sender = RecordingSender::shared();
        sender.fail_everything();
        let (_tmp, store) = temp_pending(4);
        let c = ctx(sender.clone(), &store, 2);
        let (tx, rx) = crossbeam_channel::bounded(8);
        for e in entries(2) {
            tx.send(e).unwrap();
        }
        drop(tx);
        drive(rx, c).await;
        let ids = store.list_newest_first();
        assert_eq!(ids.len(), 1, "expected one persisted batch, got {ids:?}");
        assert!(ids[0].starts_with("dev-test-01"), "got {ids:?}");
        assert_eq!(store.load(&ids[0]).unwrap(), entries(2));
    }

    #[tokio::test]
    async fn pending_batches_are_retried_newest_first_and_removed_on_success() {
        let dir = tempfile::tempdir().unwrap();
        let store = PendingStore::new(dir.path().to_path_buf(), 10);
        store.create_dir().unwrap();
        store.persist("dev-A01HQ", &["old".to_string()]).unwrap();
        store.persist("dev-A09HQ", &["new".to_string()]).unwrap();

        let sender = RecordingSender::shared();
        let c = ctx(sender.clone(), &store, 100);
        let (tx, rx) = crossbeam_channel::bounded(8);
        drop(tx);
        drive(rx, c).await;

        let ids: Vec<String> = sender.accepted().into_iter().map(|(id, _)| id).collect();
        assert_eq!(ids, vec!["dev-A09HQ".to_string(), "dev-A01HQ".to_string()]);
        assert!(
            store.list_newest_first().is_empty(),
            "pending dir should drain"
        );
    }

    #[tokio::test]
    async fn drain_stops_at_the_first_failure_and_holds_the_fresh_batch() {
        let dir = tempfile::tempdir().unwrap();
        let store = PendingStore::new(dir.path().to_path_buf(), 10);
        store.create_dir().unwrap();
        store.persist("dev-A01HQ", &["older".to_string()]).unwrap();
        store.persist("dev-A09HQ", &["newer".to_string()]).unwrap();

        let sender = RecordingSender::shared();
        sender.fail_for(&["dev-A09HQ"]);
        let c = ctx(sender.clone(), &store, 1);
        let (tx, rx) = crossbeam_channel::bounded(8);
        tx.send("fresh".to_string()).unwrap();
        drop(tx);
        drive(rx, c).await;

        assert!(
            sender.accepted().is_empty(),
            "nothing should be accepted: {:?}",
            sender.accepted()
        );
        let left = store.list_newest_first();
        assert_eq!(
            left,
            vec!["dev-A09HQ".to_string(), "dev-A01HQ".to_string()],
            "the older pending file must not be touched"
        );
    }

    #[tokio::test]
    async fn pending_drain_continues_past_already_ingested_batch() {
        let dir = tempfile::tempdir().unwrap();
        let store = PendingStore::new(dir.path().to_path_buf(), 10);
        store.create_dir().unwrap();
        store.persist("dev-A01HQ", &["older".to_string()]).unwrap();
        store.persist("dev-A09HQ", &["newer".to_string()]).unwrap();

        let sender = Arc::new(AlreadyIngestedSender::default());
        let c = ctx(sender.clone(), &store, 1);
        let (tx, rx) = crossbeam_channel::bounded(8);
        tx.send("fresh".to_string()).unwrap();
        drop(tx);
        drive(rx, c).await;

        let seen = sender.seen.lock().unwrap().clone();
        let fresh = seen.last().cloned().unwrap();
        assert_eq!(
            seen,
            vec!["dev-A09HQ".to_string(), "dev-A01HQ".to_string(), fresh],
            "drain must continue past a 409, not stop on it"
        );
        assert!(
            store.list_newest_first().is_empty(),
            "an already-ingested batch is done; its file must go"
        );
    }
}
