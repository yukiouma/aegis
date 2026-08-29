//! Tauri command shims for `http::crf::unit`.

use tauri::State;

use crate::http::client::HttpClient;
use crate::http::crf::unit::{self, CrfUnitViewResponse, UpdateCrfUnitRequest};
use crate::http::dto::ApiError;

#[tauri::command]
pub async fn update_crf_unit(
    client: State<'_, HttpClient>,
    id: i64,
    body: UpdateCrfUnitRequest,
) -> Result<CrfUnitViewResponse, ApiError> {
    unit::update(&client, id, body).await
}