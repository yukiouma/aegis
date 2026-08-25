//! Tauri command shims for the SDTM domain HTTP layer.

use tauri::State;

use crate::http::client::HttpClient;
use crate::http::dto::ApiError;
use crate::http::domain_model::domain::{
    self, CreateSdtmDomainRequest, SdtmDomainViewResponse, UpdateSdtmDomainRequest,
};

#[tauri::command]
pub async fn create_sdtm_domain(
    client: State<'_, HttpClient>,
    input: CreateSdtmDomainRequest,
) -> Result<SdtmDomainViewResponse, ApiError> {
    domain::create(&client, input).await
}

#[tauri::command]
pub async fn list_sdtm_domains_by_version(
    client: State<'_, HttpClient>,
    version_id: i64,
) -> Result<Vec<SdtmDomainViewResponse>, ApiError> {
    domain::list_by_version(&client, version_id).await
}

#[tauri::command]
pub async fn get_sdtm_domain_by_id(
    client: State<'_, HttpClient>,
    id: i64,
) -> Result<SdtmDomainViewResponse, ApiError> {
    domain::get_by_id(&client, id).await
}

#[tauri::command]
pub async fn update_sdtm_domain(
    client: State<'_, HttpClient>,
    id: i64,
    body: UpdateSdtmDomainRequest,
) -> Result<SdtmDomainViewResponse, ApiError> {
    domain::update(&client, id, body).await
}

#[tauri::command]
pub async fn delete_sdtm_domain(
    client: State<'_, HttpClient>,
    id: i64,
) -> Result<(), ApiError> {
    domain::delete(&client, id).await
}