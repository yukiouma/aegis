//! Tauri command shims for the SDTM variable HTTP layer.

use tauri::State;
use trace_id::TraceIdGenerator;

use crate::http::client::{HttpClient, TRACE_ID};
use crate::http::domain_model::variable::{
    self, CreateSdtmVariableRequest, SdtmVariableListResponse, SdtmVariableViewResponse,
    UpdateSdtmVariableRequest,
};
use crate::http::dto::ApiError;

#[tauri::command]
pub async fn create_sdtm_variable(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    input: CreateSdtmVariableRequest,
) -> Result<SdtmVariableViewResponse, ApiError> {
    let trace_id = generator.client_side();
    tracing::info!(trace_id = %trace_id, "enter");

    let result = TRACE_ID
        .scope(trace_id.clone(), async { variable::create(&client, input).await })
        .await;

    match &result {
        Ok(_) => tracing::info!(trace_id = %trace_id, "success"),
        Err(e) => tracing::error!(trace_id = %trace_id, error = %e, "failed"),
    }
    result
}

#[tauri::command]
pub async fn list_sdtm_variables_by_domain(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    domain_id: i64,
) -> Result<SdtmVariableListResponse, ApiError> {
    let trace_id = generator.client_side();
    tracing::info!(trace_id = %trace_id, "enter");

    let result = TRACE_ID
        .scope(trace_id.clone(), async {
            variable::list_by_domain(&client, domain_id).await
        })
        .await;

    match &result {
        Ok(_) => tracing::info!(trace_id = %trace_id, "success"),
        Err(e) => tracing::error!(trace_id = %trace_id, error = %e, "failed"),
    }
    result
}

#[tauri::command]
pub async fn get_sdtm_variable_by_id(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    id: i64,
) -> Result<SdtmVariableViewResponse, ApiError> {
    let trace_id = generator.client_side();
    tracing::info!(trace_id = %trace_id, "enter");

    let result = TRACE_ID
        .scope(trace_id.clone(), async { variable::get_by_id(&client, id).await })
        .await;

    match &result {
        Ok(_) => tracing::info!(trace_id = %trace_id, "success"),
        Err(e) => tracing::error!(trace_id = %trace_id, error = %e, "failed"),
    }
    result
}

#[tauri::command]
pub async fn update_sdtm_variable(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    id: i64,
    body: UpdateSdtmVariableRequest,
) -> Result<SdtmVariableViewResponse, ApiError> {
    let trace_id = generator.client_side();
    tracing::info!(trace_id = %trace_id, "enter");

    let result = TRACE_ID
        .scope(trace_id.clone(), async {
            variable::update(&client, id, body).await
        })
        .await;

    match &result {
        Ok(_) => tracing::info!(trace_id = %trace_id, "success"),
        Err(e) => tracing::error!(trace_id = %trace_id, error = %e, "failed"),
    }
    result
}

#[tauri::command]
pub async fn delete_sdtm_variable(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    id: i64,
) -> Result<(), ApiError> {
    let trace_id = generator.client_side();
    tracing::info!(trace_id = %trace_id, "enter");

    let result = TRACE_ID
        .scope(trace_id.clone(), async { variable::delete(&client, id).await })
        .await;

    match &result {
        Ok(_) => tracing::info!(trace_id = %trace_id, "success"),
        Err(e) => tracing::error!(trace_id = %trace_id, error = %e, "failed"),
    }
    result
}
