//! HTTP functions under `/api/crf/units/{id}`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::http::client::HttpClient;
use crate::http::dto::ApiError;

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

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCrfUnitRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_submitted: Option<bool>,
}

pub async fn update(
    c: &HttpClient,
    id: i64,
    body: UpdateCrfUnitRequest,
) -> Result<CrfUnitViewResponse, ApiError> {
    c.request(
        reqwest::Method::PATCH,
        &format!("/api/crf/units/{id}"),
        Some(&body),
    )
    .await
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrfUnitListResponse {
    pub units: Vec<CrfUnitViewResponse>,
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
) -> Result<CrfUnitListResponse, ApiError> {
    let encoded = percent_encode_fragment(&fragment);
    c.request(
        reqwest::Method::GET,
        &format!("/api/crf/versions/{version_id}/units/search?fragment={encoded}"),
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

    fn unit_view_json(id: i64, item_id: i64, value: &str) -> serde_json::Value {
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
            .and(path("/api/crf/units/41"))
            .respond_with(ResponseTemplate::new(200).set_body_json(unit_view_json(41, 21, "kg")))
            .mount(&server)
            .await;
        let resp = update(
            &client(&server),
            41,
            UpdateCrfUnitRequest {
                value: Some("kg".into()),
                not_submitted: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(resp.value, "kg");
        assert_eq!(
            resp.created_at,
            Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
        );
    }

    #[test]
    fn update_request_skips_none_fields() {
        let body = UpdateCrfUnitRequest {
            value: None,
            not_submitted: Some(true),
        };
        let j = serde_json::to_string(&body).unwrap();
        assert_eq!(j, r#"{"notSubmitted":true}"#);
    }

    #[tokio::test]
    async fn search_by_version_with_fragment_includes_query_param() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/crf/versions/7/units/search"))
            .and(query_param("fragment", "mg"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "units": [unit_view_json(41, 21, "mg")]
            })))
            .mount(&server)
            .await;
        let resp = search_by_version(&client(&server), 7, "mg".into())
            .await
            .unwrap();
        assert_eq!(resp.units.len(), 1);
        assert_eq!(resp.units[0].value, "mg");
    }
}
