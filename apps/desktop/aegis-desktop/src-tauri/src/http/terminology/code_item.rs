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
pub struct CodeItemPagedResponse {
    pub items: Vec<CodeItemViewResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_offset: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeItemListResponse {
    pub items: Vec<CodeItemViewResponse>,
}

#[derive(Debug, Clone)]
pub struct CodeItemListQuery {
    pub codelist_id: Option<i64>,
    pub version_id: Option<i64>,
    pub fragment: Option<String>,
    pub offset: u32,
    pub limit: u32,
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

pub async fn list_paged(
    c: &HttpClient,
    q: CodeItemListQuery,
) -> Result<CodeItemPagedResponse, ApiError> {
    let mut path = String::from("/api/terminology/code-items?offset=");
    path.push_str(&q.offset.to_string());
    path.push_str("&limit=");
    path.push_str(&q.limit.to_string());
    if let Some(id) = q.codelist_id {
        path.push_str("&codelistId=");
        path.push_str(&id.to_string());
    }
    if let Some(v) = q.version_id {
        path.push_str("&versionId=");
        path.push_str(&v.to_string());
    }
    if let Some(f) = q.fragment.as_deref().filter(|s| !s.trim().is_empty()) {
        path.push_str("&fragment=");
        path.push_str(&percent_encode_fragment(f));
    }
    c.request(reqwest::Method::GET, &path, None::<&()>).await
}

pub async fn list_by_version_and_code(
    c: &HttpClient,
    version_id: i64,
    code: &str,
) -> Result<CodeItemListResponse, ApiError> {
    let path = format!(
        "/api/terminology/code-items/by-version-and-code?versionId={}&code={}",
        version_id,
        percent_encode_fragment(code),
    );
    c.request(reqwest::Method::GET, &path, None::<&()>).await
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
    async fn list_paged_returns_first_page_with_next_offset() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/terminology/code-items"))
            .and(query_param("codelistId", "11"))
            .and(query_param("offset", "0"))
            .and(query_param("limit", "20"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [item_json(1, "Y"), item_json(2, "N")],
                "nextOffset": 20
            })))
            .mount(&server)
            .await;
        let page = list_paged(
            &client(&server),
            CodeItemListQuery {
                codelist_id: Some(11),
                version_id: None,
                fragment: None,
                offset: 0,
                limit: 20,
            },
        )
        .await
        .unwrap();
        assert_eq!(page.items.len(), 2);
        assert_eq!(page.next_offset, Some(20));
    }

    #[tokio::test]
    async fn list_paged_returns_no_next_offset_on_last_page() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/terminology/code-items"))
            .and(query_param("offset", "40"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [item_json(41, "Z")]
            })))
            .mount(&server)
            .await;
        let page = list_paged(
            &client(&server),
            CodeItemListQuery {
                codelist_id: Some(11),
                version_id: None,
                fragment: None,
                offset: 40,
                limit: 20,
            },
        )
        .await
        .unwrap();
        assert_eq!(page.items.len(), 1);
        assert!(page.next_offset.is_none());
    }

    #[tokio::test]
    async fn list_paged_with_fragment_includes_fragment_query_param() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/terminology/code-items"))
            .and(query_param("fragment", "AE"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [item_json(1, "AE")]
            })))
            .mount(&server)
            .await;
        let page = list_paged(
            &client(&server),
            CodeItemListQuery {
                codelist_id: Some(11),
                version_id: None,
                fragment: Some("AE".into()),
                offset: 0,
                limit: 20,
            },
        )
        .await
        .unwrap();
        assert_eq!(page.items[0].code, "AE");
    }

    #[tokio::test]
    async fn list_paged_with_whitespace_fragment_omits_query_param() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/terminology/code-items"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": []
            })))
            .mount(&server)
            .await;
        let page = list_paged(
            &client(&server),
            CodeItemListQuery {
                codelist_id: Some(11),
                version_id: None,
                fragment: Some("   ".into()),
                offset: 0,
                limit: 20,
            },
        )
        .await
        .unwrap();
        assert!(page.items.is_empty());
    }

    #[tokio::test]
    async fn list_paged_round_trips_camel_case_next_offset() {
        // Wire is camelCase (`nextOffset`); `serde(rename_all = "camelCase")`
        // decodes the snake_case Rust field from the camelCase JSON key.
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/terminology/code-items"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [],
                "nextOffset": 60
            })))
            .mount(&server)
            .await;
        let page = list_paged(
            &client(&server),
            CodeItemListQuery {
                codelist_id: Some(11),
                version_id: None,
                fragment: None,
                offset: 0,
                limit: 20,
            },
        )
        .await
        .unwrap();
        assert_eq!(page.next_offset, Some(60));
    }

    #[tokio::test]
    async fn list_paged_with_none_codelist_id_omits_query_param() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/terminology/code-items"))
            .and(query_param("versionId", "7"))
            .and(query_param("fragment", "AE"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [item_json(1, "AE")]
            })))
            .mount(&server)
            .await;
        let page = list_paged(
            &client(&server),
            CodeItemListQuery {
                codelist_id: None,
                version_id: Some(7),
                fragment: Some("AE".into()),
                offset: 0,
                limit: 20,
            },
        )
        .await
        .unwrap();
        assert_eq!(page.items.len(), 1);
    }

    #[tokio::test]
    async fn list_paged_with_some_codelist_id_includes_query_param() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/terminology/code-items"))
            .and(query_param("codelistId", "11"))
            .and(query_param("offset", "0"))
            .and(query_param("limit", "20"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [item_json(1, "X")]
            })))
            .mount(&server)
            .await;
        let page = list_paged(
            &client(&server),
            CodeItemListQuery {
                codelist_id: Some(11),
                version_id: None,
                fragment: None,
                offset: 0,
                limit: 20,
            },
        )
        .await
        .unwrap();
        assert_eq!(page.items.len(), 1);
    }

    #[tokio::test]
    async fn list_paged_with_some_version_id_includes_query_param() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/terminology/code-items"))
            .and(query_param("versionId", "7"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [item_json(1, "Y")]
            })))
            .mount(&server)
            .await;
        let page = list_paged(
            &client(&server),
            CodeItemListQuery {
                codelist_id: None,
                version_id: Some(7),
                fragment: None,
                offset: 0,
                limit: 20,
            },
        )
        .await
        .unwrap();
        assert_eq!(page.items.len(), 1);
    }

    #[tokio::test]
    async fn list_paged_with_none_version_id_omits_query_param() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/terminology/code-items"))
            .and(query_param("codelistId", "11"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [item_json(1, "Z")]
            })))
            .mount(&server)
            .await;
        let page = list_paged(
            &client(&server),
            CodeItemListQuery {
                codelist_id: Some(11),
                version_id: None,
                fragment: None,
                offset: 0,
                limit: 20,
            },
        )
        .await
        .unwrap();
        assert_eq!(page.items.len(), 1);
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

    fn list_response_json(version_id: i64, count: usize) -> serde_json::Value {
        let items: Vec<_> = (0..count).map(|i| {
            serde_json::json!({
                "id": i, "codelistId": 10 + i as i64, "versionId": version_id,
                "code": "YES", "submissionValue": "SV",
                "synonym": "syn", "definition": "def", "nciPreferredTerm": "nci",
                "createdAt": "2026-01-01T00:00:00Z",
                "updatedAt": "2026-01-01T00:00:00Z"
            })
        }).collect();
        serde_json::json!({ "items": items })
    }

    #[tokio::test]
    async fn list_by_version_and_code_returns_items() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/terminology/code-items/by-version-and-code"))
            .and(query_param("versionId", "7"))
            .and(query_param("code", "YES"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(list_response_json(7, 2)))
            .mount(&server)
            .await;
        let resp = list_by_version_and_code(&client(&server), 7, "YES").await.unwrap();
        assert_eq!(resp.items.len(), 2);
        assert_eq!(resp.items[0].code, "YES");
        assert_eq!(resp.items[0].version_id, 7);
        assert_eq!(resp.items[1].codelist_id, 11);
    }

    #[tokio::test]
    async fn list_by_version_and_code_percent_encodes_value() {
        // Code values may contain spaces or punctuation; the wire path
        // must percent-encode them so the server parser sees the original
        // value back after URL decoding.
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/terminology/code-items/by-version-and-code"))
            .and(query_param("versionId", "7"))
            .and(query_param("code", "A B"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(list_response_json(7, 1)))
            .mount(&server)
            .await;
        let resp = list_by_version_and_code(&client(&server), 7, "A B")
            .await.unwrap();
        assert_eq!(resp.items.len(), 1);
    }
}
