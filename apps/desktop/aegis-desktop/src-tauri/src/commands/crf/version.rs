//! Tauri command shim for `http::crf::version::list_by_project`.

use tauri::State;

use crate::http::client::HttpClient;
use crate::http::crf::version::{self, CrfVersionViewResponse};
use crate::http::dto::ApiError;

#[tauri::command]
pub async fn list_crf_versions(
    client: State<'_, HttpClient>,
    project_code: String,
) -> Result<Vec<CrfVersionViewResponse>, ApiError> {
    version::list_by_project(&client, &project_code).await
}