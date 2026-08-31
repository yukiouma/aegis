//! Tauri command shims for `http::crf::unit`.

use tauri::State;

use crate::http::client::HttpClient;
use crate::http::crf::unit::{
    self, CrfUnitListResponse, CrfUnitViewResponse, UpdateCrfUnitRequest,
};
use crate::http::dto::ApiError;

#[tauri::command]
pub async fn update_crf_unit(
    client: State<'_, HttpClient>,
    id: i64,
    body: UpdateCrfUnitRequest,
) -> Result<CrfUnitViewResponse, ApiError> {
    unit::update(&client, id, body).await
}

#[tauri::command]
pub async fn get_crf_unit_by_id(
    client: State<'_, HttpClient>,
    id: i64,
) -> Result<CrfUnitViewResponse, ApiError> {
    unit::get_by_id(&client, id).await
}

#[tauri::command]
pub async fn search_crf_units_by_version(
    client: State<'_, HttpClient>,
    version_id: i64,
    fragment: String,
) -> Result<CrfUnitListResponse, ApiError> {
    unit::search_by_version(&client, version_id, fragment).await
}
