//! HTTP functions under `/api/crf/versions/{id}/forms` and
//! `/api/crf/forms/{id}`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::http::client::HttpClient;
use crate::http::dto::ApiError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrfFormViewResponse {
    pub id: i64,
    pub version_id: i64,
    pub code: String,
    pub name: String,
    pub order: i32,
    pub not_submitted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrfFormListResponse {
    pub forms: Vec<CrfFormViewResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCrfFormRequest {
    pub code: String,
    pub name: String,
    pub order: i32,
    pub not_submitted: bool,
}

/// Wire mirror of `apis::crf::CrfItemKind`. Used by the bulk-create
/// request and by `http::crf::version::import_als` to tag items
/// when transcribing the parsed ALS `Project` into wire shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CrfItemKind {
    Text,
    Selection,
    Checkbox,
    Datetime,
    Label,
}

/// Body for `POST /api/crf/versions/{version_id}/forms/bulk`. Owning
/// `version_id` is supplied via the path segment; the body carries
/// the form's scalar fields plus every item (each with its own
/// options + units subtree). The bulk port stamps the surrogate
/// `form_id` / `item_id` at insert time, so neither appears here.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkCreateCrfFormRequest {
    pub form: CreateCrfFormRequest,
    pub items: Vec<BulkCreateCrfFormItemInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkCreateCrfFormItemInput {
    pub item: CreateCrfItemRequest,
    pub options: Vec<CreateCrfOptionRequest>,
    pub units: Vec<CreateCrfUnitRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCrfItemRequest {
    pub code: String,
    pub name: String,
    pub kind: CrfItemKind,
    pub order: i32,
    pub not_submitted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCrfOptionRequest {
    pub value: String,
    pub not_submitted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCrfUnitRequest {
    pub value: String,
    pub not_submitted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkCreateCrfFormResponse {
    pub form: CrfFormViewResponse,
    pub items: Vec<CrfItemViewResponse>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCrfFormRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_submitted: Option<bool>,
}

pub async fn list_by_version(
    c: &HttpClient,
    version_id: i64,
) -> Result<CrfFormListResponse, ApiError> {
    c.request(
        reqwest::Method::GET,
        &format!("/api/crf/versions/{version_id}/forms"),
        None::<&()>,
    )
    .await
}

pub async fn create(
    c: &HttpClient,
    version_id: i64,
    body: CreateCrfFormRequest,
) -> Result<CrfFormViewResponse, ApiError> {
    c.request(
        reqwest::Method::POST,
        &format!("/api/crf/versions/{version_id}/forms"),
        Some(&body),
    )
    .await
}

pub async fn bulk_create(
    c: &HttpClient,
    version_id: i64,
    body: BulkCreateCrfFormRequest,
) -> Result<BulkCreateCrfFormResponse, ApiError> {
    c.request(
        reqwest::Method::POST,
        &format!("/api/crf/versions/{version_id}/forms/bulk"),
        Some(&body),
    )
    .await
}

pub async fn update(
    c: &HttpClient,
    id: i64,
    body: UpdateCrfFormRequest,
) -> Result<CrfFormViewResponse, ApiError> {
    c.request(
        reqwest::Method::PATCH,
        &format!("/api/crf/forms/{id}"),
        Some(&body),
    )
    .await
}

pub async fn delete(c: &HttpClient, id: i64) -> Result<(), ApiError> {
    let _ = c
        .request_bytes(
            reqwest::Method::DELETE,
            &format!("/api/crf/forms/{id}"),
            None::<&()>,
        )
        .await?;
    Ok(())
}

pub async fn get_by_id(c: &HttpClient, id: i64) -> Result<CrfFormViewResponse, ApiError> {
    c.request(
        reqwest::Method::GET,
        &format!("/api/crf/forms/{id}"),
        None::<&()>,
    )
    .await
}

// ---- detail composition ----

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AnnotationOwner {
    Form {
        id: i64,
    },
    Item {
        id: i64,
    },
    #[serde(rename = "option")]
    Option {
        id: i64,
    },
    Unit {
        id: i64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrfItemViewResponse {
    pub id: i64,
    pub form_id: i64,
    pub code: String,
    pub name: String,
    pub kind: String,
    pub order: i32,
    pub not_submitted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrfOptionViewResponse {
    pub id: i64,
    pub item_id: i64,
    pub value: String,
    pub not_submitted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrfUnitViewResponse {
    pub id: i64,
    pub item_id: i64,
    pub value: String,
    pub not_submitted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnotationViewResponse {
    pub id: i64,
    pub domain_annotation_id: i64,
    pub content: String,
    pub assign: bool,
    pub owner: AnnotationOwner,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DomainAnnotationViewResponse {
    pub id: i64,
    pub form_id: i64,
    pub name: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrfOptionDetailResponse {
    pub option: CrfOptionViewResponse,
    pub annotations: Vec<AnnotationViewResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrfUnitDetailResponse {
    pub unit: CrfUnitViewResponse,
    pub annotations: Vec<AnnotationViewResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrfItemDetailResponse {
    pub item: CrfItemViewResponse,
    pub options: Vec<CrfOptionDetailResponse>,
    pub units: Vec<CrfUnitDetailResponse>,
    pub annotations: Vec<AnnotationViewResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrfFormDetailResponse {
    pub form: CrfFormViewResponse,
    pub form_annotations: Vec<AnnotationViewResponse>,
    pub items: Vec<CrfItemDetailResponse>,
    pub domain_annotations: Vec<DomainAnnotationViewResponse>,
}

pub async fn details(c: &HttpClient, id: i64) -> Result<CrfFormDetailResponse, ApiError> {
    c.request(
        reqwest::Method::GET,
        &format!("/api/crf/forms/{id}/details"),
        None::<&()>,
    )
    .await
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
) -> Result<CrfFormListResponse, ApiError> {
    let encoded = percent_encode_fragment(&fragment);
    c.request(
        reqwest::Method::GET,
        &format!("/api/crf/versions/{version_id}/forms/search?fragment={encoded}"),
        None::<&()>,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
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

    fn form_view_json(id: i64, version_id: i64, code: &str, name: &str) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "versionId": version_id,
            "code": code,
            "name": name,
            "order": 0,
            "notSubmitted": false,
            "createdAt": "2026-01-01T00:00:00Z",
            "updatedAt": "2026-01-02T00:00:00Z"
        })
    }

    #[tokio::test]
    async fn list_by_version_returns_forms() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/crf/versions/7/forms"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "forms": [form_view_json(11, 7, "AE", "Adverse Events")]
            })))
            .mount(&server)
            .await;
        let resp = list_by_version(&client(&server), 7).await.unwrap();
        assert_eq!(resp.forms.len(), 1);
        assert_eq!(resp.forms[0].id, 11);
        assert_eq!(resp.forms[0].version_id, 7);
        assert_eq!(resp.forms[0].code, "AE");
        assert_eq!(resp.forms[0].name, "Adverse Events");
    }

    #[tokio::test]
    async fn create_returns_view() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/crf/versions/7/forms"))
            .respond_with(ResponseTemplate::new(201).set_body_json(form_view_json(
                11,
                7,
                "AE",
                "Adverse Events",
            )))
            .mount(&server)
            .await;
        let f = create(
            &client(&server),
            7,
            CreateCrfFormRequest {
                code: "AE".into(),
                name: "Adverse Events".into(),
                order: 0,
                not_submitted: false,
            },
        )
        .await
        .unwrap();
        assert_eq!(f.id, 11);
        assert_eq!(f.code, "AE");
        assert_eq!(
            f.created_at,
            Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
        );
    }

    #[tokio::test]
    async fn update_returns_view() {
        let server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path("/api/crf/forms/11"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(form_view_json(11, 7, "AE", "Renamed")),
            )
            .mount(&server)
            .await;
        let f = update(
            &client(&server),
            11,
            UpdateCrfFormRequest {
                code: None,
                name: Some("Renamed".into()),
                order: None,
                not_submitted: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(f.name, "Renamed");
    }

    #[tokio::test]
    async fn delete_succeeds() {
        let server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/api/crf/forms/11"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;
        delete(&client(&server), 11).await.unwrap();
    }

    #[tokio::test]
    async fn get_by_id_returns_view() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/crf/forms/11"))
            .respond_with(ResponseTemplate::new(200).set_body_json(form_view_json(
                11,
                7,
                "AE",
                "Adverse Events",
            )))
            .mount(&server)
            .await;
        let f = get_by_id(&client(&server), 11).await.unwrap();
        assert_eq!(f.id, 11);
        assert_eq!(f.code, "AE");
    }

    #[test]
    fn update_request_skips_none_fields() {
        let body = UpdateCrfFormRequest {
            code: None,
            name: Some("renamed".into()),
            order: None,
            not_submitted: None,
        };
        let j = serde_json::to_string(&body).unwrap();
        assert_eq!(j, r#"{"name":"renamed"}"#);
    }

    #[tokio::test]
    async fn details_returns_composed_view() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/crf/forms/11/details"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "form": form_view_json(11, 7, "AE", "Adverse Events"),
                "formAnnotations": [{
                    "id": 100,
                    "domainAnnotationId": 50,
                    "content": "form-level note",
                    "assign": false,
                    "owner": { "kind": "form", "id": 11 },
                    "createdAt": "2026-01-01T00:00:00Z",
                    "updatedAt": "2026-01-02T00:00:00Z"
                }],
                "items": [{
                    "item": {
                        "id": 21, "formId": 11, "code": "AETERM", "name": "Term",
                        "kind": "text", "order": 0, "notSubmitted": false,
                        "createdAt": "2026-01-01T00:00:00Z",
                        "updatedAt": "2026-01-02T00:00:00Z"
                    },
                    "options": [],
                    "units": [],
                    "annotations": []
                }],
                "domainAnnotations": [{
                    "id": 50, "formId": 11,
                    "name": "Adverse Events", "description": "AE",
                    "createdAt": "2026-01-01T00:00:00Z",
                    "updatedAt": "2026-01-02T00:00:00Z"
                }]
            })))
            .mount(&server)
            .await;
        let resp = details(&client(&server), 11).await.unwrap();
        assert_eq!(resp.form.id, 11);
        assert_eq!(resp.form_annotations.len(), 1);
        assert_eq!(resp.items.len(), 1);
        assert_eq!(resp.items[0].item.code, "AETERM");
        assert_eq!(resp.domain_annotations.len(), 1);
        assert_eq!(resp.domain_annotations[0].name, "Adverse Events");
    }

    #[tokio::test]
    async fn search_by_version_with_fragment_includes_query_param() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/crf/versions/7/forms/search"))
            .and(query_param("fragment", "AE"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "forms": [form_view_json(11, 7, "AE", "Adverse Events")]
            })))
            .mount(&server)
            .await;
        let resp = search_by_version(&client(&server), 7, "AE".into())
            .await
            .unwrap();
        assert_eq!(resp.forms.len(), 1);
        assert_eq!(resp.forms[0].id, 11);
    }
}
