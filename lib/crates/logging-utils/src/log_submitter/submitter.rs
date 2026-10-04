//! Batch id minting and the worker's flush loop.
//!
//! `run_worker` is a free function taking its dependencies so it can
//! be driven directly from tests without spawning a thread; Task 6
//! wraps it in one.

use std::sync::Arc;
use std::time::Duration;

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender};
use ulid::Ulid;

use super::config::LogSubmitterConfig;
use super::pending::PendingStore;
use super::sender::{LogSender, SubmitterError};

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
    /// Shared with the `SubmitHandle` so trims and channel-full drops
    /// land in one counter an operator can read.
    pub(crate) dropped: Arc<std::sync::atomic::AtomicU64>,
}

/// The server refuses a request carrying more than this many entries
/// (`MAX_ENTRIES_PER_REQUEST` in `aegis-server`), answering
/// `400 validation_failed`. An in-memory batch that grows past it
/// while the drain is blocked can therefore never be submitted, so
/// the worker trims instead. Trimming is counted in
/// `dropped()` — this is the only thing that makes an over-cap batch
/// visible rather than a silent loss.
pub(crate) const MAX_BATCH_ENTRIES: usize = 10_000;

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
                    trim_to_max(&ctx, &mut batch);
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                flush(&ctx, &mut batch).await;
                trim_to_max(&ctx, &mut batch);
            }
            Err(RecvTimeoutError::Disconnected) => {
                flush(&ctx, &mut batch).await;
                // A drain blocked by a transient failure leaves the
                // batch unsent. Persist it rather than dropping it:
                // the next launch retries it, which is strictly
                // better than losing the tail of this session.
                if !batch.is_empty() {
                    let batch_id = mint_batch_id(&ctx.device_id);
                    let entries = std::mem::take(&mut batch);
                    if let Err(err) = ctx.pending.persist(&batch_id, &entries) {
                        tracing::warn!(
                            batch_id = %batch_id,
                            error = %err,
                            "could not persist buffered log entries at shutdown; dropping them"
                        );
                    }
                }
                break;
            }
        }
    }
    let _ = done_tx.send(());
}

