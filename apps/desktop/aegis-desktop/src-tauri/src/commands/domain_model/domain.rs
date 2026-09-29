//! Tauri command shims for the SDTM domain HTTP layer.

use tauri::State;
use trace_id::TraceIdGenerator;
use tracing::Instrument;

use crate::http::client::{HttpClient, TRACE_ID};
use crate::http::domain_model::domain::{
    self, CreateSdtmDomainRequest, SdtmDomainListResponse, SdtmDomainViewResponse,
    UpdateSdtmDomainRequest,
};
use crate::http::dto::ApiError;

#[tauri::command]
pub async fn create_sdtm_domain(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    input: CreateSdtmDomainRequest,
) -> Result<SdtmDomainViewResponse, ApiError> {
    let trace_id = generator.client_side();
    let span = tracing::info_span!(
        "command",
        trace_id = %trace_id,
        command = "create_sdtm_domain"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(trace_id, async { domain::create(&client, input).await })
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
pub async fn list_sdtm_domains_by_version(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    version_id: i64,
) -> Result<SdtmDomainListResponse, ApiError> {
    let trace_id = generator.client_side();
    let span = tracing::info_span!(
        "command",
        trace_id = %trace_id,
        command = "list_sdtm_domains_by_version"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(trace_id, async {
                domain::list_by_version(&client, version_id).await
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
pub async fn get_sdtm_domain_by_id(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    id: i64,
) -> Result<SdtmDomainViewResponse, ApiError> {
    let trace_id = generator.client_side();
    let span = tracing::info_span!(
        "command",
        trace_id = %trace_id,
        command = "get_sdtm_domain_by_id"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(trace_id, async { domain::get_by_id(&client, id).await })
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
pub async fn update_sdtm_domain(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    id: i64,
    body: UpdateSdtmDomainRequest,
) -> Result<SdtmDomainViewResponse, ApiError> {
    let trace_id = generator.client_side();
    let span = tracing::info_span!(
        "command",
        trace_id = %trace_id,
        command = "update_sdtm_domain"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(trace_id, async { domain::update(&client, id, body).await })
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
pub async fn delete_sdtm_domain(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    id: i64,
) -> Result<(), ApiError> {
    let trace_id = generator.client_side();
    let span = tracing::info_span!(
        "command",
        trace_id = %trace_id,
        command = "delete_sdtm_domain"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(trace_id, async { domain::delete(&client, id).await })
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