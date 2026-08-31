//! HTTP functions for `DomainAnnotation`. CRUD plus a list-by-form
//! helper used by the detail page's domain-annotation chip row.

use serde::{Deserialize, Serialize};

use crate::http::client::HttpClient;
use crate::http::dto::ApiError;

pub use super::form::DomainAnnotationViewResponse;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDomainAnnotationRequest {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDomainAnnotationRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DomainAnnotationListResponse {
    pub domain_annotations: Vec<DomainAnnotationViewResponse>,
}

pub async fn create(
    c: &HttpClient,
    form_id: i64,
    body: CreateDomainAnnotationRequest,
) -> Result<DomainAnnotationViewResponse, ApiError> {
    c.request(
        reqwest::Method::POST,
        &format!("/api/crf/forms/{form_id}/domain-annotations"),
        Some(&body),
    )
    .await
}

pub async fn list_by_form(
    c: &HttpClient,
    form_id: i64,
) -> Result<DomainAnnotationListResponse, ApiError> {
    c.request(
        reqwest::Method::GET,
        &format!("/api/crf/forms/{form_id}/domain-annotations"),
        None::<&()>,
    )
    .await
}

pub async fn update(
    c: &HttpClient,
    id: i64,
    body: UpdateDomainAnnotationRequest,
) -> Result<DomainAnnotationViewResponse, ApiError> {
    c.request(
        reqwest::Method::PATCH,
        &format!("/api/crf/domain-annotations/{id}"),
        Some(&body),
    )
    .await
}

pub async fn delete(c: &HttpClient, id: i64) -> Result<(), ApiError> {
    let _ = c
        .request_bytes(
            reqwest::Method::DELETE,
            &format!("/api/crf/domain-annotations/{id}"),
            None::<&()>,
        )
        .await?;
    Ok(())
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
) -> Result<DomainAnnotationListResponse, ApiError> {
    let encoded = percent_encode_fragment(&fragment);
    c.request(
        reqwest::Method::GET,
        &format!("/api/crf/versions/{version_id}/domain-annotations/search?fragment={encoded}"),
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

    fn domain_json(id: i64) -> serde_json::Value {
        serde_json::json!({
            "id": id, "formId": 11,
            "name": "Adverse Events", "description": "AE",
            "createdAt": "2026-01-01T00:00:00Z",
            "updatedAt": "2026-01-02T00:00:00Z"
        })
    }

    #[tokio::test]
    async fn create_returns_view() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/crf/forms/11/domain-annotations"))
            .respond_with(ResponseTemplate::new(201).set_body_json(domain_json(50)))
            .mount(&server)
            .await;
        let view = create(
            &client(&server),
            11,
            CreateDomainAnnotationRequest {
                name: "Adverse Events".into(),
                description: "AE".into(),
            },
        )
        .await
        .unwrap();
        assert_eq!(view.id, 50);
    }

    #[tokio::test]
    async fn list_by_form_returns_views() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/crf/forms/11/domain-annotations"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "domainAnnotations": [domain_json(50), domain_json(51)]
            })))
            .mount(&server)
            .await;
        let resp = list_by_form(&client(&server), 11).await.unwrap();
        assert_eq!(resp.domain_annotations.len(), 2);
    }

    #[tokio::test]
    async fn update_returns_view() {
        let server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path("/api/crf/domain-annotations/50"))
            .respond_with(ResponseTemplate::new(200).set_body_json(domain_json(50)))
            .mount(&server)
            .await;
        let view = update(
            &client(&server),
            50,
            UpdateDomainAnnotationRequest {
                name: Some("Renamed".into()),
                description: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(view.name, "Adverse Events");
    }

    #[test]
    fn update_request_skips_none_fields() {
        let body = UpdateDomainAnnotationRequest {
            name: Some("renamed".into()),
            description: None,
        };
        let j = serde_json::to_string(&body).unwrap();
        assert_eq!(j, r#"{"name":"renamed"}"#);
    }

    #[tokio::test]
    async fn delete_succeeds() {
        let server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/api/crf/domain-annotations/50"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;
        delete(&client(&server), 50).await.unwrap();
    }

    #[tokio::test]
    async fn search_by_version_with_fragment_includes_query_param() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/crf/versions/7/domain-annotations/search"))
            .and(query_param("fragment", "Severity"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "domainAnnotations": [domain_json(50)]
            })))
            .mount(&server)
            .await;
        let resp = search_by_version(&client(&server), 7, "Severity".into())
            .await
            .unwrap();
        assert_eq!(resp.domain_annotations.len(), 1);
        assert_eq!(resp.domain_annotations[0].name, "Adverse Events");
    }
}
