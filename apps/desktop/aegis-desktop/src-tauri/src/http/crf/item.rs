//! HTTP functions under `/api/crf/items/{id}` and `/api/crf/forms/{id}/items`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::http::client::HttpClient;
use crate::http::dto::ApiError;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrfItemListResponse {
    pub items: Vec<CrfItemViewResponse>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCrfItemRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_submitted: Option<bool>,
}

pub async fn list_by_form(
    c: &HttpClient,
    form_id: i64,
) -> Result<CrfItemListResponse, ApiError> {
    c.request(
        reqwest::Method::GET,
        &format!("/api/crf/forms/{form_id}/items"),
        None::<&()>,
    )
    .await
}

pub async fn get_by_id(
    c: &HttpClient,
    id: i64,
) -> Result<CrfItemViewResponse, ApiError> {
    c.request(
        reqwest::Method::GET,
        &format!("/api/crf/items/{id}"),
        None::<&()>,
    )
    .await
}

pub async fn update(
    c: &HttpClient,
    id: i64,
    body: UpdateCrfItemRequest,
) -> Result<CrfItemViewResponse, ApiError> {
    c.request(
        reqwest::Method::PATCH,
        &format!("/api/crf/items/{id}"),
        Some(&body),
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

    fn item_view_json(id: i64, form_id: i64, code: &str) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "formId": form_id,
            "code": code,
            "name": code,
            "kind": "text",
            "order": 0,
            "notSubmitted": false,
            "createdAt": "2026-01-01T00:00:00Z",
            "updatedAt": "2026-01-02T00:00:00Z"
        })
    }

    #[tokio::test]
    async fn list_by_form_returns_items() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/crf/forms/11/items"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [item_view_json(21, 11, "AETERM")]
            })))
            .mount(&server)
            .await;
        let resp = list_by_form(&client(&server), 11).await.unwrap();
        assert_eq!(resp.items.len(), 1);
        assert_eq!(resp.items[0].code, "AETERM");
    }

    #[tokio::test]
    async fn get_by_id_returns_view() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/crf/items/21"))
            .respond_with(ResponseTemplate::new(200).set_body_json(item_view_json(21, 11, "AETERM")))
            .mount(&server)
            .await;
        let resp = get_by_id(&client(&server), 21).await.unwrap();
        assert_eq!(resp.id, 21);
        assert_eq!(
            resp.created_at,
            Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
        );
    }

    #[tokio::test]
    async fn update_returns_view() {
        let server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path("/api/crf/items/21"))
            .respond_with(ResponseTemplate::new(200).set_body_json(item_view_json(21, 11, "AETERMX")))
            .mount(&server)
            .await;
        let resp = update(
            &client(&server),
            21,
            UpdateCrfItemRequest {
                code: None,
                name: Some("AETERMX".into()),
                order: None,
                not_submitted: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(resp.code, "AETERMX");
    }

    #[test]
    fn update_request_skips_none_fields() {
        let body = UpdateCrfItemRequest {
            code: None,
            name: Some("renamed".into()),
            order: None,
            not_submitted: None,
        };
        let j = serde_json::to_string(&body).unwrap();
        assert_eq!(j, r#"{"name":"renamed"}"#);
    }
}
