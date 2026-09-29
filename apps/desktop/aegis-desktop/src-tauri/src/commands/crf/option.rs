//! Tauri command shims for `http::crf::option`.

use tauri::State;
use trace_id::TraceIdGenerator;

use crate::http::client::{HttpClient, TRACE_ID};
use crate::http::crf::option::{
    self, CrfOptionListResponse, CrfOptionViewResponse, UpdateCrfOptionRequest,
};
use crate::http::dto::ApiError;

#[tauri::command]
pub async fn update_crf_option(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    id: i64,
    body: UpdateCrfOptionRequest,
) -> Result<CrfOptionViewResponse, ApiError> {
    let trace_id = generator.client_side();
    tracing::info!(trace_id = %trace_id, "enter");

    let result = TRACE_ID
        .scope(trace_id.clone(), async { option::update(&client, id, body).await })
        .await;

    match &result {
        Ok(_) => tracing::info!(trace_id = %trace_id, "success"),
        Err(e) => tracing::error!(trace_id = %trace_id, error = %e, "failed"),
    }
    result
}

#[tauri::command]
pub async fn get_crf_option_by_id(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    id: i64,
) -> Result<CrfOptionViewResponse, ApiError> {
    let trace_id = generator.client_side();
    tracing::info!(trace_id = %trace_id, "enter");

    let result = TRACE_ID
        .scope(trace_id.clone(), async { option::get_by_id(&client, id).await })
        .await;

    match &result {
        Ok(_) => tracing::info!(trace_id = %trace_id, "success"),
        Err(e) => tracing::error!(trace_id = %trace_id, error = %e, "failed"),
    }
    result
}

#[tauri::command]
pub async fn search_crf_options_by_version(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    version_id: i64,
    fragment: String,
) -> Result<CrfOptionListResponse, ApiError> {
    let trace_id = generator.client_side();
    tracing::info!(trace_id = %trace_id, "enter");

    let result = TRACE_ID
        .scope(trace_id.clone(), async {
            option::search_by_version(&client, version_id, fragment).await
        })
        .await;

    match &result {
        Ok(_) => tracing::info!(trace_id = %trace_id, "success"),
        Err(e) => tracing::error!(trace_id = %trace_id, error = %e, "failed"),
    }
    result
}
