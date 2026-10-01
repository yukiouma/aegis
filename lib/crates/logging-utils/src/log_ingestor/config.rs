//! Knobs for [`crate::LogIngestor`]. See the spec for the design
//! rationale; defaults are pinned in [`LogIngestorConfig::new`].

use std::path::PathBuf;
use std::time::Duration;

/// Default moka cache capacity (10 000 entries).
const DEFAULT_CACHE_CAPACITY: u64 = 10_000;
/// Default moka cache TTL (600 s).
const DEFAULT_CACHE_TTL: Duration = Duration::from_secs(600);
/// Default bounded-channel capacity (1 000 envelopes).
const DEFAULT_CHANNEL_CAPACITY: usize = 1_000;
/// Default shutdown / drop deadline (5 s).
const DEFAULT_SHUTDOWN_DEADLINE: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
pub struct LogIngestorConfig {
    pub log_dir: PathBuf,
    pub file_name_prefix: String,
    pub cache_capacity: u64,
    pub cache_ttl: Duration,
    pub channel_capacity: usize,
    pub shutdown_deadline: Duration,
}

impl LogIngestorConfig {
    pub fn new(log_dir: PathBuf, file_name_prefix: String) -> Self {
        Self {
            log_dir,
            file_name_prefix,
            cache_capacity: DEFAULT_CACHE_CAPACITY,
            cache_ttl: DEFAULT_CACHE_TTL,
            channel_capacity: DEFAULT_CHANNEL_CAPACITY,
            shutdown_deadline: DEFAULT_SHUTDOWN_DEADLINE,
        }
    }

    pub fn builder() -> LogIngestorConfigBuilder {
        LogIngestorConfigBuilder::default()
    }
}

#[derive(Debug, Clone)]
pub struct LogIngestorConfigBuilder {
    log_dir: Option<PathBuf>,
    file_name_prefix: Option<String>,
    cache_capacity: Option<u64>,
    cache_ttl: Option<Duration>,
    channel_capacity: Option<usize>,
    shutdown_deadline: Option<Duration>,
}

impl Default for LogIngestorConfigBuilder {
    fn default() -> Self {
        Self {
            log_dir: None,
            file_name_prefix: None,
            cache_capacity: None,
            cache_ttl: None,
            channel_capacity: None,
            shutdown_deadline: None,
        }
    }
}

impl LogIngestorConfigBuilder {
    pub fn log_dir(mut self, dir: PathBuf) -> Self {
        self.log_dir = Some(dir);
        self
    }
    pub fn file_name_prefix(mut self, prefix: String) -> Self {
        self.file_name_prefix = Some(prefix);
        self
    }
    pub fn cache_capacity(mut self, capacity: u64) -> Self {
        self.cache_capacity = Some(capacity);
        self
    }
    pub fn cache_ttl(mut self, ttl: Duration) -> Self {
        self.cache_ttl = Some(ttl);
        self
    }
    pub fn channel_capacity(mut self, capacity: usize) -> Self {
        self.channel_capacity = Some(capacity);
        self
    }
    pub fn shutdown_deadline(mut self, deadline: Duration) -> Self {
        self.shutdown_deadline = Some(deadline);
        self
    }
    pub fn build(self) -> LogIngestorConfig {
        let defaults = LogIngestorConfig::new(PathBuf::new(), String::new());
        LogIngestorConfig {
            log_dir: self.log_dir.unwrap_or(defaults.log_dir),
            file_name_prefix: self.file_name_prefix.unwrap_or(defaults.file_name_prefix),
            cache_capacity: self.cache_capacity.unwrap_or(defaults.cache_capacity),
            cache_ttl: self.cache_ttl.unwrap_or(defaults.cache_ttl),
            channel_capacity: self.channel_capacity.unwrap_or(defaults.channel_capacity),
            shutdown_deadline: self.shutdown_deadline.unwrap_or(defaults.shutdown_deadline),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_apply_when_only_dir_and_prefix_set() {
        let cfg = LogIngestorConfig::new(
            PathBuf::from("/tmp/aegis"),
            "test.log".to_string(),
        );
        assert_eq!(cfg.log_dir, PathBuf::from("/tmp/aegis"));
        assert_eq!(cfg.file_name_prefix, "test.log");
        assert_eq!(cfg.cache_capacity, DEFAULT_CACHE_CAPACITY);
        assert_eq!(cfg.cache_ttl, DEFAULT_CACHE_TTL);
        assert_eq!(cfg.channel_capacity, DEFAULT_CHANNEL_CAPACITY);
        assert_eq!(cfg.shutdown_deadline, DEFAULT_SHUTDOWN_DEADLINE);
    }

    #[test]
    fn builder_overrides_each_knob_independently() {
        let cfg = LogIngestorConfig::builder()
            .log_dir(PathBuf::from("/tmp/aegis-builder"))
            .file_name_prefix("builder.log".to_string())
            .cache_capacity(123)
            .cache_ttl(Duration::from_secs(7))
            .channel_capacity(45)
            .shutdown_deadline(Duration::from_secs(2))
            .build();
        assert_eq!(cfg.log_dir, PathBuf::from("/tmp/aegis-builder"));
        assert_eq!(cfg.file_name_prefix, "builder.log");
        assert_eq!(cfg.cache_capacity, 123);
        assert_eq!(cfg.cache_ttl, Duration::from_secs(7));
        assert_eq!(cfg.channel_capacity, 45);
        assert_eq!(cfg.shutdown_deadline, Duration::from_secs(2));
    }

    #[test]
    fn clone_preserves_fields() {
        let original = LogIngestorConfig::new(
            PathBuf::from("/tmp/aegis-clone"),
            "clone.log".to_string(),
        );
        let cloned = original.clone();
        assert_eq!(original.log_dir, cloned.log_dir);
        assert_eq!(original.file_name_prefix, cloned.file_name_prefix);
        assert_eq!(original.cache_capacity, cloned.cache_capacity);
        assert_eq!(original.cache_ttl, cloned.cache_ttl);
        assert_eq!(original.channel_capacity, cloned.channel_capacity);
        assert_eq!(original.shutdown_deadline, cloned.shutdown_deadline);
    }
}
