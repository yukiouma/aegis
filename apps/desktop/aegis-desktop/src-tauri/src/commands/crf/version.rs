//! Tauri command shim for `http::crf::version::list_by_project`.

use tauri::State;
use trace_id::TraceIdGenerator;

use crate::http::client::{HttpClient, TRACE_ID};
use crate::http::crf::version::{self, CrfVersionListResponse, CrfVersionViewResponse, EdcType};
use crate::http::dto::ApiError;

#[tauri::command]
pub async fn list_crf_versions(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    project_code: String,
) -> Result<CrfVersionListResponse, ApiError> {
    let trace_id = generator.client_side();
    let span = tracing::info_span!("command", trace_id = %trace_id, command = "list_crf_versions");
    let _enter = span.enter();
    tracing::info!("enter");

    let result = TRACE_ID
        .scope(trace_id, async {
            version::list_by_project(&client, &project_code).await
        })
        .await;

    match &result {
        Ok(_) => tracing::info!("success"),
        Err(e) => tracing::error!(error = %e, "failed"),
    }
    result
}

#[tauri::command]
pub async fn import_als(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    name: String,
    project_code: String,
    filepath: String,
    edc_type: EdcType,
) -> Result<CrfVersionViewResponse, ApiError> {
    let trace_id = generator.client_side();
    let span = tracing::info_span!("command", trace_id = %trace_id, command = "import_als");
    let _enter = span.enter();
    tracing::info!("enter");

    let result = TRACE_ID
        .scope(trace_id, async {
            version::import_als(&client, &project_code, &name, &filepath, edc_type).await
        })
        .await;

    match &result {
        Ok(_) => tracing::info!("success"),
        Err(e) => tracing::error!(error = %e, "failed"),
    }
    result
}