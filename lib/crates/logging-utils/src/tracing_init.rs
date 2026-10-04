//! Tracing bootstrap. See module-level docs at the crate root.

use std::path::PathBuf;
use thiserror::Error;
use tracing::Subscriber;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer, Registry};

#[derive(Debug, Error)]
pub enum LoggingInitError {
    #[error("failed to create log directory {dir}: {source}")]
    CreateDir {
        dir: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Debug)]
pub struct LogGuard(pub WorkerGuard);

#[derive(Debug, Clone)]
pub struct LoggingConfig {
    pub log_dir: PathBuf,
    pub file_name_prefix: String,
}

/// Build the global `EnvFilter` from `AEGIS_LOG_LEVEL`. Defaults to
/// `info` when the variable is unset.
pub fn build_filter() -> EnvFilter {
    let level = std::env::var("AEGIS_LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
    EnvFilter::new(level)
}

/// Build the subscriber without installing it.
///
/// Split out from [`init_tracing`] so the composition can be tested
/// through `tracing::subscriber::with_default`. `init_tracing`
/// installs globally and is `try_init`-idempotent, so a test calling
/// it would silently get whichever subscriber the first test in the
/// process installed — the extra layers it passes would never be
/// observed.
fn build_subscriber(
    config: &LoggingConfig,
    layers: Vec<Box<dyn Layer<Registry> + Send + Sync>>,
) -> Result<(impl Subscriber + Send + Sync, LogGuard), LoggingInitError> {
    std::fs::create_dir_all(&config.log_dir).map_err(|source| LoggingInitError::CreateDir {
        dir: config.log_dir.clone(),
        source,
    })?;
    let file_appender = tracing_appender::rolling::daily(&config.log_dir, &config.file_name_prefix);
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    let fmt_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_current_span(true)
        .with_span_list(false)
        .with_writer(non_blocking);
    // Layer order matters twice over here.
    //
    // The boxed layers are `Layer<Registry>`, so they have to attach
    // to the bare registry — they cannot sit outside an
    // already-`Layered` subscriber. And the filter is added LAST,
    // making it the outermost layer, where its `enabled`
    // short-circuits before any inner layer sees the event. Left on
    // `fmt_layer` it would gate only the file, and the desktop's
    // submit layer would ship DEBUG events while the process runs
    // at `info`.
    let subscriber = tracing_subscriber::registry()
        .with(layers)
        .with(fmt_layer)
        .with(build_filter());
    Ok((subscriber, LogGuard(guard)))
}

/// Install the global JSON `tracing` subscriber writing to
/// `{log_dir}/{file_name_prefix}.YYYY-MM-DD` (one file per day,
/// rotation at local midnight), plus any extra `layers` the caller
/// supplies. The `EnvFilter` is built from `AEGIS_LOG_LEVEL`
/// (default `info`) and gates every layer, not just the file.
///
/// The returned [`LogGuard`] MUST be held for the lifetime of the
/// program; dropping it flushes the buffered writer. `try_init`
/// semantics make a re-entry from tests a no-op.
pub fn init_tracing(
    config: &LoggingConfig,
    layers: Vec<Box<dyn Layer<Registry> + Send + Sync>>,
) -> Result<LogGuard, LoggingInitError> {
    let (subscriber, guard) = build_subscriber(config, layers)?;
    let _ = subscriber.try_init();
    Ok(guard)
}

// ---- Tests ----

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::sync::{Mutex, MutexGuard};

    static ENV_LOCK: Mutex<()> = Mutex::new(());
    fn lock_env() -> MutexGuard<'static, ()> {
        ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }
    struct EnvGuard {
        key: &'static str,
        prev: Option<String>,
    }
    impl Drop for EnvGuard {
        fn drop(&mut self) {
            // SAFETY: env vars are process-global; ENV_LOCK serializes.
            unsafe {
                match &self.prev {
                    Some(v) => std::env::set_var(self.key, v),
                    None => std::env::remove_var(self.key),
                }
            }
        }
    }
    fn set_env(key: &'static str, value: &str) -> EnvGuard {
        let prev = std::env::var(key).ok();
        // SAFETY: serialized via ENV_LOCK.
        unsafe {
            std::env::set_var(key, value);
        }
        EnvGuard { key, prev }
    }

    fn cfg(dir: &Path, prefix: &str) -> LoggingConfig {
        LoggingConfig {
            log_dir: dir.to_path_buf(),
            file_name_prefix: prefix.to_string(),
        }
    }

    fn no_layers()
    -> Vec<Box<dyn tracing_subscriber::Layer<tracing_subscriber::Registry> + Send + Sync>> {
        Vec::new()
    }

    #[test]
    fn init_tracing_creates_log_dir_when_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let log_dir = tmp.path().join("aegis-subdir");
        assert!(!log_dir.exists());
        let _g = lock_env();
        let _lvl = set_env("AEGIS_LOG_LEVEL", "info");
        let guard = init_tracing(&cfg(&log_dir, "aegis-test.log"), no_layers())
            .expect("init_tracing succeeds");
        assert!(log_dir.is_dir(), "log dir should be created");
        let _ = guard;
    }

    #[test]
    fn init_tracing_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let _g = lock_env();
        let _lvl = set_env("AEGIS_LOG_LEVEL", "info");
        let _a = init_tracing(&cfg(tmp.path(), "aegis-test.log"), no_layers()).expect("first init");
        let _b =
            init_tracing(&cfg(tmp.path(), "aegis-test.log"), no_layers()).expect("second init");
    }

    #[test]
    fn init_tracing_propagates_create_dir_failure() {
        let tmp = tempfile::tempdir().unwrap();
        let blocker = tmp.path().join("blocker");
        std::fs::write(&blocker, b"not a dir").unwrap();
        let bad = blocker.join("inside");
        let err = init_tracing(&cfg(&bad, "aegis-test.log"), no_layers()).unwrap_err();
        match err {
            LoggingInitError::CreateDir { dir, .. } => assert_eq!(dir, bad),
        }
    }

    #[test]
    fn init_tracing_writes_to_configured_file_name_prefix() {
        let tmp = tempfile::tempdir().unwrap();
        let _g = lock_env();
        let _lvl = set_env("AEGIS_LOG_LEVEL", "info");
        let guard = init_tracing(&cfg(tmp.path(), "aegis-write-test.log"), no_layers())
            .expect("init_tracing succeeds");
        tracing::info!("hello from test");
        drop(guard);
        let entries: Vec<_> = std::fs::read_dir(tmp.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with("aegis-write-test.log")
            })
            .collect();
        assert!(
            !entries.is_empty(),
            "expected at least one daily-rotate file named with the configured prefix"
        );
    }

    #[test]
    fn build_filter_defaults_to_info_when_env_missing() {
        let _g = lock_env();
        unsafe {
            std::env::remove_var("AEGIS_LOG_LEVEL");
        }
        let filter = build_filter();
        assert_eq!(filter.to_string(), "info");
    }

    #[test]
    fn build_filter_uses_aegis_log_level_when_set() {
        let _g = lock_env();
        let _lvl = set_env("AEGIS_LOG_LEVEL", "debug");
        let filter = build_filter();
        assert_eq!(filter.to_string(), "debug");
    }

    #[test]
    fn logging_config_clone_preserves_fields() {
        let original = LoggingConfig {
            log_dir: PathBuf::from("/tmp/aegis"),
            file_name_prefix: "aegis-clone-test.log".to_string(),
        };
        let cloned = original.clone();
        assert_eq!(original.log_dir, cloned.log_dir);
        assert_eq!(original.file_name_prefix, cloned.file_name_prefix);
    }

    #[test]
    fn init_tracing_installs_extra_layers() {
        use std::sync::{Arc, Mutex};

        let tmp = tempfile::tempdir().unwrap();
        let _g = lock_env();
        let _lvl = set_env("AEGIS_LOG_LEVEL", "info");

        #[derive(Clone, Default)]
        struct Counter(Arc<Mutex<usize>>);
        impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for Counter {
            fn on_event(
                &self,
                _ev: &tracing::Event<'_>,
                _ctx: tracing_subscriber::layer::Context<'_, S>,
            ) {
                *self.0.lock().unwrap() += 1;
            }
        }

        let counter = Counter::default();
        let (subscriber, guard) = build_subscriber(
            &cfg(tmp.path(), "aegis-layers.log"),
            vec![Box::new(counter.clone())],
        )
        .expect("build_subscriber succeeds");
        tracing::subscriber::with_default(subscriber, || tracing::info!("counted"));
        drop(guard);
        assert_eq!(
            *counter.0.lock().unwrap(),
            1,
            "the extra layer should see the event"
        );
    }

    #[test]
    fn init_tracing_gates_every_layer_with_the_env_filter() {
        // The filter must sit on the registry, not on the file
        // layer: on the file layer it would gate only the file and
        // a submit layer would ship DEBUG events while the process
        // runs at `info`.
        use std::sync::{Arc, Mutex};

        let tmp = tempfile::tempdir().unwrap();
        let _g = lock_env();
        let _lvl = set_env("AEGIS_LOG_LEVEL", "info");

        #[derive(Clone, Default)]
        struct Counter(Arc<Mutex<usize>>);
        impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for Counter {
            fn on_event(
                &self,
                _ev: &tracing::Event<'_>,
                _ctx: tracing_subscriber::layer::Context<'_, S>,
            ) {
                *self.0.lock().unwrap() += 1;
            }
        }

        let counter = Counter::default();
        let (subscriber, guard) = build_subscriber(
            &cfg(tmp.path(), "aegis-filter.log"),
            vec![Box::new(counter.clone())],
        )
        .expect("build_subscriber succeeds");
        tracing::subscriber::with_default(subscriber, || {
            tracing::debug!("should be filtered out");
            tracing::info!("should pass");
        });
        drop(guard);
        assert_eq!(
            *counter.0.lock().unwrap(),
            1,
            "an extra layer must be gated by AEGIS_LOG_LEVEL too"
        );
    }
}
