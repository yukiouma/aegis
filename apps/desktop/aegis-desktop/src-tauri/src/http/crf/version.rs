//! HTTP functions under `/api/crf/projects/{project_code}/versions`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::http::client::HttpClient;
use crate::http::dto::ApiError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrfVersionViewResponse {
    pub id: i64,
    pub project_code: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrfVersionListResponse {
    pub versions: Vec<CrfVersionViewResponse>,
}

/// Local error taxonomy for the `import_als` orchestrator.
///
/// Pre-validation is a fast-fail mirror of the server-side rules in
/// `lib/crates/crf/src/domain/crf_bulk_form.rs`; we surface violations
/// as `ApiError::Parse` so the page renders them through the same Snackbar
/// path as parse failures.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum AlsImportError {
    #[error("form #{form_index}: {target} must not be empty")]
    Empty { target: &'static str, form_index: usize },

    #[error("form #{form_index} item '{item_code}': {field} must not be empty")]
    EmptyItem {
        form_index: usize,
        item_code: String,
        field: &'static str,
    },

    #[error("form #{form_index} item '{item_code}': kind={kind} requires non-empty {field}")]
    KindShapeViolation {
        form_index: usize,
        item_code: String,
        kind: String,
        field: &'static str,
    },

    #[error("I/O error: {0}")]
    Io(String),
}

impl AlsImportError {
    pub(crate) fn from_io(e: std::io::Error) -> Self {
        AlsImportError::Io(e.to_string())
    }
}

impl From<AlsImportError> for ApiError {
    fn from(err: AlsImportError) -> Self {
        ApiError::Parse { message: err.to_string() }
    }
}

pub async fn list_by_project(
    c: &HttpClient,
    project_code: &str,
) -> Result<CrfVersionListResponse, ApiError> {
    c.request(
        reqwest::Method::GET,
        &format!("/api/crf/projects/{project_code}/versions"),
        None::<&()>,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::sync::Arc;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use crate::http::client::{HttpClient, MemoryStore, TokenStore};

    fn client(server: &MockServer) -> HttpClient {
        let store = Arc::new(MemoryStore::default());
        let _ = store.set_access_token("AT");
        let _ = store.set_refresh_token("RT");
        HttpClient::new(server.uri(), store)
    }

    #[tokio::test]
    async fn list_by_project_returns_versions() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/crf/projects/abc/versions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "versions": [{
                    "id": 1, "projectCode": "abc", "name": "v1",
                    "createdAt": "2026-01-01T00:00:00Z",
                    "updatedAt": "2026-01-02T00:00:00Z"
                }]
            })))
            .mount(&server)
            .await;
        let resp = list_by_project(&client(&server), "abc").await.unwrap();
        assert_eq!(resp.versions.len(), 1);
        assert_eq!(resp.versions[0].id, 1);
        assert_eq!(resp.versions[0].project_code, "abc");
        assert_eq!(resp.versions[0].name, "v1");
        assert_eq!(
            resp.versions[0].created_at,
            Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
        );
    }

    #[test]
    fn als_import_error_wraps_to_api_error_parse() {
        let err = AlsImportError::KindShapeViolation {
            form_index: 0,
            item_code: "X".to_string(),
            kind: "selection".to_string(),
            field: "options",
        };
        let api: ApiError = err.into();
        match api {
            ApiError::Parse { message } => {
                assert!(
                    message.contains("selection") && message.contains("X"),
                    "got: {message}"
                );
            }
            other => panic!("expected Parse variant, got {other:?}"),
        }
    }
}