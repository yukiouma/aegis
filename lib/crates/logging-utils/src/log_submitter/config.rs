//! Knobs for [`crate::LogSubmitter`]. Defaults are pinned in
//! [`LogSubmitterConfig::new`] and match the values named in the
//! design spec.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use super::sender::{LogSender, SubmitterError};

/// Entries per batch before a size-triggered flush.
const DEFAULT_BATCH_SIZE: usize = 200;
/// Idle window before a timer-triggered flush.
const DEFAULT_INTERVAL: Duration = Duration::from_secs(60);
/// Bounded channel capacity between the submit layer and the worker.
const DEFAULT_BUFFER_CAPACITY: usize = 1_000;
/// Ceiling on persisted failed batches; the oldest is evicted past it.
const DEFAULT_MAX_PENDING_FILES: usize = 10;
/// Bound on the shutdown join.
const DEFAULT_SHUTDOWN_DEADLINE: Duration = Duration::from_secs(5);

/// Placeholder used only to source scalar defaults in
/// `LogSubmitterConfigBuilder::build`. Never reaches a worker: the
/// builder's `build` overwrites it with the caller's sender.
#[derive(Debug)]
struct NoopSender;

#[async_trait::async_trait]
impl LogSender for NoopSender {
    async fn send(&self, _batch_id: &str, _log_entries: Vec<String>) -> Result<(), SubmitterError> {
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct LogSubmitterConfig {
    /// Per-install identifier, and the first segment of every batch id.
    pub device_id: String,
    /// The transport the worker submits batches through.
    pub sender: Arc<dyn LogSender>,
    /// Flush once this many entries are buffered.
    pub batch_size: usize,
    /// Flush after this much inactivity on the buffer.
    pub interval: Duration,
    /// Bounded channel capacity between the layer and the worker.
    pub buffer_capacity: usize,
    /// Where failed batches are persisted, and where they are
    /// retried from.
    pub pending_dir: PathBuf,
    /// Ceiling on pending files; the oldest is evicted past it.
    pub max_pending_files: usize,
    /// Bound on the shutdown join.
    pub shutdown_deadline: Duration,
}

impl LogSubmitterConfig {
    /// Build a config with every optional knob at its default.
    pub fn new(device_id: String, sender: Arc<dyn LogSender>, pending_dir: PathBuf) -> Self {
        Self {
            device_id,
            sender,
            batch_size: DEFAULT_BATCH_SIZE,
            interval: DEFAULT_INTERVAL,
            buffer_capacity: DEFAULT_BUFFER_CAPACITY,
            pending_dir,
            max_pending_files: DEFAULT_MAX_PENDING_FILES,
            shutdown_deadline: DEFAULT_SHUTDOWN_DEADLINE,
        }
    }

    pub fn builder() -> LogSubmitterConfigBuilder {
        LogSubmitterConfigBuilder::default()
    }
}

#[derive(Debug, Clone, Default)]
pub struct LogSubmitterConfigBuilder {
    device_id: Option<String>,
    sender: Option<Arc<dyn LogSender>>,
    batch_size: Option<usize>,
    interval: Option<Duration>,
    buffer_capacity: Option<usize>,
    pending_dir: Option<PathBuf>,
    max_pending_files: Option<usize>,
    shutdown_deadline: Option<Duration>,
}

impl LogSubmitterConfigBuilder {
    pub fn device_id(mut self, v: String) -> Self {
        self.device_id = Some(v);
        self
    }
    pub fn sender(mut self, v: Arc<dyn LogSender>) -> Self {
        self.sender = Some(v);
        self
    }
    pub fn batch_size(mut self, v: usize) -> Self {
        self.batch_size = Some(v);
        self
    }
    pub fn interval(mut self, v: Duration) -> Self {
        self.interval = Some(v);
        self
    }
    pub fn buffer_capacity(mut self, v: usize) -> Self {
        self.buffer_capacity = Some(v);
        self
    }
    pub fn pending_dir(mut self, v: PathBuf) -> Self {
        self.pending_dir = Some(v);
        self
    }
    pub fn max_pending_files(mut self, v: usize) -> Self {
        self.max_pending_files = Some(v);
        self
    }
    pub fn shutdown_deadline(mut self, v: Duration) -> Self {
        self.shutdown_deadline = Some(v);
        self
    }

    pub fn build(self) -> LogSubmitterConfig {
        let d = LogSubmitterConfig::new(
            String::new(),
            Arc::new(NoopSender) as Arc<dyn LogSender>,
            PathBuf::new(),
        );
        LogSubmitterConfig {
            device_id: self.device_id.unwrap_or(d.device_id),
            sender: self.sender.unwrap_or(d.sender),
            batch_size: self.batch_size.unwrap_or(d.batch_size),
            interval: self.interval.unwrap_or(d.interval),
            buffer_capacity: self.buffer_capacity.unwrap_or(d.buffer_capacity),
            pending_dir: self.pending_dir.unwrap_or(d.pending_dir),
            max_pending_files: self.max_pending_files.unwrap_or(d.max_pending_files),
            shutdown_deadline: self.shutdown_deadline.unwrap_or(d.shutdown_deadline),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[derive(Debug, Default)]
    struct NullSender;

    #[async_trait::async_trait]
    impl LogSender for NullSender {
        async fn send(
            &self,
            _batch_id: &str,
            _log_entries: Vec<String>,
        ) -> Result<(), SubmitterError> {
            Ok(())
        }
    }

    fn sender() -> Arc<dyn LogSender> {
        Arc::new(NullSender)
    }

    #[test]
    fn new_applies_documented_defaults() {
        let cfg = LogSubmitterConfig::new(
            "dev-1".to_string(),
            sender(),
            PathBuf::from("/tmp/aegis-pending"),
        );
        assert_eq!(cfg.device_id, "dev-1");
        assert_eq!(cfg.pending_dir, PathBuf::from("/tmp/aegis-pending"));
        assert_eq!(cfg.batch_size, 200);
        assert_eq!(cfg.interval, Duration::from_secs(60));
        assert_eq!(cfg.buffer_capacity, 1_000);
        assert_eq!(cfg.max_pending_files, 10);
        assert_eq!(cfg.shutdown_deadline, Duration::from_secs(5));
    }

    #[test]
    fn builder_overrides_each_knob_independently() {
        let cfg = LogSubmitterConfig::builder()
            .device_id("dev-2".to_string())
            .sender(sender())
            .pending_dir(PathBuf::from("/tmp/p2"))
            .batch_size(7)
            .interval(Duration::from_millis(250))
            .buffer_capacity(9)
            .max_pending_files(3)
            .shutdown_deadline(Duration::from_secs(2))
            .build();
        assert_eq!(cfg.device_id, "dev-2");
        assert_eq!(cfg.pending_dir, PathBuf::from("/tmp/p2"));
        assert_eq!(cfg.batch_size, 7);
        assert_eq!(cfg.interval, Duration::from_millis(250));
        assert_eq!(cfg.buffer_capacity, 9);
        assert_eq!(cfg.max_pending_files, 3);
        assert_eq!(cfg.shutdown_deadline, Duration::from_secs(2));
    }

    #[test]
    fn builder_fills_unset_knobs_with_defaults() {
        let cfg = LogSubmitterConfig::builder()
            .device_id("dev-3".to_string())
            .sender(sender())
            .pending_dir(PathBuf::from("/tmp/p3"))
            .build();
        assert_eq!(cfg.batch_size, 200);
        assert_eq!(cfg.buffer_capacity, 1_000);
    }

    #[test]
    fn clone_preserves_scalar_fields() {
        let original =
            LogSubmitterConfig::new("dev-4".to_string(), sender(), PathBuf::from("/tmp/p4"));
        let cloned = original.clone();
        assert_eq!(original.device_id, cloned.device_id);
        assert_eq!(original.batch_size, cloned.batch_size);
        assert_eq!(original.pending_dir, cloned.pending_dir);
        assert_eq!(original.max_pending_files, cloned.max_pending_files);
    }
}
