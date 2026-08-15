//! User CRUD.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::client::HttpClient;
use super::dto::{ApiError, Role};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserViewResponse {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub role: Role,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserListResponse {
    pub users: Vec<UserViewResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserRequest {
    pub code: String,
    pub name: String,
    pub role: Role,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<Role>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
}

pub async fn create(c: &HttpClient, body: CreateUserRequest) -> Result<UserViewResponse, ApiError> {
    c.request(reqwest::Method::POST, "/api/user", Some(&body)).await
}

pub async fn list(c: &HttpClient) -> Result<Vec<UserViewResponse>, ApiError> {
    let resp: UserListResponse = c
        .request(reqwest::Method::GET, "/api/user", None::<&()>)
        .await?;
    Ok(resp.users)
}

pub async fn get_by_code(c: &HttpClient, code: &str) -> Result<UserViewResponse, ApiError> {
    c.request(reqwest::Method::GET, &format!("/api/user/{code}"), None::<&()>)
        .await
}

pub async fn update(
    c: &HttpClient,
    code: &str,
    body: UpdateUserRequest,
) -> Result<UserViewResponse, ApiError> {
    c.request(
        reqwest::Method::PATCH,
        &format!("/api/user/{code}"),
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

    use super::super::client::{HttpClient, MemoryStore, TokenStore};

    #[tokio::test]
    async fn list_returns_users() {
        let server = MockServer::start().await;
        let store = Arc::new(MemoryStore::default());
        store.set_access_token("AT").await.unwrap();
        store.set_refresh_token("RT").await.unwrap();
        server
            .register(
                Mock::given(method("GET"))
                    .and(path("/api/user"))
                    .respond_with(ResponseTemplate::new(200).set_body_json(
                        serde_json::json!({
                            "users": [{
                                "id": 1, "code": "a", "name": "Alice",
                                "role": "admin", "active": true,
                                "createdAt": "2026-01-01T00:00:00Z",
                                "updatedAt": "2026-01-02T00:00:00Z"
                            }]
                        }),
                    )),
            )
            .await;
        let c = HttpClient::new(server.uri(), store);
        let users = list(&c).await.unwrap();
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].code, "a");
        assert_eq!(
            users[0].created_at,
            Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
        );
    }

    #[test]
    fn update_request_skips_none_fields() {
        let body = UpdateUserRequest {
            name: Some("Alice".into()),
            ..Default::default()
        };
        let j = serde_json::to_string(&body).unwrap();
        assert_eq!(j, r#"{"name":"Alice"}"#);
    }
}
