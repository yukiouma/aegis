use tauri::State;

use crate::http::client::HttpClient;
use crate::http::dto::ApiError;
use crate::http::user_credential::{
    self, RegisterUserRequest, RegisterUserResponse, UpdateUserCredentialRequest,
    UserCredentialViewResponse,
};

#[tauri::command]
pub async fn register_user(
    client: State<'_, HttpClient>,
    user_code: String,
    user_name: String,
    domain_name: String,
    hostname: String,
    sid: String,
    password: String,
) -> Result<RegisterUserResponse, ApiError> {
    user_credential::register(
        &client,
        RegisterUserRequest {
            user_code,
            user_name,
            domain_name,
            hostname,
            sid,
            password,
        },
    )
    .await
}

#[tauri::command]
pub async fn update_user_credential(
    client: State<'_, HttpClient>,
    user_code: String,
    password: Option<String>,
) -> Result<UserCredentialViewResponse, ApiError> {
    user_credential::update(
        &client,
        UpdateUserCredentialRequest { user_code, password },
    )
    .await
}
