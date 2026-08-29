//! HTTP functions under `/api/crf/options/{id}`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::http::client::HttpClient;
use crate::http::dto::ApiError;

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

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCrfOptionRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_submitted: Option<bool>,
}

pub async fn update(
    c: &HttpClient,
    id: i64,
    body: UpdateCrfOptionRequest,
) -> Result<CrfOptionViewResponse, ApiError> {
    c.request(
        reqwest::Method::PATCH,
        &format!("/api/crf/options/{id}"),
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

    fn option_view_json(id: i64, item_id: i64, value: &str) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "itemId": item_id,
            "value": value,
            "notSubmitted": false,
            "createdAt": "2026-01-01T00:00:00Z",
            "updatedAt": "2026-01-02T00:00:00Z"
        })
    }

    #[tokio::test]
    async fn update_returns_view() {
        let server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path("/api/crf/options/31"))
            .respond_with(ResponseTemplate::new(200).set_body_json(option_view_json(31, 21, "NO")))
            .mount(&server)
            .await;
        let resp = update(
            &client(&server),
            31,
            UpdateCrfOptionRequest {
                value: Some("NO".into()),
                not_submitted: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(resp.value, "NO");
        assert_eq!(
            resp.created_at,
            Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
        );
    }

    #[test]
    fn update_request_skips_none_fields() {
        let body = UpdateCrfOptionRequest {
            value: None,
            not_submitted: Some(true),
        };
        let j = serde_json::to_string(&body).unwrap();
        assert_eq!(j, r#"{"notSubmitted":true}"#);
    }
}