//! Tauri command shims for the SDTM variable HTTP layer.

use tauri::State;
use trace_id::TraceIdGenerator;
use tracing::Instrument;

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
    let span = tracing::info_span!(
        "command",
        trace_id = %trace_id,
        command = "create_sdtm_variable"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(trace_id, async { variable::create(&client, input).await })
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
pub async fn list_sdtm_variables_by_domain(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    domain_id: i64,
) -> Result<SdtmVariableListResponse, ApiError> {
    let trace_id = generator.client_side();
    let span = tracing::info_span!(
        "command",
        trace_id = %trace_id,
        command = "list_sdtm_variables_by_domain"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(trace_id, async {
                variable::list_by_domain(&client, domain_id).await
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
pub async fn get_sdtm_variable_by_id(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    id: i64,
) -> Result<SdtmVariableViewResponse, ApiError> {
    let trace_id = generator.client_side();
    let span = tracing::info_span!(
        "command",
        trace_id = %trace_id,
        command = "get_sdtm_variable_by_id"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(trace_id, async { variable::get_by_id(&client, id).await })
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
pub async fn update_sdtm_variable(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    id: i64,
    body: UpdateSdtmVariableRequest,
) -> Result<SdtmVariableViewResponse, ApiError> {
    let trace_id = generator.client_side();
    let span = tracing::info_span!(
        "command",
        trace_id = %trace_id,
        command = "update_sdtm_variable"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(trace_id, async {
                variable::update(&client, id, body).await
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
pub async fn delete_sdtm_variable(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    id: i64,
) -> Result<(), ApiError> {
    let trace_id = generator.client_side();
    let span = tracing::info_span!(
        "command",
        trace_id = %trace_id,
        command = "delete_sdtm_variable"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(trace_id, async { variable::delete(&client, id).await })
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
