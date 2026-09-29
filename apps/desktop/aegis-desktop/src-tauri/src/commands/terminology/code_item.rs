//! Tauri command shims for the terminology code-item HTTP layer.

use tauri::State;
use trace_id::TraceIdGenerator;

use crate::http::client::{HttpClient, TRACE_ID};
use crate::http::dto::ApiError;
use crate::http::terminology::code_item::{
    self, CodeItemListQuery, CodeItemListResponse, CodeItemPagedResponse, CodeItemViewResponse,
    CreateCodeItemRequest, UpdateCodeItemRequest,
};

#[tauri::command]
pub async fn create_code_item(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    codelist_id: i64,
    version_id: i64,
    code: String,
    submission_value: String,
    synonym: String,
    definition: String,
    nci_preferred_term: String,
) -> Result<CodeItemViewResponse, ApiError> {
    let trace_id = generator.client_side();
    tracing::info!(trace_id = %trace_id, "enter");

    let result = TRACE_ID
        .scope(trace_id.clone(), async {
            code_item::create(
                &client,
                CreateCodeItemRequest {
                    codelist_id,
                    version_id,
                    code,
                    submission_value,
                    synonym,
                    definition,
                    nci_preferred_term,
                },
            )
            .await
        })
        .await;

    match &result {
        Ok(_) => tracing::info!(trace_id = %trace_id, "success"),
        Err(e) => tracing::error!(trace_id = %trace_id, error = %e, "failed"),
    }
    result
}

#[tauri::command]
pub async fn list_code_items(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    codelist_id: Option<i64>,
    version_id: Option<i64>,
    fragment: Option<String>,
    offset: u32,
    limit: u32,
) -> Result<CodeItemPagedResponse, ApiError> {
    let trace_id = generator.client_side();
    tracing::info!(trace_id = %trace_id, "enter");

    let result = TRACE_ID
        .scope(trace_id.clone(), async {
            code_item::list_paged(
                &client,
                CodeItemListQuery {
                    codelist_id,
                    version_id,
                    fragment,
                    offset,
                    limit,
                },
            )
            .await
        })
        .await;

    match &result {
        Ok(_) => tracing::info!(trace_id = %trace_id, "success"),
        Err(e) => tracing::error!(trace_id = %trace_id, error = %e, "failed"),
    }
    result
}

#[tauri::command]
pub async fn update_code_item(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    id: i64,
    body: UpdateCodeItemRequest,
) -> Result<CodeItemViewResponse, ApiError> {
    let trace_id = generator.client_side();
    tracing::info!(trace_id = %trace_id, "enter");

    let result = TRACE_ID
        .scope(trace_id.clone(), async {
            code_item::update(&client, id, body).await
        })
        .await;

    match &result {
        Ok(_) => tracing::info!(trace_id = %trace_id, "success"),
        Err(e) => tracing::error!(trace_id = %trace_id, error = %e, "failed"),
    }
    result
}

#[tauri::command]
pub async fn delete_code_item(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    id: i64,
) -> Result<(), ApiError> {
    let trace_id = generator.client_side();
    tracing::info!(trace_id = %trace_id, "enter");

    let result = TRACE_ID
        .scope(trace_id.clone(), async { code_item::delete(&client, id).await })
        .await;

    match &result {
        Ok(_) => tracing::info!(trace_id = %trace_id, "success"),
        Err(e) => tracing::error!(trace_id = %trace_id, error = %e, "failed"),
    }
    result
}

#[tauri::command]
pub async fn list_code_items_by_version_and_code(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    version_id: i64,
    code: String,
) -> Result<CodeItemListResponse, ApiError> {
    let trace_id = generator.client_side();
    tracing::info!(trace_id = %trace_id, "enter");

    let result = TRACE_ID
        .scope(trace_id.clone(), async {
            code_item::list_by_version_and_code(&client, version_id, &code).await
        })
        .await;

    match &result {
        Ok(_) => tracing::info!(trace_id = %trace_id, "success"),
        Err(e) => tracing::error!(trace_id = %trace_id, error = %e, "failed"),
    }
    result
}
