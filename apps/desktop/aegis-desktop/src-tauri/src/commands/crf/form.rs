//! Tauri command shims for `http::crf::form`.

use tauri::State;

use crate::http::client::HttpClient;
use crate::http::crf::form::{
    self, CreateCrfFormRequest, CrfFormListResponse, CrfFormViewResponse,
    UpdateCrfFormRequest,
};
use crate::http::dto::ApiError;

#[tauri::command]
pub async fn list_crf_forms_by_version(
    client: State<'_, HttpClient>,
    version_id: i64,
) -> Result<CrfFormListResponse, ApiError> {
    form::list_by_version(&client, version_id).await
}

#[tauri::command]
pub async fn create_crf_form(
    client: State<'_, HttpClient>,
    version_id: i64,
    body: CreateCrfFormRequest,
) -> Result<CrfFormViewResponse, ApiError> {
    form::create(&client, version_id, body).await
}

#[tauri::command]
pub async fn update_crf_form(
    client: State<'_, HttpClient>,
    id: i64,
    body: UpdateCrfFormRequest,
) -> Result<CrfFormViewResponse, ApiError> {
    form::update(&client, id, body).await
}

#[tauri::command]
pub async fn delete_crf_form(client: State<'_, HttpClient>, id: i64) -> Result<(), ApiError> {
    form::delete(&client, id).await
}

#[tauri::command]
pub async fn get_crf_form_by_id(
    client: State<'_, HttpClient>,
    id: i64,
) -> Result<CrfFormViewResponse, ApiError> {
    form::get_by_id(&client, id).await
}