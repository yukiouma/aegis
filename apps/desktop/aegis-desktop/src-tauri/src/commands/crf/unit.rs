//! Tauri command shims for `http::crf::unit`.

use tauri::State;
use trace_id::TraceIdGenerator;

use crate::http::client::{HttpClient, TRACE_ID};
use crate::http::crf::unit::{
    self, CrfUnitListResponse, CrfUnitViewResponse, UpdateCrfUnitRequest,
};
use crate::http::dto::ApiError;

#[tauri::command]
pub async fn update_crf_unit(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    id: i64,
    body: UpdateCrfUnitRequest,
) -> Result<CrfUnitViewResponse, ApiError> {
    let trace_id = generator.client_side();
    tracing::info!(trace_id = %trace_id, "enter");

    let result = TRACE_ID
        .scope(trace_id.clone(), async { unit::update(&client, id, body).await })
        .await;

    match &result {
        Ok(_) => tracing::info!(trace_id = %trace_id, "success"),
        Err(e) => tracing::error!(trace_id = %trace_id, error = %e, "failed"),
    }
    result
}

#[tauri::command]
pub async fn get_crf_unit_by_id(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    id: i64,
) -> Result<CrfUnitViewResponse, ApiError> {
    let trace_id = generator.client_side();
    tracing::info!(trace_id = %trace_id, "enter");

    let result = TRACE_ID
        .scope(trace_id.clone(), async { unit::get_by_id(&client, id).await })
        .await;

    match &result {
        Ok(_) => tracing::info!(trace_id = %trace_id, "success"),
        Err(e) => tracing::error!(trace_id = %trace_id, error = %e, "failed"),
    }
    result
}

#[tauri::command]
pub async fn search_crf_units_by_version(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    version_id: i64,
    fragment: String,
) -> Result<CrfUnitListResponse, ApiError> {
    let trace_id = generator.client_side();
    tracing::info!(trace_id = %trace_id, "enter");

    let result = TRACE_ID
        .scope(trace_id.clone(), async {
            unit::search_by_version(&client, version_id, fragment).await
        })
        .await;

    match &result {
        Ok(_) => tracing::info!(trace_id = %trace_id, "success"),
        Err(e) => tracing::error!(trace_id = %trace_id, error = %e, "failed"),
    }
    result
}
