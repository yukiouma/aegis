//! `POST /api/log-ingest/submit` — accept client log batches and
//! route them through [`logging_utils::LogIngestor`].

pub mod handlers;
pub mod router;

pub use router::router;
