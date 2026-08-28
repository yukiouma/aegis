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

pub async fn get_by_id(
    c: &HttpClient,
    id: i64,
) -> Result<CrfFormViewResponse, ApiError> {
    c.request(
        reqwest::Method::GET,
        &format!("/api/crf/forms/{id}"),
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
            .respond_with(ResponseTemplate::new(201).set_body_json(form_view_json(11, 7, "AE", "Adverse Events")))
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
            .respond_with(ResponseTemplate::new(200).set_body_json(form_view_json(11, 7, "AE", "Renamed")))
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
            .respond_with(ResponseTemplate::new(200).set_body_json(form_view_json(11, 7, "AE", "Adverse Events")))
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
}