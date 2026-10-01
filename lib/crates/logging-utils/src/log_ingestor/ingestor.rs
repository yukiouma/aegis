//! [`LogIngestor`] — sync, deduplicated, bounded-queue log submitter
//! + the background writer thread lifecycle. See the spec for
//! rationale; see [`crate::LogIngestorConfig`] for knobs.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use crossbeam_channel::{bounded, Receiver, Sender};
use moka::sync::Cache;
use thiserror::Error;

use super::config::LogIngestorConfig;
use super::writer::{run_writer, BatchEnvelope};

#[derive(Debug, Error)]
pub enum IngestorError {
    #[error("failed to create log directory {dir}: {source}")]
    CreateDir {
        dir: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("batch id {0} was already submitted within the cache TTL")]
    DuplicateBatchId(String),
    #[error("ingestor channel is full (capacity {0}); caller should retry")]
    ChannelFull(usize),
    #[error("ingestor has been shut down")]
    ShutDown,
    #[error("writer thread failed to join within {deadline:?}: {message}")]
    WriterJoinTimeout {
        deadline: Duration,
        message: String,
    },
    #[error("writer thread panicked: {0}")]
    WriterPanic(String),
}

#[derive(Debug)]
pub struct LogIngestor {
    cache: Cache<String, ()>,
    tx: Option<Sender<BatchEnvelope>>,
    done_rx: Option<Receiver<()>>,
    writer: Option<JoinHandle<()>>,
    shutdown_flag: AtomicBool,
    deadline: Duration,
    channel_capacity: usize,
}

impl LogIngestor {
    /// Build a [`LogIngestor`] with the configured knobs. Creates
    /// the log directory (failure → [`IngestorError::CreateDir`])
    /// and spawns the writer thread.
    pub fn new(config: LogIngestorConfig) -> Result<Self, IngestorError> {
        std::fs::create_dir_all(&config.log_dir).map_err(|source| {
            IngestorError::CreateDir {
                dir: config.log_dir.clone(),
                source,
            }
        })?;
        let appender = tracing_appender::rolling::daily(
            &config.log_dir,
            &config.file_name_prefix,
        );
        let cache = Cache::builder()
            .max_capacity(config.cache_capacity)
            .time_to_live(config.cache_ttl)
            .build();
        let (tx, rx) = bounded::<BatchEnvelope>(config.channel_capacity);
        let (done_tx, done_rx) = bounded::<()>(1);
        let writer = std::thread::spawn(move || run_writer(rx, appender, done_tx));

        Ok(Self {
            cache,
            tx: Some(tx),
            done_rx: Some(done_rx),
            writer: Some(writer),
            shutdown_flag: AtomicBool::new(false),
            deadline: config.shutdown_deadline,
            channel_capacity: config.channel_capacity,
        })
    }

    /// Submit a batch of UTF-8 log lines identified by `batch_id`.
    /// See module-level docs and the spec for the full contract.
    pub fn submit(
        &self,
        batch_id: impl Into<String>,
        entries: &[String],
    ) -> Result<(), IngestorError> {
        if self.shutdown_flag.load(Ordering::Acquire) {
            return Err(IngestorError::ShutDown);
        }
        let tx = self.tx.as_ref().ok_or(IngestorError::ShutDown)?;
        let id = batch_id.into();
        if self.cache.get(&id).is_some() {
            return Err(IngestorError::DuplicateBatchId(id));
        }
        self.cache.insert(id.clone(), ());
        let envelope = BatchEnvelope {
            batch_id: id,
            entries: entries.to_vec(),
        };
        match tx.send(envelope) {
            Ok(()) => Ok(()),
            Err(crossbeam_channel::SendError(env)) => {
                // Channel full: undo the cache insert so the same id
                // can be retried later. `SendError` carries the
                // un-sent envelope back.
                self.cache.invalidate(&env.batch_id);
                Err(IngestorError::ChannelFull(self.channel_capacity))
            }
        }
    }

    /// Drain the channel and join the writer thread within
    /// `config.shutdown_deadline`. Idempotent.
    ///
    /// **Stub:** Task 4 ships the minimal drain-and-join version so
    /// the submit-path tests can run. Task 6 replaces this with
    /// the deadline-aware implementation that distinguishes
    /// `WriterJoinTimeout` from `WriterPanic`.
    pub fn shutdown(&mut self) -> Result<(), IngestorError> {
        if self.writer.is_none() {
            return Ok(()); // already shut down
        }
        self.shutdown_flag.store(true, Ordering::Release);
        self.tx.take();
        self.done_rx.take();
        let writer = self.writer.take()
            .expect("writer present iff not yet shut down");
        match writer.join() {
            Ok(()) => Ok(()),
            Err(payload) => Err(IngestorError::WriterPanic(format!("{payload:?}"))),
        }
    }
}

impl Drop for LogIngestor {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::sync::Arc;
    use std::thread;

    fn read_log(dir: &Path, prefix: &str) -> (String, String) {
        let entry = std::fs::read_dir(dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .find(|e| e.file_name().to_string_lossy().starts_with(prefix))
            .expect("expected at least one daily-rotate file under prefix");
        let name = entry.file_name().to_string_lossy().into_owned();
        let contents = std::fs::read_to_string(entry.path()).unwrap();
        (name, contents)
    }

    fn cfg(dir: &Path) -> LogIngestorConfig {
        LogIngestorConfig::builder()
            .log_dir(dir.to_path_buf())
            .file_name_prefix("ingestor-test.log".to_string())
            .shutdown_deadline(Duration::from_secs(2))
            .build()
    }

    #[test]
    fn new_creates_log_dir_when_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let nested = tmp.path().join("nested/dir");
        assert!(!nested.exists());
        let ingestor = LogIngestor::new(cfg(&nested)).expect("new succeeds");
        assert!(nested.is_dir());
        drop(ingestor);
    }

