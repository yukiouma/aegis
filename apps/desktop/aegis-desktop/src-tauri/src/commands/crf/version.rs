//! Tauri command shim for `http::crf::version::list_by_project`.

use tauri::State;

use crate::http::client::HttpClient;
use crate::http::crf::version::{
    self, CrfVersionListResponse, CrfVersionViewResponse, EdcType,
};
use crate::http::dto::ApiError;

#[tauri::command]
pub async fn list_crf_versions(
    client: State<'_, HttpClient>,
    project_code: String,
) -> Result<CrfVersionListResponse, ApiError> {
    version::list_by_project(&client, &project_code).await
}

#[tauri::command]
pub async fn import_als(
    client: State<'_, HttpClient>,
    name: String,
    project_code: String,
    filepath: String,
    edc_type: EdcType,
) -> Result<CrfVersionViewResponse, ApiError> {
    version::import_als(&client, &project_code, &name, &filepath, edc_type).await
}