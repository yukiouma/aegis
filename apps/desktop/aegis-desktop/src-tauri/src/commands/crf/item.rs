//! Tauri command shims for `http::crf::item`.

use logging_utils::TraceIdGenerator;
use tauri::State;
use tracing::Instrument;

use crate::http::client::{HttpClient, TRACE_ID};
use crate::http::crf::item::{
    self, CrfItemListResponse, CrfItemViewResponse, UpdateCrfItemRequest,
};
use crate::http::dto::ApiError;

#[tauri::command]
pub async fn list_crf_items_by_form(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    form_id: i64,
) -> Result<CrfItemListResponse, ApiError> {
    let trace_id = generator.client_side();
    let span = tracing::info_span!(
        "command",
        trace_id = %trace_id,
        command = "list_crf_items_by_form"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(trace_id, async {
                item::list_by_form(&client, form_id).await
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
pub async fn get_crf_item_by_id(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    id: i64,
) -> Result<CrfItemViewResponse, ApiError> {
    let trace_id = generator.client_side();
    let span = tracing::info_span!(
        "command",
        trace_id = %trace_id,
        command = "get_crf_item_by_id"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(trace_id, async { item::get_by_id(&client, id).await })
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
pub async fn update_crf_item(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    id: i64,
    body: UpdateCrfItemRequest,
) -> Result<CrfItemViewResponse, ApiError> {
    let trace_id = generator.client_side();
    let span = tracing::info_span!(
        "command",
        trace_id = %trace_id,
        command = "update_crf_item"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(trace_id, async { item::update(&client, id, body).await })
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
pub async fn search_crf_items_by_version(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    version_id: i64,
    fragment: String,
) -> Result<CrfItemListResponse, ApiError> {
    let trace_id = generator.client_side();
    let span = tracing::info_span!(
        "command",
        trace_id = %trace_id,
        command = "search_crf_items_by_version"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(trace_id, async {
                item::search_by_version(&client, version_id, fragment).await
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
