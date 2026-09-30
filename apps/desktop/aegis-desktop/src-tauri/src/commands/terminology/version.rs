//! Tauri command shims for the terminology version HTTP layer.

use logging_utils::TraceIdGenerator;
use tauri::State;
use tracing::Instrument;

use crate::http::client::{HttpClient, TRACE_ID};
use crate::http::dto::{ApiError, TerminologyKind};
use crate::http::terminology::version::{
    self, CreateTerminologyVersionRequest, TerminologyVersionViewResponse,
    UpdateTerminologyVersionRequest,
};

#[tauri::command]
pub async fn create_terminology_version(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    kind: TerminologyKind,
    name: String,
) -> Result<TerminologyVersionViewResponse, ApiError> {
    let trace_id = generator.client_side();
    let span = tracing::info_span!(
        "command",
        trace_id = %trace_id,
        command = "create_terminology_version"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(trace_id, async {
                version::create(&client, CreateTerminologyVersionRequest { kind, name }).await
            })
            .await;
        match &result {
            Ok(_) => tracing::info!("success"),
            Err(e) => tracing::error!(error = %e, "failed"),
        }
        result
    }
    .instrument(span)
    .await
}

#[tauri::command]
pub async fn list_terminology_versions(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
) -> Result<Vec<TerminologyVersionViewResponse>, ApiError> {
    let trace_id = generator.client_side();
    let span = tracing::info_span!(
        "command",
        trace_id = %trace_id,
        command = "list_terminology_versions"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(trace_id, async { version::list(&client).await })
            .await;
        match &result {
            Ok(_) => tracing::info!("success"),
            Err(e) => tracing::error!(error = %e, "failed"),
        }
        result
    }
    .instrument(span)
    .await
}

#[tauri::command]
pub async fn get_terminology_version_by_id(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    id: i64,
) -> Result<TerminologyVersionViewResponse, ApiError> {
    let trace_id = generator.client_side();
    let span = tracing::info_span!(
        "command",
        trace_id = %trace_id,
        command = "get_terminology_version_by_id"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(trace_id, async { version::get_by_id(&client, id).await })
            .await;
        match &result {
            Ok(_) => tracing::info!("success"),
            Err(e) => tracing::error!(error = %e, "failed"),
        }
        result
    }
    .instrument(span)
    .await
}

#[tauri::command]
pub async fn update_terminology_version(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    id: i64,
    body: UpdateTerminologyVersionRequest,
) -> Result<TerminologyVersionViewResponse, ApiError> {
    let trace_id = generator.client_side();
    let span = tracing::info_span!(
        "command",
        trace_id = %trace_id,
        command = "update_terminology_version"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(trace_id, async { version::update(&client, id, body).await })
            .await;
        match &result {
            Ok(_) => tracing::info!("success"),
            Err(e) => tracing::error!(error = %e, "failed"),
        }
        result
    }
    .instrument(span)
    .await
}

#[tauri::command]
pub async fn delete_terminology_version(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    id: i64,
) -> Result<(), ApiError> {
    let trace_id = generator.client_side();
    let span = tracing::info_span!(
        "command",
        trace_id = %trace_id,
        command = "delete_terminology_version"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(trace_id, async { version::delete(&client, id).await })
            .await;
        match &result {
            Ok(_) => tracing::info!("success"),
            Err(e) => tracing::error!(error = %e, "failed"),
        }
        result
    }
    .instrument(span)
    .await
}
