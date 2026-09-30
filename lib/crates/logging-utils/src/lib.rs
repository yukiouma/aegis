//! `logging-utils` workspace crate.
//!
//! Unifies the `tracing` bootstrap used by `aegis-server` and
//! `aegis-desktop`, and owns the `TraceIdGenerator` / `Side` types
//! moved from `lib/crates/trace-id`.
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

pub mod trace_id;
pub mod tracing_init;

pub use trace_id::{Side, TraceIdGenerator};
pub use tracing_init::{LogGuard, LoggingConfig, LoggingInitError, build_filter, init_tracing};
