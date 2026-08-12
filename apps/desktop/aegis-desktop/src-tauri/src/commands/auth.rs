use tauri::State;

use crate::http::auth::{self, LoginRequest};
use crate::http::client::HttpClient;
use crate::http::dto::ApiError;

#[tauri::command]
pub async fn login(
    client: State<'_, HttpClient>,
    code: String,
    password: String,
) -> Result<(), ApiError> {
    auth::login(
        &client,
        LoginRequest { code, password },
    )
    .await
}

#[tauri::command]
pub async fn login_domain(
    client: State<'_, HttpClient>,
    code: String,
) -> Result<(), ApiError> {
    auth::login_domain(&client, &code).await
}

#[tauri::command]
pub async fn is_logged_in(
    client: State<'_, HttpClient>,
) -> Result<bool, ApiError> {
    Ok(client.tokens().access_token().await?.is_some())
}

#[tauri::command]
pub async fn refresh(client: State<'_, HttpClient>) -> Result<(), ApiError> {
    auth::refresh(&client).await
}

#[tauri::command]
pub async fn logout(client: State<'_, HttpClient>) -> Result<(), ApiError> {
    auth::logout(&client).await
}
