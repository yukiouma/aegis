//! `logging-utils` workspace crate.
//!
//! Unifies the `tracing` bootstrap used by `aegis-server` and
//! `aegis-desktop`. Consumers build a [`LoggingConfig`] and call
//! [`init_tracing`]; the returned [`LogGuard`] must be held for the
//! lifetime of the program.
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

pub mod tracing_init;

pub use tracing_init::{LogGuard, LoggingConfig, LoggingInitError, build_filter, init_tracing};
