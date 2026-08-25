//! Tauri command shims for the SDTM variable HTTP layer.

use tauri::State;

use crate::http::client::HttpClient;
use crate::http::domain_model::variable::{
    self, CreateSdtmVariableRequest, SdtmVariableListResponse, SdtmVariableViewResponse,
    UpdateSdtmVariableRequest,
};
use crate::http::dto::ApiError;

#[tauri::command]
pub async fn create_sdtm_variable(
    client: State<'_, HttpClient>,
    input: CreateSdtmVariableRequest,
) -> Result<SdtmVariableViewResponse, ApiError> {
    variable::create(&client, input).await
}

#[tauri::command]
pub async fn list_sdtm_variables_by_domain(
    client: State<'_, HttpClient>,
    domain_id: i64,
) -> Result<SdtmVariableListResponse, ApiError> {
    variable::list_by_domain(&client, domain_id).await
}

#[tauri::command]
pub async fn get_sdtm_variable_by_id(
    client: State<'_, HttpClient>,
    id: i64,
) -> Result<SdtmVariableViewResponse, ApiError> {
    variable::get_by_id(&client, id).await
}

#[tauri::command]
pub async fn update_sdtm_variable(
    client: State<'_, HttpClient>,
    id: i64,
    body: UpdateSdtmVariableRequest,
) -> Result<SdtmVariableViewResponse, ApiError> {
    variable::update(&client, id, body).await
}

#[tauri::command]
pub async fn delete_sdtm_variable(
    client: State<'_, HttpClient>,
    id: i64,
) -> Result<(), ApiError> {
    variable::delete(&client, id).await
}