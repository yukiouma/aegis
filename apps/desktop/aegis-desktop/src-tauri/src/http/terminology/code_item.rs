//! HTTP functions under `/api/terminology/code-items`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::http::client::HttpClient;
use crate::http::dto::ApiError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeItemViewResponse {
    pub id: i64,
    pub codelist_id: i64,
    pub version_id: i64,
    pub code: String,
    pub submission_value: String,
    pub synonym: String,
    pub definition: String,
    pub nci_preferred_term: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeItemListResponse {
    pub items: Vec<CodeItemViewResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeItemSearchHitsResponse {
    pub hits: Vec<CodeItemSearchHitResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeItemSearchHitResponse {
    pub item: CodeItemViewResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCodeItemRequest {
    pub codelist_id: i64,
    pub version_id: i64,
    pub code: String,
    pub submission_value: String,
    pub synonym: String,
    pub definition: String,
    pub nci_preferred_term: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCodeItemRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub submission_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synonym: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nci_preferred_term: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchCodeItemEntry {
    pub code: String,
    pub submission_value: String,
    pub synonym: String,
    pub definition: String,
    pub nci_preferred_term: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchCreateCodeItemsRequest {
    pub codelist_id: i64,
    pub version_id: i64,
    pub items: Vec<BatchCodeItemEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchCreateCodeItemsResponse {
    pub count: usize,
    pub codelist_id: i64,
    pub version_id: i64,
}

#[derive(Debug, Clone)]
pub struct CodeItemSearchQuery {
    pub version_id: i64,
    pub fragment: String,
    pub limit: u32,
}

pub async fn create(
    c: &HttpClient,
    body: CreateCodeItemRequest,
) -> Result<CodeItemViewResponse, ApiError> {
    c.request(
        reqwest::Method::POST,
        "/api/terminology/code-items",
        Some(&body),
    )
    .await
}

pub async fn batch_create(
    c: &HttpClient,
    body: BatchCreateCodeItemsRequest,
) -> Result<BatchCreateCodeItemsResponse, ApiError> {
    c.request(
        reqwest::Method::POST,
        "/api/terminology/code-items/batch",
        Some(&body),
    )
    .await
}

pub async fn list(
    c: &HttpClient,
    codelist_id: i64,
) -> Result<Vec<CodeItemViewResponse>, ApiError> {
    let path = format!("/api/terminology/code-items?codelistId={codelist_id}");
    let resp: CodeItemListResponse =
        c.request(reqwest::Method::GET, &path, None::<&()>).await?;
    Ok(resp.items)
}

pub async fn update(
    c: &HttpClient,
    id: i64,
    body: UpdateCodeItemRequest,
) -> Result<CodeItemViewResponse, ApiError> {
    c.request(
        reqwest::Method::PATCH,
        &format!("/api/terminology/code-items/{id}"),
        Some(&body),
    )
    .await
}

pub async fn delete(c: &HttpClient, id: i64) -> Result<(), ApiError> {
    let _ = c
        .request_bytes(
            reqwest::Method::DELETE,
            &format!("/api/terminology/code-items/{id}"),
            None::<&()>,
        )
        .await?;
    Ok(())
}

pub async fn search(
    c: &HttpClient,
    query: CodeItemSearchQuery,
) -> Result<Vec<CodeItemSearchHitResponse>, ApiError> {
    if query.fragment.trim().is_empty() {
        return Err(ApiError::Http {
            status: 400,
            code: "validation_failed".into(),
            message: "search fragment must not be empty".into(),
        });
    }
    let fragment_encoded = percent_encode_fragment(&query.fragment);
    let path = format!(
        "/api/terminology/code-items/search?versionId={}&fragment={}&limit={}",
        query.version_id, fragment_encoded, query.limit
    );
    let resp: CodeItemSearchHitsResponse =
        c.request(reqwest::Method::GET, &path, None::<&()>).await?;
    Ok(resp.hits)
}

/// Minimal URL fragment encoding for query-string `fragment=`. See
/// `code_list.rs` for the rationale.
fn percent_encode_fragment(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for b in input.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9'
            | b'-' | b'.' | b'_' | b'~' => out.push(b as char),
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use crate::http::client::{HttpClient, MemoryStore, TokenStore};

    fn client(server: &MockServer) -> HttpClient {
        let store = Arc::new(MemoryStore::default());
        let _ = store.set_access_token("AT");
        let _ = store.set_refresh_token("RT");
        HttpClient::new(server.uri(), store)
    }

    fn item_json(id: i64, code: &str) -> serde_json::Value {
        serde_json::json!({
            "id": id, "codelistId": 11, "versionId": 7, "code": code,
            "submissionValue": "SV", "synonym": "syn",
            "definition": "def", "nciPreferredTerm": "nci",
            "createdAt": "2026-01-01T00:00:00Z", "updatedAt": "2026-01-01T00:00:00Z"
        })
    }

    #[tokio::test]
    async fn list_returns_items() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/terminology/code-items"))
            .and(query_param("codelistId", "11"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [item_json(1, "Y"), item_json(2, "N")]
            })))
            .mount(&server)
            .await;
        let items = list(&client(&server), 11).await.unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].code, "Y");
    }

    #[tokio::test]
    async fn create_returns_view() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/terminology/code-items"))
            .respond_with(ResponseTemplate::new(201).set_body_json(item_json(99, "NEW")))
            .mount(&server)
            .await;
        let v = create(
            &client(&server),
            CreateCodeItemRequest {
                codelist_id: 11,
                version_id: 7,
                code: "NEW".into(),
                submission_value: "SV".into(),
                synonym: "syn".into(),
                definition: "def".into(),
                nci_preferred_term: "nci".into(),
            },
        )
        .await
        .unwrap();
        assert_eq!(v.id, 99);
    }

    #[tokio::test]
    async fn update_returns_view() {
        let server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path("/api/terminology/code-items/3"))
            .respond_with(ResponseTemplate::new(200).set_body_json(item_json(3, "UPD")))
            .mount(&server)
            .await;
        let v = update(
            &client(&server),
            3,
            UpdateCodeItemRequest {
                submission_value: Some("SV-UPD".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(v.code, "UPD");
    }

    #[tokio::test]
    async fn delete_succeeds() {
        let server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/api/terminology/code-items/3"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;
        delete(&client(&server), 3).await.unwrap();
    }

    #[tokio::test]
    async fn search_returns_hits() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/terminology/code-items/search"))
            .and(query_param("versionId", "7"))
            .and(query_param("fragment", "alzheimer"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "hits": [{ "item": item_json(11, "ALZ") }]
            })))
            .mount(&server)
            .await;
        let hits = search(
            &client(&server),
            CodeItemSearchQuery {
                version_id: 7,
                fragment: "alzheimer".into(),
                limit: 50,
            },
        )
        .await
        .unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].item.code, "ALZ");
    }

    #[tokio::test]
    async fn search_rejects_empty_fragment() {
        let server = MockServer::start().await;
        let c = client(&server);
        let res = search(
            &c,
            CodeItemSearchQuery {
                version_id: 7,
                fragment: "  ".into(),
                limit: 50,
            },
        )
        .await;
        assert!(matches!(res, Err(ApiError::Http { status: 400, .. })));
    }

    #[test]
    fn update_request_skips_none_fields() {
        let body = UpdateCodeItemRequest {
            submission_value: Some("SV".into()),
            ..Default::default()
        };
        let j = serde_json::to_string(&body).unwrap();
        assert_eq!(j, r#"{"submissionValue":"SV"}"#);
    }

    fn batch_response_json(codelist_id: i64, version_id: i64, count: usize) -> serde_json::Value {
        serde_json::json!({
            "count": count, "codelistId": codelist_id, "versionId": version_id
        })
    }

    #[tokio::test]
    async fn batch_create_returns_count() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/terminology/code-items/batch"))
            .respond_with(ResponseTemplate::new(201)
                .set_body_json(batch_response_json(11, 7, 42)))
            .mount(&server)
            .await;
        let resp = batch_create(
            &client(&server),
            BatchCreateCodeItemsRequest {
                codelist_id: 11,
                version_id: 7,
                items: vec![BatchCodeItemEntry {
                    code: "Y".into(),
                    submission_value: "SV".into(),
                    synonym: "syn".into(),
                    definition: "def".into(),
                    nci_preferred_term: "nci".into(),
                }],
            },
        )
        .await
        .unwrap();
        assert_eq!(resp.count, 42);
        assert_eq!(resp.codelist_id, 11);
        assert_eq!(resp.version_id, 7);
    }

    #[test]
    fn batch_request_serializes_camel_case() {
        let body = BatchCreateCodeItemsRequest {
            codelist_id: 11,
            version_id: 7,
            items: vec![BatchCodeItemEntry {
                code: "Y".into(),
                submission_value: "SV".into(),
                synonym: "syn".into(),
                definition: "def".into(),
                nci_preferred_term: "nci".into(),
            }],
        };
        let j = serde_json::to_string(&body).unwrap();
        assert_eq!(
            j,
            r#"{"codelistId":11,"versionId":7,"items":[{"code":"Y","submissionValue":"SV","synonym":"syn","definition":"def","nciPreferredTerm":"nci"}]}"#
        );
    }
}