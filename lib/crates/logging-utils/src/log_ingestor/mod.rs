//! `log_ingestor` — batched, deduplicated, bounded-queue log writer.
//!
//! See the [`LogIngestor`](crate::LogIngestor) docs and the design
//! doc `docs/superpowers/specs/2026-10-02-log-ingestor-design.md`.

mod config;
mod ingestor;
mod writer;

pub(super) use writer::BatchEnvelope;
