//! Domains under `/api/domain-model/domains` and
//! `/api/domain-model/versions/{id}/domains`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::http::client::HttpClient;
use crate::http::dto::{ApiError, DomainCategory};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SdtmDomainDescriptionDetail {
    pub description: String,
    pub structure: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SdtmDomainDescription {
    pub lang: String,
    pub details: SdtmDomainDescriptionDetail,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SdtmDomainViewResponse {
    pub id: i64,
    pub version_id: i64,
    pub name: String,
    pub category: DomainCategory,
    pub descriptions: Vec<SdtmDomainDescription>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SdtmDomainListResponse {
    pub domains: Vec<SdtmDomainViewResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSdtmDomainRequest {
    pub version_id: i64,
    pub name: String,
    pub category: DomainCategory,
    pub descriptions: Vec<SdtmDomainDescription>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSdtmDomainRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<DomainCategory>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub descriptions: Option<Vec<SdtmDomainDescription>>,
}

pub async fn create(
    c: &HttpClient,
    body: CreateSdtmDomainRequest,
) -> Result<SdtmDomainViewResponse, ApiError> {
    c.request(reqwest::Method::POST, "/api/domain-model/domains", Some(&body))
        .await
}

pub async fn list_by_version(
    c: &HttpClient,
    version_id: i64,
) -> Result<Vec<SdtmDomainViewResponse>, ApiError> {
    let resp: SdtmDomainListResponse = c
        .request(
            reqwest::Method::GET,
            &format!("/api/domain-model/versions/{version_id}/domains"),
            None::<&()>,
        )
        .await?;
    Ok(resp.domains)
}

pub async fn get_by_id(
    c: &HttpClient,
    id: i64,
) -> Result<SdtmDomainViewResponse, ApiError> {
    c.request(
        reqwest::Method::GET,
        &format!("/api/domain-model/domains/{id}"),
        None::<&()>,
    )
    .await
}

pub async fn update(
    c: &HttpClient,
    id: i64,
    body: UpdateSdtmDomainRequest,
) -> Result<SdtmDomainViewResponse, ApiError> {
    c.request(
        reqwest::Method::PUT,
        &format!("/api/domain-model/domains/{id}"),
        Some(&body),
    )
    .await
}

pub async fn delete(c: &HttpClient, id: i64) -> Result<(), ApiError> {
    let _ = c
        .request_bytes(
            reqwest::Method::DELETE,
            &format!("/api/domain-model/domains/{id}"),
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
    async fn list_by_version_returns_domains() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/domain-model/versions/5/domains"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "domains": [{
                    "id": 1, "versionId": 5, "name": "AE",
                    "category": "Events",
                    "descriptions": [
                        {"lang": "en", "details": {"description": "Adverse Events", "structure": "One per AE"}}
                    ],
                    "createdAt": "2026-01-01T00:00:00Z",
                    "updatedAt": "2026-01-02T00:00:00Z"
                }]
            })))
            .mount(&server)
            .await;
        let domains = list_by_version(&client(&server), 5).await.unwrap();
        assert_eq!(domains.len(), 1);
        assert_eq!(domains[0].name, "AE");
        assert_eq!(domains[0].category, DomainCategory::Events);
        assert_eq!(domains[0].descriptions.len(), 1);
        assert_eq!(domains[0].descriptions[0].lang, "en");
        assert_eq!(
            domains[0].descriptions[0].details.description,
            "Adverse Events"
        );
        assert_eq!(
            domains[0].created_at,
            Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
        );
    }

    #[tokio::test]
    async fn create_returns_view() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/domain-model/domains"))
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
                "id": 9, "versionId": 5, "name": "DM",
                "category": "Special Purpose",
                "descriptions": [],
                "createdAt": "2026-02-01T00:00:00Z",
                "updatedAt": "2026-02-01T00:00:00Z"
            })))
            .mount(&server)
            .await;
        let v = create(
            &client(&server),
            CreateSdtmDomainRequest {
                version_id: 5,
                name: "DM".into(),
                category: DomainCategory::SpecialPurpose,
                descriptions: vec![],
            },
        )
        .await
        .unwrap();
        assert_eq!(v.id, 9);
        assert_eq!(v.name, "DM");
        assert_eq!(v.category, DomainCategory::SpecialPurpose);
    }

    #[tokio::test]
    async fn get_by_id_returns_view() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/domain-model/domains/7"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": 7, "versionId": 5, "name": "VS",
                "category": "Findings",
                "descriptions": [],
                "createdAt": "2025-12-01T00:00:00Z",
                "updatedAt": "2025-12-01T00:00:00Z"
            })))
            .mount(&server)
            .await;
        let v = get_by_id(&client(&server), 7).await.unwrap();
        assert_eq!(v.id, 7);
        assert_eq!(v.name, "VS");
    }

    #[tokio::test]
    async fn update_returns_view() {
        let server = MockServer::start().await;
        Mock::given(method("PUT"))
            .and(path("/api/domain-model/domains/7"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": 7, "versionId": 5, "name": "renamed",
                "category": "Findings",
                "descriptions": [],
                "createdAt": "2025-12-01T00:00:00Z",
                "updatedAt": "2026-03-01T00:00:00Z"
            })))
            .mount(&server)
            .await;
        let v = update(
            &client(&server),
            7,
            UpdateSdtmDomainRequest {
                name: Some("renamed".into()),
                ..Default::default()
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
            .and(path("/api/domain-model/domains/9"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;
        delete(&client(&server), 9).await.unwrap();
    }

    #[test]
    fn update_request_skips_none_fields() {
        let body = UpdateSdtmDomainRequest {
            name: Some("renamed".into()),
            ..Default::default()
        };
        let j = serde_json::to_string(&body).unwrap();
        assert_eq!(j, r#"{"name":"renamed"}"#);
    }

    #[test]
    fn update_request_skips_none_for_descriptions() {
        let body = UpdateSdtmDomainRequest {
            descriptions: Some(vec![]),
            ..Default::default()
        };
        let j = serde_json::to_string(&body).unwrap();
        assert_eq!(j, r#"{"descriptions":[]}"#);
    }
}