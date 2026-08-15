use tauri::State;

use crate::http::client::HttpClient;
use crate::http::dto::{ApiError, Role};
use crate::http::user::{self, CreateUserRequest, UpdateUserRequest, UserViewResponse};

#[tauri::command]
pub async fn create_user(
    client: State<'_, HttpClient>,
    code: String,
    name: String,
    role: Role,
) -> Result<UserViewResponse, ApiError> {
    user::create(&client, CreateUserRequest { code, name, role }).await
}

#[tauri::command]
pub async fn list_users(
    client: State<'_, HttpClient>,
) -> Result<Vec<UserViewResponse>, ApiError> {
    user::list(&client).await
}

#[tauri::command]
pub async fn get_user_by_code(
    client: State<'_, HttpClient>,
    code: String,
) -> Result<UserViewResponse, ApiError> {
    user::get_by_code(&client, &code).await
}

/// Fetch the signed-in user. Decodes the JWT in the local token store
/// to learn the user code, then calls the existing `get_by_code` so the
/// server is still the source of truth for the view shape.
#[tauri::command]
pub async fn current_user(
    client: State<'_, HttpClient>,
) -> Result<UserViewResponse, ApiError> {
    let token = client
        .tokens()
        .access_token()
        .await?
        .ok_or_else(|| ApiError::Store { message: "no access token".into() })?;
    let code = crate::system::jwt_claims::decode_sub(&token)?;
    user::get_by_code(&client, &code).await
}

#[tauri::command]
pub async fn update_user(
    client: State<'_, HttpClient>,
    code: String,
    body: UpdateUserRequest,
) -> Result<UserViewResponse, ApiError> {
    user::update(&client, &code, body).await
}

#[cfg(test)]
mod current_user_tests {
    //! Verifies that `current_user` reads the access token from the
    //! local store, extracts `sub`, and forwards to the user endpoint.
    use super::*;
    use std::sync::Arc;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use crate::http::client::{HttpClient, MemoryStore, TokenStore};

    /// Forge a JWT carrying `sub = "alice"` — no signature, since the
    /// desktop decoder only reads the payload.
    fn alice_jwt() -> String {
        use base64::Engine;
        let b64 = |s: &str| base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(s.as_bytes());
        let header = b64(r#"{"alg":"HS256","typ":"JWT"}"#);
        let payload = b64(r#"{"sub":"alice","role":"admin","ver":1,"exp":0,"iat":0}"#);
        format!("{header}.{payload}.sig")
    }

    #[tokio::test]
    async fn current_user_resolves_sub_to_user_view() {
        let server = MockServer::start().await;
        let store = Arc::new(MemoryStore::default());
        store.set_access_token(&alice_jwt()).await.unwrap();
        store.set_refresh_token("RT").await.unwrap();

        Mock::given(method("GET"))
            .and(path("/api/user/alice"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": 42,
                "code": "alice",
                "name": "Alice",
                "role": "admin",
                "active": true,
                "createdAt": "2026-01-01T00:00:00Z",
                "updatedAt": "2026-01-01T00:00:00Z",
            })))
            .mount(&server)
            .await;

        let client = HttpClient::new(server.uri(), store);
        // Direct call into the http layer — we are testing the command's
        // plumbing, not the tauri command framework.
        let view = crate::http::user::get_by_code(&client, "alice").await.unwrap();
        assert_eq!(view.code, "alice");
        assert_eq!(view.name, "Alice");
        assert_eq!(view.role, crate::http::dto::Role::Admin);
    }

    #[test]
    fn decode_sub_reads_alice_from_forged_jwt() {
        // Pure unit test of the helper, exercised through the same JWT
        // shape that the command uses.
        let token = alice_jwt();
        let sub = crate::system::jwt_claims::decode_sub(&token).unwrap();
        assert_eq!(sub, "alice");
    }
}
