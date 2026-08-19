//! HTTP functions under `/api/terminology/versions`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::http::client::HttpClient;
use crate::http::dto::{ApiError, TerminologyKind};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminologyVersionViewResponse {
    pub id: i64,
    pub kind: TerminologyKind,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminologyVersionListResponse {
    pub versions: Vec<TerminologyVersionViewResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTerminologyVersionRequest {
    pub kind: TerminologyKind,
    pub name: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTerminologyVersionRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<TerminologyKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

pub async fn create(
    c: &HttpClient,
    body: CreateTerminologyVersionRequest,
) -> Result<TerminologyVersionViewResponse, ApiError> {
    c.request(reqwest::Method::POST, "/api/terminology/versions", Some(&body))
        .await
}

pub async fn list(c: &HttpClient) -> Result<Vec<TerminologyVersionViewResponse>, ApiError> {
    let resp: TerminologyVersionListResponse = c
        .request(
            reqwest::Method::GET,
            "/api/terminology/versions",
            None::<&()>,
        )
        .await?;
    Ok(resp.versions)
}

pub async fn get_by_id(
    c: &HttpClient,
    id: i64,
) -> Result<TerminologyVersionViewResponse, ApiError> {
    c.request(
        reqwest::Method::GET,
        &format!("/api/terminology/versions/{id}"),
        None::<&()>,
    )
    .await
}

pub async fn update(
    c: &HttpClient,
    id: i64,
    body: UpdateTerminologyVersionRequest,
) -> Result<TerminologyVersionViewResponse, ApiError> {
    c.request(
        reqwest::Method::PATCH,
        &format!("/api/terminology/versions/{id}"),
        Some(&body),
    )
    .await
}

pub async fn delete(c: &HttpClient, id: i64) -> Result<(), ApiError> {
    // Use `request_bytes` so we don't try to deserialize an empty 204 body
    // into a typed TResp.
    let _ = c
        .request_bytes(
            reqwest::Method::DELETE,
            &format!("/api/terminology/versions/{id}"),
            None::<&()>,
        )
        .await?;
    Ok(())
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
    async fn list_returns_versions() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/terminology/versions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "versions": [{
                    "id": 1, "kind": "sdtm", "name": "2024-06-28",
                    "createdAt": "2026-01-01T00:00:00Z",
                    "updatedAt": "2026-01-02T00:00:00Z"
                }]
            })))
            .mount(&server)
            .await;
        let versions = list(&client(&server)).await.unwrap();
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].kind, TerminologyKind::Sdtm);
        assert_eq!(versions[0].name, "2024-06-28");
        assert_eq!(
            versions[0].created_at,
            Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
        );
    }

    #[tokio::test]
    async fn create_returns_view() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/terminology/versions"))
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
                "id": 7, "kind": "adam", "name": "2024-09-30",
                "createdAt": "2026-02-01T00:00:00Z",
                "updatedAt": "2026-02-01T00:00:00Z"
            })))
            .mount(&server)
            .await;
        let v = create(
            &client(&server),
            CreateTerminologyVersionRequest {
                kind: TerminologyKind::Adam,
                name: "2024-09-30".into(),
            },
        )
        .await
        .unwrap();
        assert_eq!(v.id, 7);
        assert_eq!(v.kind, TerminologyKind::Adam);
        assert_eq!(v.name, "2024-09-30");
    }

    #[tokio::test]
    async fn get_by_id_returns_view() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/terminology/versions/3"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": 3, "kind": "sdtm", "name": "2023-12-15",
                "createdAt": "2025-12-01T00:00:00Z",
                "updatedAt": "2025-12-01T00:00:00Z"
            })))
            .mount(&server)
            .await;
        let v = get_by_id(&client(&server), 3).await.unwrap();
        assert_eq!(v.id, 3);
        assert_eq!(v.name, "2023-12-15");
    }

    #[tokio::test]
    async fn update_returns_view() {
        let server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path("/api/terminology/versions/3"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": 3, "kind": "sdtm", "name": "renamed",
                "createdAt": "2025-12-01T00:00:00Z",
                "updatedAt": "2026-03-01T00:00:00Z"
            })))
            .mount(&server)
            .await;
        let v = update(
            &client(&server),
            3,
            UpdateTerminologyVersionRequest {
                kind: None,
                name: Some("renamed".into()),
            },
        )
        .await
        .unwrap();
        assert_eq!(v.name, "renamed");
    }

    #[tokio::test]
    async fn delete_succeeds() {
        let server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/api/terminology/versions/3"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;
        delete(&client(&server), 3).await.unwrap();
    }

    #[test]
    fn update_request_skips_none_fields() {
        let body = UpdateTerminologyVersionRequest {
            kind: None,
            name: Some("renamed".into()),
        };
        let j = serde_json::to_string(&body).unwrap();
        assert_eq!(j, r#"{"name":"renamed"}"#);
    }
}