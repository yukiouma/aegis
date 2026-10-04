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
    WorkerJoinTimeout {
        deadline: Duration,
        message: String,
    },
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
    async fn send(
        &self,
        batch_id: &str,
        log_entries: Vec<String>,
    ) -> Result<(), SubmitterError>;

    /// Classify `err` as "the sink already has this batch".
    ///
    /// This exists because the server dedups on `batch_id`: if a
    /// batch is ingested but the response is lost in transit, the
    /// submitter persists it and the retry is answered `409
    /// duplicate_batch_id` forever. Treating that as success deletes
    /// the pending file and lets the drain continue; treating it as
    /// a failure would wedge the queue permanently, because the
    /// drain stops at the first failure.
    ///
    /// Defaults to `false`, which is correct for sinks without
    /// server-side dedup.
    fn is_already_ingested(&self, _err: &SubmitterError) -> bool {
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
    fn is_already_ingested_defaults_to_false() {
        let s = Silent;
        let err = SubmitterError::ChannelClosed;
        assert!(!s.is_already_ingested(&err));
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
