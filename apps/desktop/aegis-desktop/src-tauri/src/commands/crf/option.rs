//! Tauri command shims for `http::crf::option`.

use tauri::State;

use crate::http::client::HttpClient;
use crate::http::crf::option::{self, CrfOptionViewResponse, UpdateCrfOptionRequest};
use crate::http::dto::ApiError;

#[tauri::command]
pub async fn update_crf_option(
    client: State<'_, HttpClient>,
    id: i64,
    body: UpdateCrfOptionRequest,
) -> Result<CrfOptionViewResponse, ApiError> {
    option::update(&client, id, body).await
}