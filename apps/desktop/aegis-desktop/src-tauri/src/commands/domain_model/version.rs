//! Tauri command shims for the SDTM domain-model version HTTP layer.

use tauri::State;

use crate::http::client::HttpClient;
use crate::http::domain_model::version::{
    self, CreateSdtmVersionRequest, SdtmVersionListResponse, SdtmVersionViewResponse,
    UpdateSdtmVersionRequest,
};
use crate::http::dto::ApiError;

#[tauri::command]
pub async fn create_sdtm_version(
    client: State<'_, HttpClient>,
    name: String,
) -> Result<SdtmVersionViewResponse, ApiError> {
    version::create(&client, CreateSdtmVersionRequest { name }).await
}

#[tauri::command]
pub async fn list_sdtm_versions(
    client: State<'_, HttpClient>,
) -> Result<SdtmVersionListResponse, ApiError> {
    version::list(&client).await
}

#[tauri::command]
pub async fn get_sdtm_version_by_id(
    client: State<'_, HttpClient>,
    id: i64,
) -> Result<SdtmVersionViewResponse, ApiError> {
    version::get_by_id(&client, id).await
}

#[tauri::command]
pub async fn update_sdtm_version(
    client: State<'_, HttpClient>,
    id: i64,
    body: UpdateSdtmVersionRequest,
) -> Result<SdtmVersionViewResponse, ApiError> {
    version::update(&client, id, body).await
}

#[tauri::command]
pub async fn delete_sdtm_version(client: State<'_, HttpClient>, id: i64) -> Result<(), ApiError> {
    version::delete(&client, id).await
}
