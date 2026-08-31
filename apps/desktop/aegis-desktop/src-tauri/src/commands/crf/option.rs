//! Tauri command shims for `http::crf::option`.

use tauri::State;

use crate::http::client::HttpClient;
use crate::http::crf::option::{
    self, CrfOptionListResponse, CrfOptionViewResponse, UpdateCrfOptionRequest,
};
use crate::http::dto::ApiError;

#[tauri::command]
pub async fn update_crf_option(
    client: State<'_, HttpClient>,
    id: i64,
    body: UpdateCrfOptionRequest,
) -> Result<CrfOptionViewResponse, ApiError> {
    option::update(&client, id, body).await
}

#[tauri::command]
pub async fn get_crf_option_by_id(
    client: State<'_, HttpClient>,
    id: i64,
) -> Result<CrfOptionViewResponse, ApiError> {
    option::get_by_id(&client, id).await
}

#[tauri::command]
pub async fn search_crf_options_by_version(
    client: State<'_, HttpClient>,
    version_id: i64,
    fragment: String,
) -> Result<CrfOptionListResponse, ApiError> {
    option::search_by_version(&client, version_id, fragment).await
}