    #[test]
    fn new_propagates_create_dir_failure() {
        let tmp = tempfile::tempdir().unwrap();
        let blocker = tmp.path().join("blocker");
        std::fs::write(&blocker, b"not a dir").unwrap();
        let bad = blocker.join("inside");
        let err = LogIngestor::new(cfg(&bad)).unwrap_err();
        match err {
            IngestorError::CreateDir { dir, .. } => assert_eq!(dir, bad),
            other => panic!("expected CreateDir, got {other:?}"),
        }
    }

    #[test]
    fn submit_writes_one_line_per_entry_under_configured_prefix() {
        let tmp = tempfile::tempdir().unwrap();
        let mut ingestor = LogIngestor::new(cfg(tmp.path())).expect("new succeeds");

        ingestor
            .submit(
                "b-1",
                &["alpha".to_string(), "beta".to_string(), "gamma".to_string()],
            )
            .expect("submit ok");

        ingestor.shutdown().expect("shutdown ok");

        let (_name, contents) = read_log(tmp.path(), "ingestor-test.log");
        assert_eq!(contents, "BATCH b-1\nalpha\nbeta\ngamma\n");
    }

    #[test]
    fn submit_two_batches_with_distinct_ids_both_appear_in_file() {
        let tmp = tempfile::tempdir().unwrap();
        let mut ingestor = LogIngestor::new(cfg(tmp.path())).expect("new succeeds");

        ingestor.submit("a", &["1".into()]).unwrap();
        ingestor.submit("b", &["2".into()]).unwrap();

        ingestor.shutdown().expect("shutdown ok");

        let (_name, contents) = read_log(tmp.path(), "ingestor-test.log");
        assert_eq!(contents, "BATCH a\n1\nBATCH b\n2\n");
    }

    #[test]
    fn submit_rejects_duplicate_batch_id_within_ttl() {
        let tmp = tempfile::tempdir().unwrap();
        let mut ingestor = LogIngestor::new(
            LogIngestorConfig::builder()
                .log_dir(tmp.path().to_path_buf())
                .file_name_prefix("dup.log".to_string())
                .cache_ttl(Duration::from_secs(60))
                .shutdown_deadline(Duration::from_secs(2))
                .build(),
        )
        .expect("new succeeds");

        ingestor.submit("dup", &["first".into()]).unwrap();
        let err = ingestor.submit("dup", &["second".into()]).unwrap_err();
        assert!(matches!(err, IngestorError::DuplicateBatchId(ref id) if id == "dup"));

        ingestor.shutdown().expect("shutdown ok");
    }

    #[test]
    fn submit_duplicate_after_ttl_expires_succeeds() {
        let tmp = tempfile::tempdir().unwrap();
        let mut ingestor = LogIngestor::new(
            LogIngestorConfig::builder()
                .log_dir(tmp.path().to_path_buf())
                .file_name_prefix("ttl.log".to_string())
                .cache_ttl(Duration::from_millis(50))
                .shutdown_deadline(Duration::from_secs(2))
                .build(),
        )
        .expect("new succeeds");

        ingestor.submit("ttl", &["first".into()]).unwrap();
        thread::sleep(Duration::from_millis(100));
        ingestor.submit("ttl", &["second".into()])
            .expect("submit after TTL expiry succeeds");

        ingestor.shutdown().expect("shutdown ok");
    }

    #[test]
    fn submit_concurrent_producers_no_deadlock_or_corruption() {
        // Review Focus #2: 8 threads × 25 distinct ids each = 200
        // batches submitted concurrently. All must succeed; all
        // 200 must appear in the file (moka is thread-safe, the
        // bounded channel is thread-safe).
        let tmp = tempfile::tempdir().unwrap();
        let mut ingestor = LogIngestor::new(cfg(tmp.path())).expect("new succeeds");
        let ingestor = Arc::new(ingestor);

        let mut handles = Vec::new();
        for t in 0..8 {
            let ingestor = Arc::clone(&ingestor);
            handles.push(thread::spawn(move || {
                for i in 0..25 {
                    let id = format!("t{t}-i{i}");
                    ingestor.submit(&id, &[format!("payload {id}")]).unwrap();
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }

        // Move out of Arc and shut down.
        let mut ingestor = Arc::try_unwrap(ingestor).expect("all senders joined");
        ingestor.shutdown().expect("shutdown ok");

        let (_name, contents) = read_log(tmp.path(), "ingestor-test.log");
        let batches: Vec<&str> = contents
            .lines()
            .filter(|l| l.starts_with("BATCH "))
            .collect();
        assert_eq!(batches.len(), 200, "all 200 batch headers must appear");

        // Spot-check that every distinct id appears exactly once.
        // (Use exact line equality, not substring `matches`, so
        // `t0-i1` is not counted as a match inside `t0-i10`.)
        for t in 0..8 {
            for i in 0..25 {
                let header = format!("BATCH t{t}-i{i}");
                let count = contents
                    .lines()
                    .filter(|&l| l == header)
                    .count();
                assert_eq!(count, 1, "id {header} must appear exactly once");
            }
        }
    }
}
