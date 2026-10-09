//! Tauri command shims for the terminology code-item HTTP layer.

use tracing::Instrument;

use crate::http::client::TRACE_ID;
use crate::http::dto::ApiError;
use crate::http::terminology::code_item::{
    self, CodeItemListQuery, CodeItemListResponse, CodeItemPagedResponse, CodeItemViewResponse,
    CreateCodeItemRequest, UpdateCodeItemRequest,
};
use crate::state::{Caller, RequestContext, SharedAppState};

#[tauri::command]
pub async fn create_code_item(
    app_state: tauri::State<'_, SharedAppState>,
    codelist_id: i64,
    version_id: i64,
    code: String,
    submission_value: String,
    synonym: String,
    definition: String,
    nci_preferred_term: String,
) -> Result<CodeItemViewResponse, ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    create_code_item_impl(
        app_state.inner(),
        &req_ctx,
        codelist_id,
        version_id,
        code,
        submission_value,
        synonym,
        definition,
        nci_preferred_term,
    )
    .await
}

pub async fn create_code_item_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    codelist_id: i64,
    version_id: i64,
    code: String,
    submission_value: String,
    synonym: String,
    definition: String,
    nci_preferred_term: String,
) -> Result<CodeItemViewResponse, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "create_code_item"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                code_item::create(
                    &app_state.http_client(),
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
            Ok(_) => tracing::info!("success"),
            Err(e) => tracing::error!(error = %e, "failed"),
        }
        result
    }
    .instrument(span)
    .await
}

#[tauri::command]
pub async fn list_code_items(
    app_state: tauri::State<'_, SharedAppState>,
    codelist_id: Option<i64>,
    version_id: Option<i64>,
    fragment: Option<String>,
    offset: u32,
    limit: u32,
) -> Result<CodeItemPagedResponse, ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    list_code_items_impl(
        app_state.inner(),
        &req_ctx,
        codelist_id,
        version_id,
        fragment,
        offset,
        limit,
    )
    .await
}

pub async fn list_code_items_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    codelist_id: Option<i64>,
    version_id: Option<i64>,
    fragment: Option<String>,
    offset: u32,
    limit: u32,
) -> Result<CodeItemPagedResponse, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "list_code_items"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                code_item::list_paged(
                    &app_state.http_client(),
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
            Ok(_) => tracing::info!("success"),
            Err(e) => tracing::error!(error = %e, "failed"),
        }
        result
    }
    .instrument(span)
    .await
}

#[tauri::command]
pub async fn update_code_item(
    app_state: tauri::State<'_, SharedAppState>,
    id: i64,
    body: UpdateCodeItemRequest,
) -> Result<CodeItemViewResponse, ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    update_code_item_impl(app_state.inner(), &req_ctx, id, body).await
}

pub async fn update_code_item_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    id: i64,
    body: UpdateCodeItemRequest,
) -> Result<CodeItemViewResponse, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "update_code_item"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                code_item::update(&app_state.http_client(), id, body).await
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
pub async fn delete_code_item(
    app_state: tauri::State<'_, SharedAppState>,
    id: i64,
) -> Result<(), ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    delete_code_item_impl(app_state.inner(), &req_ctx, id).await
}

pub async fn delete_code_item_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    id: i64,
) -> Result<(), ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "delete_code_item"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                code_item::delete(&app_state.http_client(), id).await
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
pub async fn list_code_items_by_version_and_code(
    app_state: tauri::State<'_, SharedAppState>,
    version_id: i64,
    code: String,
) -> Result<CodeItemListResponse, ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    list_code_items_by_version_and_code_impl(app_state.inner(), &req_ctx, version_id, code).await
}

pub async fn list_code_items_by_version_and_code_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    version_id: i64,
    code: String,
) -> Result<CodeItemListResponse, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "list_code_items_by_version_and_code"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                code_item::list_by_version_and_code(&app_state.http_client(), version_id, &code).await
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
