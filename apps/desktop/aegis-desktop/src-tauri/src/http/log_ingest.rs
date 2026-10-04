//! `LogSender` over the existing `HttpClient`.
//!
//! There is deliberately no second request client here. `HttpClient`
//! already carries the Bearer header and the 401 auto-refresh; a
//! second one would silently stop submitting the moment an access
//! token expired — which, for a log submitter, is exactly when you
//! most want the logs.

use std::sync::Arc;

use async_trait::async_trait;
use logging_utils::{LogSender, SubmitterError};
use serde::{Deserialize, Serialize};

use super::client::HttpClient;
use super::dto::ApiError;

/// The server's error `code` for a batch it has already ingested.
const DUPLICATE_BATCH_ID: &str = "duplicate_batch_id";

/// Submits batches to `POST /api/log-ingest/submit`.
pub struct HttpLogSender {
    client: Arc<HttpClient>,
}

/// Hand-written because `HttpClient` is not `Debug` (its
/// `Arc<dyn TokenStore>` is not either), and `LogSender` requires
/// `Debug` as a supertrait. `Debug` is only needed so the config can
/// derive it; there is nothing about the client worth printing, and
/// this avoids adding `Debug` to the shared client just for the log
/// sender.
impl std::fmt::Debug for HttpLogSender {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpLogSender").finish_non_exhaustive()
    }
}

/// The wire body. `camelCase` because the server's DTO is.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SubmitBody<'a> {
    batch_id: &'a str,
    entries: &'a [String],
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SubmitResponse {
    #[allow(dead_code)]
    batch_id: String,
    #[allow(dead_code)]
    accepted: usize,
}

