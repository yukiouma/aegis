//! Tauri command shims for the terminology version HTTP layer.

use tauri::State;

use crate::http::client::HttpClient;
use crate::http::dto::{ApiError, TerminologyKind};
use crate::http::terminology::version::{
    self, CreateTerminologyVersionRequest, TerminologyVersionViewResponse,
    UpdateTerminologyVersionRequest,
};

#[tauri::command]
pub async fn create_terminology_version(
    client: State<'_, HttpClient>,
    kind: TerminologyKind,
    name: String,
) -> Result<TerminologyVersionViewResponse, ApiError> {
    version::create(
        &client,
        CreateTerminologyVersionRequest { kind, name },
    )
    .await
}

#[tauri::command]
pub async fn list_terminology_versions(
    client: State<'_, HttpClient>,
) -> Result<Vec<TerminologyVersionViewResponse>, ApiError> {
    version::list(&client).await
}

#[tauri::command]
pub async fn get_terminology_version_by_id(
    client: State<'_, HttpClient>,
    id: i64,
) -> Result<TerminologyVersionViewResponse, ApiError> {
    version::get_by_id(&client, id).await
}

#[tauri::command]
pub async fn update_terminology_version(
    client: State<'_, HttpClient>,
    id: i64,
    body: UpdateTerminologyVersionRequest,
) -> Result<TerminologyVersionViewResponse, ApiError> {
    version::update(&client, id, body).await
}

#[tauri::command]
pub async fn delete_terminology_version(
    client: State<'_, HttpClient>,
    id: i64,
) -> Result<(), ApiError> {
    version::delete(&client, id).await
}