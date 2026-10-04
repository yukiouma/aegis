//! The transport seam for [`crate::LogSubmitter`]. The submitter
//! never speaks HTTP itself; it hands a batch id and its entries to
//! a [`LogSender`] and interprets the result.

use std::fmt::Debug;
use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;
use thiserror::Error;

/// Everything that can go wrong between minting a batch and getting
/// it accepted by the sink. One error type per boundary, mirroring
/// `IngestorError` on the server side; each variant wraps its inner
/// cause with `#[source]`.
#[derive(Debug, Error)]
pub enum SubmitterError {
    #[error("failed to create pending directory {dir}: {source}")]
    CreateDir {
        dir: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to persist batch {batch_id} to the pending directory: {source}")]
    Persist {
        batch_id: String,
        #[source]
        source: std::io::Error,
    },
    #[error("submitting batch {batch_id} failed: {source}")]
    Send {
        batch_id: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    #[error("the submitter worker is gone")]
    ChannelClosed,
    #[error("submitter worker thread panicked: {0}")]
    WorkerPanic(String),
    #[error("submitter worker thread failed to join within {deadline:?}: {message}")]
    WorkerJoinTimeout { deadline: Duration, message: String },
}

/// Delivers a batch of log lines to the sink.
///
/// `Debug` is a supertrait (not just a bound) so `LogSubmitterConfig`
/// can derive `Debug` alongside `Clone`.
#[async_trait]
pub trait LogSender: Debug + Send + Sync {
    /// Submit one batch. `batch_id` identifies the batch for
    /// idempotency; a sink that has already accepted a given id must
    /// say so rather than accepting it twice.
    async fn send(&self, batch_id: &str, log_entries: Vec<String>) -> Result<(), SubmitterError>;

    /// Classify `err` as "this batch will never be accepted".
    ///
    /// The submitter's drain stops at its first failure, so anything
    /// the sink will reject identically on every retry has to be
    /// recognised here and *deleted* instead — otherwise one such
    /// batch sits at the head of the queue forever and every older
    /// batch and every future one is blocked behind it.
    ///
    /// Two shapes matter against `aegis-server`:
    ///
    /// * `409 duplicate_batch_id` — the sink already ingested the
    ///   batch and the response was lost in transit, so the entries
    ///   are safe; the file is just stale. (The server's dedup cache
    ///   has a TTL, so after it expires a retry would re-ingest
    ///   rather than 409 — benign, since the entries are identical.)
    /// * Any other 4xx the server will keep refusing — an over-cap
    ///   or malformed batch answers `400 validation_failed` on every
    ///   attempt, and that is just as permanent as a 409.
    ///
    /// Transient failures (network, 5xx, 401 before the user has
    /// logged in, 408, 429) must return `false`: they clear on their
    /// own, and stopping the drain for them is correct.
    ///
    /// Defaults to `false`, which is correct for sinks that accept
    /// everything they are given.
    fn is_permanent(&self, _err: &SubmitterError) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Default)]
    struct Silent;

    #[async_trait::async_trait]
    impl LogSender for Silent {
        async fn send(
            &self,
            _batch_id: &str,
            _log_entries: Vec<String>,
        ) -> Result<(), SubmitterError> {
            Ok(())
        }
    }

    #[test]
    fn is_permanent_defaults_to_false() {
        let s = Silent;
        let err = SubmitterError::ChannelClosed;
        assert!(!s.is_permanent(&err));
    }

    #[test]
    fn send_error_reports_the_batch_id() {
        let err = SubmitterError::Send {
            batch_id: "dev-01HQ".into(),
            source: Box::new(std::io::Error::other("boom")),
        };
        assert!(err.to_string().contains("dev-01HQ"));
    }

    #[test]
    fn send_error_keeps_the_source_chain() {
        let io = std::io::Error::other("disk gone");
        let err = SubmitterError::Send {
            batch_id: "b".into(),
            source: Box::new(io),
        };
        assert!(std::error::Error::source(&err).is_some());
    }

    #[test]
    fn worker_panic_error_renders_the_payload() {
        let err = SubmitterError::WorkerPanic("thread died".into());
        assert!(err.to_string().contains("thread died"));
    }
}