impl HttpLogSender {
    pub fn new(client: Arc<HttpClient>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl LogSender for HttpLogSender {
    async fn send(&self, batch_id: &str, log_entries: Vec<String>) -> Result<(), SubmitterError> {
        let body = SubmitBody {
            batch_id,
            entries: &log_entries,
        };
        let _: SubmitResponse = self
            .client
            .request(reqwest::Method::POST, "/api/log-ingest/submit", Some(&body))
            .await
            .map_err(|source| SubmitterError::Send {
                batch_id: batch_id.to_string(),
                source: Box::new(source),
            })?;
        Ok(())
    }

    /// A batch the server already holds is not a failure — it is this
    /// batch, already ingested. The submitter's drain stops at its
    /// first failure, so without this a single lost response would
    /// park that batch at the head of the queue forever and block
    /// every older batch and every future one behind it.
    ///
    /// The same reasoning covers every other 4xx the server will keep
    /// refusing — most importantly `400 validation_failed` for a batch
    /// over its 10 000-entry cap, which is permanent in exactly the
    /// same way a 409 is. 401 (the user has not logged in yet), 408
    /// and 429 clear on their own, so they keep the drain blocked.
    fn is_permanent(&self, err: &SubmitterError) -> bool {
        match err {
            SubmitterError::Send { source, .. } => {
                source.downcast_ref::<ApiError>().is_some_and(|e| match e {
                    ApiError::Http { code, .. } if code == DUPLICATE_BATCH_ID => true,
                    ApiError::Http { status, .. } => {
                        (400..500).contains(status)
                            && *status != 401
                            && *status != 408
                            && *status != 429
                    }
                    _ => false,
                })
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::client::{MemoryStore, TokenStore};
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn entries() -> Vec<String> {
        vec![
            "{\"level\":\"info\"}".to_string(),
            "{\"level\":\"warn\"}".to_string(),
        ]
    }

    #[tokio::test]
    async fn sends_a_camel_case_body_to_the_submit_endpoint() {
        let server = MockServer::start().await;
        let store = Arc::new(MemoryStore::default());
        store.set_access_token("AT").await.unwrap();
        server
            .register(
                Mock::given(method("POST"))
                    .and(path("/api/log-ingest/submit"))
                    .and(header("authorization", "Bearer AT"))
                    .respond_with(
                        ResponseTemplate::new(200)
                            .set_body_json(serde_json::json!({"batchId": "dev-1", "accepted": 2})),
                    ),
            )
            .await;

        let s = HttpLogSender::new(Arc::new(HttpClient::new(server.uri(), store)));
        s.send("dev-1", entries()).await.unwrap();

        let requests = server.received_requests().await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
        assert_eq!(body["batchId"], "dev-1");
        assert_eq!(body["entries"].as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn maps_a_server_error_to_submit_error_send() {
        let server = MockServer::start().await;
        server
            .register(
                Mock::given(method("POST"))
                    .and(path("/api/log-ingest/submit"))
                    .respond_with(ResponseTemplate::new(500).set_body_string("boom")),
            )
            .await;
        let s = HttpLogSender::new(Arc::new(HttpClient::new(
            server.uri(),
            Arc::new(MemoryStore::default()),
        )));
        let err = s.send("dev-1", entries()).await.unwrap_err();
        match &err {
            SubmitterError::Send { batch_id, source } => {
                assert_eq!(batch_id, "dev-1");
                let api = source
                    .downcast_ref::<ApiError>()
                    .expect("the ApiError should survive as the source");
                assert!(
                    matches!(api, ApiError::Http { status: 500, .. }),
                    "got {api:?}"
                );
            }
            other => panic!("expected Send, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn maps_a_network_failure_to_submit_error_send() {
        // Bind then drop a listener so the port refuses connections.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);
        let s = HttpLogSender::new(Arc::new(HttpClient::new(
            format!("http://{addr}"),
            Arc::new(MemoryStore::default()),
        )));
        let err = s.send("dev-1", entries()).await.unwrap_err();
        assert!(matches!(err, SubmitterError::Send { .. }), "got {err:?}");
    }

    /// End-to-end: a real 409 response, through `send`, classified.
    /// The hand-built variant below cannot catch a change to how
    /// `SubmitterError::Send` boxes its source, which would silently
    /// stop the wedge fix from working.
    #[tokio::test]
    async fn a_real_409_is_classified_as_permanent() {
        let server = MockServer::start().await;
        server
            .register(
                Mock::given(method("POST"))
                    .and(path("/api/log-ingest/submit"))
                    .respond_with(ResponseTemplate::new(409).set_body_json(
                        serde_json::json!({"code": "duplicate_batch_id", "message": "already"}),
                    )),
            )
            .await;
        let s = HttpLogSender::new(Arc::new(HttpClient::new(
            server.uri(),
            Arc::new(MemoryStore::default()),
        )));
        let err = s.send("dev-1", entries()).await.unwrap_err();
        assert!(
            s.is_permanent(&err),
            "a real 409 must be permanent: {err:?}"
        );
    }

    #[tokio::test]
    async fn a_real_400_validation_failure_is_classified_as_permanent() {
        let server = MockServer::start().await;
        server
            .register(
                Mock::given(method("POST"))
                    .and(path("/api/log-ingest/submit"))
                    .respond_with(ResponseTemplate::new(400).set_body_json(
                        serde_json::json!({"code": "validation_failed", "message": "too many"}),
                    )),
            )
            .await;
        let s = HttpLogSender::new(Arc::new(HttpClient::new(
            server.uri(),
            Arc::new(MemoryStore::default()),
        )));
        let err = s.send("dev-1", entries()).await.unwrap_err();
        assert!(
            s.is_permanent(&err),
            "an over-cap batch is permanent: {err:?}"
        );
    }

    #[tokio::test]
    async fn a_401_is_not_permanent_so_the_drain_recovers_after_login() {
        let server = MockServer::start().await;
        server
            .register(
                Mock::given(method("POST"))
                    .and(path("/api/log-ingest/submit"))
                    .respond_with(ResponseTemplate::new(401).set_body_json(
                        serde_json::json!({"code": "token_verification_failed", "message": "no"}),
                    )),
            )
            .await;
        let s = HttpLogSender::new(Arc::new(HttpClient::new(
            server.uri(),
            Arc::new(MemoryStore::default()),
        )));
        let err = s.send("dev-1", entries()).await.unwrap_err();
        assert!(!s.is_permanent(&err), "401 clears once logged in: {err:?}");
    }

    #[test]
    fn recognizes_a_duplicate_batch_id_as_already_ingested() {
        let s = HttpLogSender::new(Arc::new(HttpClient::new(
            "http://127.0.0.1:1".into(),
            Arc::new(MemoryStore::default()),
        )));
        let duplicate = SubmitterError::Send {
            batch_id: "dev-1".into(),
            source: Box::new(ApiError::Http {
                status: 409,
                code: "duplicate_batch_id".into(),
                message: "already ingested".into(),
            }),
        };
        let other = SubmitterError::Send {
            batch_id: "dev-1".into(),
            source: Box::new(ApiError::Http {
                status: 500,
                code: "internal".into(),
                message: "boom".into(),
            }),
        };
        assert!(s.is_permanent(&duplicate));
        assert!(!s.is_permanent(&other));
        assert!(!s.is_permanent(&SubmitterError::ChannelClosed));
    }
}
