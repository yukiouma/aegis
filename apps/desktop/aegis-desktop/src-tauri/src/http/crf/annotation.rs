//! HTTP functions for the polymorphic `Annotation` resource.
//!
//! The polymorphic owner lives in the request body, so
//! `CreateAnnotationRequest` carries `AnnotationOwner`. Reads are
//! keyed by form / item / option / unit id per the server.

use serde::{Deserialize, Serialize};

use super::form::AnnotationOwner;
use crate::http::client::HttpClient;
use crate::http::dto::ApiError;

// We re-use the AnnotationViewResponse defined in form.rs to keep
// the wire type in one place (avoids accidentally drifting the
// serde rename between two copies).
pub use super::form::AnnotationViewResponse;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAnnotationRequest {
    pub domain_annotation_id: i64,
    pub content: String,
    pub assign: bool,
    pub owner: AnnotationOwner,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAnnotationRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assign: Option<bool>,
}

pub async fn create(
    c: &HttpClient,
    body: CreateAnnotationRequest,
) -> Result<AnnotationViewResponse, ApiError> {
    c.request(reqwest::Method::POST, "/api/crf/annotations", Some(&body))
        .await
}

pub async fn update(
    c: &HttpClient,
    id: i64,
    body: UpdateAnnotationRequest,
) -> Result<AnnotationViewResponse, ApiError> {
    c.request(
        reqwest::Method::PATCH,
        &format!("/api/crf/annotations/{id}"),
        Some(&body),
    )
    .await
}

pub async fn delete(c: &HttpClient, id: i64) -> Result<(), ApiError> {
    let _ = c
        .request_bytes(
            reqwest::Method::DELETE,
            &format!("/api/crf/annotations/{id}"),
            None::<&()>,
        )
        .await?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnotationListResponse {
    pub annotations: Vec<AnnotationViewResponse>,
}

// ---- search ----

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

pub async fn search_by_version(
    c: &HttpClient,
    version_id: i64,
    fragment: String,
) -> Result<AnnotationListResponse, ApiError> {
    let encoded = percent_encode_fragment(&fragment);
    c.request(
        reqwest::Method::GET,
        &format!("/api/crf/versions/{version_id}/annotations/search?fragment={encoded}"),
        None::<&()>,
    )
    .await
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

    fn annotation_json(id: i64, owner: AnnotationOwner) -> serde_json::Value {
        let owner_json = match owner {
            AnnotationOwner::Form { id } => serde_json::json!({ "kind": "form", "id": id }),
            AnnotationOwner::Item { id } => serde_json::json!({ "kind": "item", "id": id }),
            AnnotationOwner::Option { id } => serde_json::json!({ "kind": "option", "id": id }),
            AnnotationOwner::Unit { id } => serde_json::json!({ "kind": "unit", "id": id }),
        };
        serde_json::json!({
            "id": id,
            "domainAnnotationId": 50,
            "content": "note",
            "assign": false,
            "owner": owner_json,
            "createdAt": "2026-01-01T00:00:00Z",
            "updatedAt": "2026-01-02T00:00:00Z"
        })
    }

    #[tokio::test]
    async fn create_returns_view() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/crf/annotations"))
            .respond_with(
                ResponseTemplate::new(201)
                    .set_body_json(annotation_json(100, AnnotationOwner::Form { id: 11 })),
            )
            .mount(&server)
            .await;
        let view = create(
            &client(&server),
            CreateAnnotationRequest {
                domain_annotation_id: 50,
                content: "note".into(),
                assign: false,
                owner: AnnotationOwner::Form { id: 11 },
            },
        )
        .await
        .unwrap();
        assert_eq!(view.id, 100);
        match view.owner {
            AnnotationOwner::Form { id } => assert_eq!(id, 11),
            _ => panic!("expected form owner"),
        }
    }

    #[tokio::test]
    async fn update_returns_view() {
        let server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path("/api/crf/annotations/100"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(annotation_json(100, AnnotationOwner::Item { id: 21 })),
            )
            .mount(&server)
            .await;
        let view = update(
            &client(&server),
            100,
            UpdateAnnotationRequest {
                content: Some("renamed".into()),
                assign: None,
            },
        )
        .await
        .unwrap();
        match view.owner {
            AnnotationOwner::Item { id } => assert_eq!(id, 21),
            _ => panic!("expected item owner"),
        }
    }

    #[test]
    fn update_request_skips_none_fields() {
        let body = UpdateAnnotationRequest {
            content: Some("renamed".into()),
            assign: None,
        };
        let j = serde_json::to_string(&body).unwrap();
        assert_eq!(j, r#"{"content":"renamed"}"#);
    }

    #[tokio::test]
    async fn delete_succeeds() {
        let server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/api/crf/annotations/100"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;
        delete(&client(&server), 100).await.unwrap();
    }

    #[tokio::test]
    async fn search_by_version_with_fragment_includes_query_param() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/crf/versions/7/annotations/search"))
            .and(query_param("fragment", "mild"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "annotations": [annotation_json(100, AnnotationOwner::Form { id: 11 })]
            })))
            .mount(&server)
            .await;
        let resp = search_by_version(&client(&server), 7, "mild".into())
            .await
            .unwrap();
        assert_eq!(resp.annotations.len(), 1);
        assert_eq!(resp.annotations[0].content, "note");
    }
}
