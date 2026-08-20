//! HTTP functions under `/api/terminology/code-lists`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::http::client::HttpClient;
use crate::http::dto::ApiError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeListViewResponse {
    pub id: i64,
    pub version_id: i64,
    pub code: String,
    pub extensible: bool,
    pub name: String,
    pub submission_value: String,
    pub synonym: String,
    pub definition: String,
    pub nci_preferred_term: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeListPagedResponse {
    pub items: Vec<CodeListViewResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_offset: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct CodeListListQuery {
    pub version_id: i64,
    pub fragment: Option<String>,
    pub offset: u32,
    pub limit: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCodeListRequest {
    pub version_id: i64,
    pub code: String,
    pub extensible: bool,
    pub name: String,
    pub submission_value: String,
    pub synonym: String,
    pub definition: String,
    pub nci_preferred_term: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCodeListRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensible: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub submission_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synonym: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nci_preferred_term: Option<String>,
}

fn percent_encode_fragment(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for b in input.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

pub async fn create(
    c: &HttpClient,
    body: CreateCodeListRequest,
) -> Result<CodeListViewResponse, ApiError> {
    c.request(
        reqwest::Method::POST,
        "/api/terminology/code-lists",
        Some(&body),
    )
    .await
}

pub async fn list_paged(
    c: &HttpClient,
    q: CodeListListQuery,
) -> Result<CodeListPagedResponse, ApiError> {
    let mut path = format!(
        "/api/terminology/code-lists?versionId={}&offset={}&limit={}",
        q.version_id, q.offset, q.limit
    );
    if let Some(f) = q.fragment.as_deref().filter(|s| !s.trim().is_empty()) {
        path.push_str("&fragment=");
        path.push_str(&percent_encode_fragment(f));
    }
    c.request(reqwest::Method::GET, &path, None::<&()>).await
}

pub async fn get_by_id(c: &HttpClient, id: i64) -> Result<CodeListViewResponse, ApiError> {
    c.request(
        reqwest::Method::GET,
        &format!("/api/terminology/code-lists/{id}"),
        None::<&()>,
    )
    .await
}

pub async fn update(
    c: &HttpClient,
    id: i64,
    body: UpdateCodeListRequest,
) -> Result<CodeListViewResponse, ApiError> {
    c.request(
        reqwest::Method::PATCH,
        &format!("/api/terminology/code-lists/{id}"),
        Some(&body),
    )
    .await
}

pub async fn delete(c: &HttpClient, id: i64) -> Result<(), ApiError> {
    let _ = c
        .request_bytes(
            reqwest::Method::DELETE,
            &format!("/api/terminology/code-lists/{id}"),
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

    fn codelist_json(id: i64, code: &str) -> serde_json::Value {
        serde_json::json!({
            "id": id, "versionId": 7, "code": code, "extensible": true,
            "name": "name", "submissionValue": "SV", "synonym": "",
            "definition": "def", "nciPreferredTerm": "nci",
            "createdAt": "2026-01-01T00:00:00Z", "updatedAt": "2026-01-01T00:00:00Z"
        })
    }

    #[tokio::test]
    async fn list_paged_returns_first_page_with_next_offset() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/terminology/code-lists"))
            .and(query_param("versionId", "7"))
            .and(query_param("offset", "0"))
            .and(query_param("limit", "20"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "codelists": [codelist_json(1, "C1"), codelist_json(2, "C2")],
                "nextOffset": 20
            })))
            .mount(&server)
            .await;
        let page = list_paged(
            &client(&server),
            CodeListListQuery {
                version_id: 7,
                fragment: None,
                offset: 0,
                limit: 20,
            },
        )
        .await
        .unwrap();
        assert_eq!(page.items.len(), 2);
        assert_eq!(page.items[0].code, "C1");
        assert_eq!(page.next_offset, Some(20));
    }

    #[tokio::test]
    async fn list_paged_returns_no_next_offset_on_last_page() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/terminology/code-lists"))
            .and(query_param("offset", "40"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "codelists": [codelist_json(41, "C41")]
            })))
            .mount(&server)
            .await;
        let page = list_paged(
            &client(&server),
            CodeListListQuery {
                version_id: 7,
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
            .and(path("/api/terminology/code-lists"))
            .and(query_param("fragment", "AE"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "codelists": [codelist_json(1, "AE")]
            })))
            .mount(&server)
            .await;
        let page = list_paged(
            &client(&server),
            CodeListListQuery {
                version_id: 7,
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
            .and(path("/api/terminology/code-lists"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": []
            })))
            .mount(&server)
            .await;
        let page = list_paged(
            &client(&server),
            CodeListListQuery {
                version_id: 7,
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
            .and(path("/api/terminology/code-lists"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "codelists": [],
                "nextOffset": 100
            })))
            .mount(&server)
            .await;
        let page = list_paged(
            &client(&server),
            CodeListListQuery {
                version_id: 7,
                fragment: None,
                offset: 0,
                limit: 20,
            },
        )
        .await
        .unwrap();
        assert_eq!(page.next_offset, Some(100));
    }

    #[tokio::test]
    async fn get_by_id_returns_view() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/terminology/code-lists/42"))
            .respond_with(ResponseTemplate::new(200).set_body_json(codelist_json(42, "C42")))
            .mount(&server)
            .await;
        let v = get_by_id(&client(&server), 42).await.unwrap();
        assert_eq!(v.id, 42);
        assert_eq!(v.code, "C42");
    }

    #[tokio::test]
    async fn create_returns_view() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/terminology/code-lists"))
            .respond_with(ResponseTemplate::new(201).set_body_json(codelist_json(99, "NEW")))
            .mount(&server)
            .await;
        let v = create(
            &client(&server),
            CreateCodeListRequest {
                version_id: 7,
                code: "NEW".into(),
                extensible: true,
                name: "name".into(),
                submission_value: "SV".into(),
                synonym: "".into(),
                definition: "def".into(),
                nci_preferred_term: "nci".into(),
            },
        )
        .await
        .unwrap();
        assert_eq!(v.id, 99);
        assert_eq!(v.code, "NEW");
    }

    #[tokio::test]
    async fn update_returns_view() {
        let server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path("/api/terminology/code-lists/3"))
            .respond_with(ResponseTemplate::new(200).set_body_json(codelist_json(3, "UPD")))
            .mount(&server)
            .await;
        let v = update(
            &client(&server),
            3,
            UpdateCodeListRequest {
                code: Some("UPD".into()),
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
            .and(path("/api/terminology/code-lists/3"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;
        delete(&client(&server), 3).await.unwrap();
    }

    #[test]
    fn update_request_skips_none_fields() {
        let body = UpdateCodeListRequest {
            name: Some("renamed".into()),
            ..Default::default()
        };
        let j = serde_json::to_string(&body).unwrap();
        assert_eq!(j, r#"{"name":"renamed"}"#);
    }
}
