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

#[tauri::command]
pub async fn update_user(
    client: State<'_, HttpClient>,
    code: String,
    body: UpdateUserRequest,
) -> Result<UserViewResponse, ApiError> {
    user::update(&client, &code, body).await
}
