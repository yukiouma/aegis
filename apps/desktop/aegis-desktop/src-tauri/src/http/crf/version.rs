//! HTTP functions under `/api/crf/projects/{project_code}/versions`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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

pub async fn list_by_project(
    c: &HttpClient,
    project_code: &str,
) -> Result<Vec<CrfVersionViewResponse>, ApiError> {
    let resp: CrfVersionListResponse = c
        .request(
            reqwest::Method::GET,
            &format!("/api/crf/projects/{project_code}/versions"),
            None::<&()>,
        )
        .await?;
    Ok(resp.versions)
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
        let versions = list_by_project(&client(&server), "abc").await.unwrap();
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].id, 1);
        assert_eq!(versions[0].project_code, "abc");
        assert_eq!(versions[0].name, "v1");
        assert_eq!(
            versions[0].created_at,
            Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
        );
    }
}