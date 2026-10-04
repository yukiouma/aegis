//! `log_submitter` — batched, bounded-queue, disk-backed log
//! submitter for `aegis-desktop`. The producer-side counterpart to
//! `log_ingestor`, which is the server-side consumer.
//!
//! See `docs/superpowers/specs/2026-10-04-log-submitter-design.md`.

mod sender;

pub use sender::{LogSender, SubmitterError};
