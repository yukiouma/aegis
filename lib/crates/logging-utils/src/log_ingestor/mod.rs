//! `log_ingestor` — batched, deduplicated, bounded-queue log writer.
//!
//! See the [`LogIngestor`] docs and the design doc
//! `docs/superpowers/specs/2026-10-02-log-ingestor-design.md`.

mod config;
mod ingestor;
mod writer;

pub use config::{LogIngestorConfig, LogIngestorConfigBuilder};
pub use ingestor::{IngestorError, LogIngestor};
