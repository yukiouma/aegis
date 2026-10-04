//! `logging-utils` workspace crate.
//!
//! Unifies the `tracing` bootstrap used by `aegis-server` and
//! `aegis-desktop`, owns the `LogIngestor` for `aegis-desktop`'s
//! client log submissions (deduplicated, bounded-queue, daily-rotating
//! file writer), and carries an independent copy of the
//! `TraceIdGenerator` / `Side` types (mirrored from
//! `lib/crates/trace-id`). The two `trace-id` crates do not depend on
//! each other.
//!
//! ```ignore
//! use logging_utils::{LoggingConfig, init_tracing};
//!
//! let _guard = init_tracing(&LoggingConfig {
//!     log_dir: "./logs".into(),
//!     file_name_prefix: "aegis-server.log".into(),
//! })?;
//! # Ok::<(), logging_utils::LoggingInitError>(())
//! ```

pub mod log_ingestor;
pub mod log_submitter;
pub mod trace_id;
pub mod tracing_init;

pub use log_ingestor::{IngestorError, LogIngestor, LogIngestorConfig, LogIngestorConfigBuilder};
pub use log_submitter::{LogSender, SubmitterError};
pub use trace_id::{Side, TraceIdGenerator};
pub use tracing_init::{LogGuard, LoggingConfig, LoggingInitError, build_filter, init_tracing};
