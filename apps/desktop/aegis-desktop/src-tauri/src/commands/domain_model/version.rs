//! Tauri command shims for the SDTM domain-model version HTTP layer.

use tauri::State;
use trace_id::TraceIdGenerator;

use crate::http::client::{HttpClient, TRACE_ID};
use crate::http::domain_model::version::{
    self, CreateSdtmVersionRequest, SdtmVersionListResponse, SdtmVersionViewResponse,
    UpdateSdtmVersionRequest,
};
use crate::http::dto::ApiError;

#[tauri::command]
pub async fn create_sdtm_version(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    name: String,
) -> Result<SdtmVersionViewResponse, ApiError> {
    let trace_id = generator.client_side();
    tracing::info!(trace_id = %trace_id, "enter");

    let result = TRACE_ID
        .scope(trace_id.clone(), async {
            version::create(&client, CreateSdtmVersionRequest { name }).await
        })
        .await;

    match &result {
        Ok(_) => tracing::info!(trace_id = %trace_id, "success"),
        Err(e) => tracing::error!(trace_id = %trace_id, error = %e, "failed"),
    }
    result
}

#[tauri::command]
pub async fn list_sdtm_versions(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
) -> Result<SdtmVersionListResponse, ApiError> {
    let trace_id = generator.client_side();
    tracing::info!(trace_id = %trace_id, "enter");

    let result = TRACE_ID
        .scope(trace_id.clone(), async { version::list(&client).await })
        .await;

    match &result {
        Ok(_) => tracing::info!(trace_id = %trace_id, "success"),
        Err(e) => tracing::error!(trace_id = %trace_id, error = %e, "failed"),
    }
    result
}

#[tauri::command]
pub async fn get_sdtm_version_by_id(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    id: i64,
) -> Result<SdtmVersionViewResponse, ApiError> {
    let trace_id = generator.client_side();
    tracing::info!(trace_id = %trace_id, "enter");

    let result = TRACE_ID
        .scope(trace_id.clone(), async { version::get_by_id(&client, id).await })
        .await;

    match &result {
        Ok(_) => tracing::info!(trace_id = %trace_id, "success"),
        Err(e) => tracing::error!(trace_id = %trace_id, error = %e, "failed"),
    }
    result
}

#[tauri::command]
pub async fn update_sdtm_version(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    id: i64,
    body: UpdateSdtmVersionRequest,
) -> Result<SdtmVersionViewResponse, ApiError> {
    let trace_id = generator.client_side();
    tracing::info!(trace_id = %trace_id, "enter");

    let result = TRACE_ID
        .scope(trace_id.clone(), async { version::update(&client, id, body).await })
        .await;

    match &result {
        Ok(_) => tracing::info!(trace_id = %trace_id, "success"),
        Err(e) => tracing::error!(trace_id = %trace_id, error = %e, "failed"),
    }
    result
}

#[tauri::command]
pub async fn delete_sdtm_version(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    id: i64,
) -> Result<(), ApiError> {
    let trace_id = generator.client_side();
    tracing::info!(trace_id = %trace_id, "enter");

    let result = TRACE_ID
        .scope(trace_id.clone(), async { version::delete(&client, id).await })
        .await;

    match &result {
        Ok(_) => tracing::info!(trace_id = %trace_id, "success"),
        Err(e) => tracing::error!(trace_id = %trace_id, error = %e, "failed"),
    }
    result
}
