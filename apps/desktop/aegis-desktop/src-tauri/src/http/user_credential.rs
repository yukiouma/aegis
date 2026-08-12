//! User-credential management: register (admin/root) + self-service rotation.

use serde::{Deserialize, Serialize};

use super::client::HttpClient;
use super::dto::{ApiError, Role};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterUserRequest {
    pub user_code: String,
    pub user_name: String,
    pub domain_name: String,
    pub hostname: String,
    pub sid: String,
    pub password: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegisterUserResponse {
    pub user_code: String,
    pub user_name: String,
    pub role: Role,
    pub active: bool,
    pub domain_name: String,
    pub hostname: String,
    pub sid: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateUserCredentialRequest {
    pub user_code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserCredentialViewResponse {
    pub user_code: String,
    pub password_hash: String,
    pub token_version: u32,
}

pub async fn register(
    c: &HttpClient,
    body: RegisterUserRequest,
) -> Result<RegisterUserResponse, ApiError> {
    c.request(
        reqwest::Method::POST,
        "/api/auth/user-credential",
        Some(&body),
    )
    .await
}

pub async fn update(
    c: &HttpClient,
    body: UpdateUserCredentialRequest,
) -> Result<UserCredentialViewResponse, ApiError> {
    c.request(
        reqwest::Method::PATCH,
        "/api/auth/user-credential",
        Some(&body),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::super::client::{HttpClient, MemoryStore};

    #[tokio::test]
    async fn register_round_trips_role() {
        let server = MockServer::start().await;
        let store = Arc::new(MemoryStore::default());
        let m = Mock::given(method("POST"))
            .and(path("/api/auth/user-credential"))
            .and(body_json(serde_json::json!({
                "user_code": "u", "user_name": "n",
                "domain_name": "d", "hostname": "h", "sid": "s",
                "password": "p"
            })))
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
                "user_code": "u", "user_name": "n", "role": "general", "active": true,
                "domain_name": "d", "hostname": "h", "sid": "s"
            })));
        server.register(m).await;
        let c = HttpClient::new(server.uri(), store);
        let resp = register(
            &c,
            RegisterUserRequest {
                user_code: "u".into(),
                user_name: "n".into(),
                domain_name: "d".into(),
                hostname: "h".into(),
                sid: "s".into(),
                password: "p".into(),
            },
        )
        .await
        .unwrap();
        assert_eq!(resp.role, Role::General);
        assert!(resp.active);
    }

    #[test]
    fn update_with_no_password_skips_field_in_json() {
        let body = UpdateUserCredentialRequest {
            user_code: "u".into(),
            password: None,
        };
        let j = serde_json::to_string(&body).unwrap();
        assert_eq!(j, r#"{"user_code":"u"}"#);
    }
}