/// Keep the in-memory batch at or below the server's per-request cap.
///
/// Only bites while the drain is blocked — when a flush succeeds the
/// batch is emptied anyway. The alternative is growing without bound
/// for the length of an outage and then persisting a file the server
/// can never accept, which would wedge the queue permanently.
fn trim_to_max(ctx: &WorkerCtx, batch: &mut Vec<String>) {
    if batch.len() <= MAX_BATCH_ENTRIES {
        return;
    }
    let overflow = batch.len() - MAX_BATCH_ENTRIES;
    batch.drain(..overflow);
    ctx.dropped
        .fetch_add(overflow as u64, std::sync::atomic::Ordering::Relaxed);
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
                if ctx.sender.is_permanent(&err) {
                    // The sink will refuse this batch on every
                    // retry, so keeping the file would block the
                    // entire queue behind it forever. Delete it and
                    // keep draining.
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
        if ctx.sender.is_permanent(&err) {
            // Permanently refused: a freshly-minted id can only be
            // refused for the batch's *content*, so retrying is
            // pointless. Loud, because the entries are lost.
            tracing::warn!(
                batch_id = %batch_id,
                error = %err,
                "log batch was permanently rejected; dropping it"
            );
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
        permanent: AtomicBool,
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
        /// Fail every batch AND report the failure as permanent —
        /// the shape of a 4xx the server will never accept.
        pub(crate) fn fail_permanently(&self) {
            self.fail_everything();
            self.permanent.store(true, Ordering::Release);
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

        fn is_permanent(&self, _err: &SubmitterError) -> bool {
            self.permanent.load(Ordering::Acquire)
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

        fn is_permanent(&self, _err: &SubmitterError) -> bool {
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
            dropped: Arc::new(std::sync::atomic::AtomicU64::new(0)),
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

    /// Hold the channel open past several intervals, so the worker
    /// actually takes the `RecvTimeoutError::Timeout` branch, then
    /// let it disconnect. Dropping `tx` up front exercises
    /// `Disconnected` instead, which is a different path entirely.
    fn hold_open_for_intervals(
        tx: crossbeam_channel::Sender<String>,
    ) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(200));
            drop(tx);
        })
    }

    #[tokio::test]
    async fn interval_flushes_a_partial_batch() {
        let sender = RecordingSender::shared();
        let (_tmp, store) = temp_pending(4);
        let c = ctx(sender.clone(), &store, 100);
        let (tx, rx) = crossbeam_channel::bounded(8);
        tx.send("only-one".to_string()).unwrap();
        let dropper = hold_open_for_intervals(tx);
        drive(rx, c).await;
        dropper.join().unwrap();
        assert_eq!(sender.accepted().len(), 1);
    }

    #[tokio::test]
    async fn interval_with_empty_buffer_sends_nothing() {
        let sender = RecordingSender::shared();
        let (_tmp, store) = temp_pending(4);
        let c = ctx(sender.clone(), &store, 100);
        let (tx, rx) = crossbeam_channel::bounded(8);
        let dropper = hold_open_for_intervals(tx);
        drive(rx, c).await;
        dropper.join().unwrap();
        assert!(
            sender.accepted().is_empty(),
            "an empty batch would earn a 400 and wedge the drain: {:?}",
            sender.accepted()
        );
    }

    /// The server caps a request at 10 000 entries and answers
    /// anything larger with a permanent 400. A batch that grows past
    /// that while the drain is blocked would therefore become a file
    /// the server can never accept, wedging the queue forever. The
    /// in-memory batch must be trimmed instead, and the loss counted.
    #[tokio::test]
    async fn an_over_cap_batch_is_trimmed_and_counted() {
        let sender = RecordingSender::shared();
        sender.fail_everything(); // transient: the drain stays blocked
        let dir = tempfile::tempdir().unwrap();
        let store = PendingStore::new(dir.path().to_path_buf(), 100);
        store.create_dir().unwrap();
        // A pending file that keeps failing blocks every flush, which
        // is how the batch is left to grow in the first place.
        store
            .persist("dev-A01HQ", &["blocker".to_string()])
            .unwrap();

        let c = ctx(sender.clone(), &store, 1);
        // Unbounded, so every entry reaches the worker. A bounded
        // channel would drop most of them before the worker ever
        // drained them and the batch would never reach the cap.
        let (tx, rx) = crossbeam_channel::unbounded::<String>();
        for i in 0..12_000 {
            tx.send(format!("e-{i}")).unwrap();
        }
        let dropped = Arc::clone(&c.dropped);
        drop(tx);
        drive(rx, c).await;

        assert!(
            dropped.load(std::sync::atomic::Ordering::Relaxed) > 0,
            "a batch past the server's 10k cap must be trimmed, not grown"
        );
    }

    /// A batch the server will never accept must be dropped, not left
    /// at the head of the queue. This is the general form of the
    /// duplicate-batch-id case: a permanent 4xx (over-cap, malformed)
    /// wedges the drain forever just as a 409 does.
    #[tokio::test]
    async fn a_permanent_failure_does_not_wedge_the_drain() {
        let dir = tempfile::tempdir().unwrap();
        let store = PendingStore::new(dir.path().to_path_buf(), 10);
        store.create_dir().unwrap();
        store.persist("dev-A01HQ", &["older".to_string()]).unwrap();
        store
            .persist("dev-A09HQ", &["newer-too-big".to_string()])
            .unwrap();

        let sender = RecordingSender::shared();
        sender.fail_permanently();
        let c = ctx(sender.clone(), &store, 1);
        let (tx, rx) = crossbeam_channel::bounded(8);
        tx.send("fresh".to_string()).unwrap();
        drop(tx);
        drive(rx, c).await;

        // Every pending file was tried — the permanent failure did not
        // stop the walk.
        assert!(
            store.list_newest_first().is_empty(),
            "permanently-rejected batches must be deleted, not retried forever: {:?}",
            store.list_newest_first()
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
    async fn drain_stops_at_the_first_failure_and_persists_the_fresh_batch() {
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
        assert!(
            left.contains(&"dev-A09HQ".to_string()) && left.contains(&"dev-A01HQ".to_string()),
            "the older pending file must not be touched: {left:?}"
        );
        // The fresh batch could not be sent, and shutdown is now its
        // last chance — it is persisted rather than dropped, so the
        // next launch retries it.
        assert_eq!(
            left.len(),
            3,
            "the held fresh batch should be persisted on shutdown, got {left:?}"
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

#[cfg(test)]
mod lifecycle_tests {
    use super::tests_support::{RecordingSender, make_config};
    use super::*;

    fn wait_for(sender: &RecordingSender, n: usize) {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while sender.accepted().len() < n {
            assert!(
                std::time::Instant::now() < deadline,
                "timed out waiting for {n} batches; got {:?}",
                sender.accepted()
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// A sender that stalls on its *first* call only, so the worker
    /// parks long enough for the test to fill the channel, then
    /// drains promptly so shutdown is not serialised behind 64 slow
    /// sends.
    #[derive(Debug, Default)]
    struct SlowSender {
        stalled: std::sync::atomic::AtomicBool,
    }

    #[async_trait::async_trait]
    impl LogSender for SlowSender {
        async fn send(
            &self,
            _batch_id: &str,
            _log_entries: Vec<String>,
        ) -> Result<(), SubmitterError> {
            if !self.stalled.swap(true, std::sync::atomic::Ordering::AcqRel) {
                std::thread::sleep(Duration::from_millis(1_500));
            }
            Ok(())
        }
    }

    #[test]
    fn submit_delivers_an_entry_to_the_sender() {
        let sender = RecordingSender::shared();
        let mut sub = LogSubmitter::new(make_config(sender.clone(), 2, 16)).unwrap();
        sub.submit("one".into()).unwrap();
        sub.submit("two".into()).unwrap();
        wait_for(&sender, 1);
        let accepted = sender.accepted();
        assert_eq!(accepted[0].1, vec!["one".to_string(), "two".to_string()]);
        sub.shutdown().unwrap();
    }

    #[test]
    fn dropped_counts_entries_lost_to_a_full_buffer() {
        // Backpressure only bites while the worker is parked in an
        // in-flight `send`; otherwise it drains the channel into its
        // own in-memory batch and `try_send` never sees it full.
        let sender = Arc::new(SlowSender::default());
        let mut sub = LogSubmitter::new(make_config(sender, 1, 64)).unwrap();
        sub.submit("occupy-the-worker".into()).unwrap();
        // Give the worker time to reach the slow send.
        std::thread::sleep(Duration::from_millis(100));
        for i in 0..5_000 {
            let _ = sub.submit(format!("flood-{i}"));
        }
        assert!(
            sub.dropped() > 0,
            "a full buffer should be counted, not silently swallowed"
        );
        sub.shutdown().unwrap();
    }

    #[test]
    fn shutdown_is_idempotent() {
        let sender = RecordingSender::shared();
        let mut sub = LogSubmitter::new(make_config(sender, 100, 16)).unwrap();
        sub.shutdown().unwrap();
        sub.shutdown().unwrap();
    }

    #[test]
    fn submit_after_shutdown_reports_channel_closed() {
        let sender = RecordingSender::shared();
        let mut sub = LogSubmitter::new(make_config(sender, 100, 16)).unwrap();
        sub.shutdown().unwrap();
        let err = sub.submit("late".into()).unwrap_err();
        assert!(matches!(err, SubmitterError::ChannelClosed), "got {err:?}");
    }

    #[test]
    fn handle_shares_the_dropped_counter_and_survives_shutdown() {
        let sender = RecordingSender::shared();
        let mut sub = LogSubmitter::new(make_config(sender, 100, 16)).unwrap();
        let handle = sub.handle();
        assert_eq!(handle.dropped(), sub.dropped());
        handle.submit("via-handle".into()).unwrap();
        sub.shutdown().unwrap();
        // The layer's handle outlives shutdown; it must report the
        // worker as gone rather than silently dropping.
        assert!(matches!(
            handle.submit("after".into()),
            Err(SubmitterError::ChannelClosed)
        ));
    }
}

/// The producer side, shared with the `tracing` layer.
///
/// Cheap to clone and holds no thread handle, so the layer can keep
/// one for the process lifetime without pinning the worker alive.
#[derive(Clone)]
pub struct SubmitHandle {
    inner: Arc<Inner>,
}

struct Inner {
    /// `None` once the worker is shut down. The sender lives behind
    /// an `Option` rather than being dropped directly because the
    /// `tracing` layer holds a clone of this handle: dropping only
    /// the `LogSubmitter`'s copy would leave the channel open and
    /// the worker blocked in `recv_timeout` until its full interval
    /// elapsed, turning every shutdown into a timeout.
    tx: std::sync::Mutex<Option<crossbeam_channel::Sender<String>>>,
    /// Shared with the worker, which trims over-cap batches and
    /// counts the same losses a full buffer produces.
    dropped: Arc<std::sync::atomic::AtomicU64>,
}

impl std::fmt::Debug for SubmitHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubmitHandle")
            .field("dropped", &self.dropped())
            .finish()
    }
}

impl SubmitHandle {
    /// Hand one formatted entry to the worker.
    ///
    /// A full buffer is **not** an error: this is called from a
    /// `tracing` layer's write path, where the only honest options
    /// are to drop the line or to block the thread that emitted it.
    /// The drop is counted instead, so the loss is observable via
    /// [`SubmitHandle::dropped`].
    pub fn submit(&self, entry: String) -> Result<(), SubmitterError> {
        let guard = self
            .inner
            .tx
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(tx) = guard.as_ref() else {
            return Err(SubmitterError::ChannelClosed);
        };
        let outcome = tx.try_send(entry);
        drop(guard);
        match outcome {
            Ok(()) => Ok(()),
            Err(crossbeam_channel::TrySendError::Full(_)) => {
                self.inner
                    .dropped
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                Ok(())
            }
            Err(crossbeam_channel::TrySendError::Disconnected(_)) => {
                Err(SubmitterError::ChannelClosed)
            }
        }
    }

    /// Entries lost to a full buffer since startup.
    pub fn dropped(&self) -> u64 {
        self.inner
            .dropped
            .load(std::sync::atomic::Ordering::Relaxed)
    }
}

/// The batching worker. Owns the thread; manage it for the process
/// lifetime or it stops submitting.
#[derive(Debug)]
pub struct LogSubmitter {
    done_rx: Option<crossbeam_channel::Receiver<()>>,
    worker: Option<std::thread::JoinHandle<()>>,
    handle: SubmitHandle,
    deadline: Duration,
}

impl LogSubmitter {
    /// Create the pending directory and spawn the worker thread.
    pub fn new(config: LogSubmitterConfig) -> Result<Self, SubmitterError> {
        let pending = PendingStore::new(config.pending_dir.clone(), config.max_pending_files);
        pending.create_dir()?;

        let (tx, rx) = crossbeam_channel::bounded::<String>(config.buffer_capacity);
        let (done_tx, done_rx) = crossbeam_channel::bounded::<()>(1);
        // One counter for both kinds of loss: entries trimmed off an
        // over-cap batch in the worker, and entries the layer could
        // not hand over because the buffer was full.
        let dropped = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let ctx = WorkerCtx {
            device_id: config.device_id.clone(),
            batch_size: config.batch_size,
            interval: config.interval,
            pending,
            sender: config.sender,
            dropped: Arc::clone(&dropped),
        };

        // A current-thread runtime with `enable_all` cannot fail in
        // practice; `HttpClient::new` makes the same bet for its
        // client. `expect` keeps a second error variant off the
        // public surface for a case that does not occur.
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("current-thread runtime builds");

        let worker = std::thread::spawn(move || {
            runtime.block_on(run_worker(rx, ctx, done_tx));
        });

        let handle = SubmitHandle {
            inner: Arc::new(Inner {
                tx: std::sync::Mutex::new(Some(tx)),
                dropped,
            }),
        };

        Ok(Self {
            done_rx: Some(done_rx),
            worker: Some(worker),
            handle,
            deadline: config.shutdown_deadline,
        })
    }

    /// A cloneable producer handle for the `tracing` layer.
    pub fn handle(&self) -> SubmitHandle {
        self.handle.clone()
    }

    /// See [`SubmitHandle::submit`].
    pub fn submit(&self, entry: String) -> Result<(), SubmitterError> {
        self.handle.submit(entry)
    }

    /// See [`SubmitHandle::dropped`].
    pub fn dropped(&self) -> u64 {
        self.handle.dropped()
    }

    /// Drain the buffer and join the worker within
    /// `shutdown_deadline`. Idempotent; also runs from `Drop`.
    pub fn shutdown(&mut self) -> Result<(), SubmitterError> {
        if self.worker.is_none() {
            return Ok(());
        }
        // Taking the sender — which the `SubmitHandle` shares with
        // the layer — is what closes the channel. The worker then
        // sees `Disconnected`, flushes what it has, and signals done.
        {
            let mut guard = self
                .handle
                .inner
                .tx
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            guard.take();
        }
        let done_rx = self.done_rx.take().expect("done_rx present iff worker is");
        let worker = self
            .worker
            .take()
            .expect("worker present iff not yet shut down");

        match done_rx.recv_timeout(self.deadline) {
            Ok(()) => match worker.join() {
                Ok(()) => Ok(()),
                Err(payload) => Err(SubmitterError::WorkerPanic(panic_message(&payload))),
            },
            Err(_) => {
                if worker.is_finished() {
                    match worker.join() {
                        Ok(()) => Err(SubmitterError::WorkerJoinTimeout {
                            deadline: self.deadline,
                            message: "worker exited without signalling done".into(),
                        }),
                        Err(payload) => Err(SubmitterError::WorkerPanic(panic_message(&payload))),
                    }
                } else {
                    Err(SubmitterError::WorkerJoinTimeout {
                        deadline: self.deadline,
                        message: "worker thread did not exit in time".into(),
                    })
                }
            }
        }
    }
}

impl Drop for LogSubmitter {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

/// Best-effort conversion of a panic payload to a UTF-8 string;
/// mirrors the helper in `log_ingestor`.
fn panic_message(payload: &Box<dyn std::any::Any + Send + 'static>) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        format!("{payload:?}")
    }
}
