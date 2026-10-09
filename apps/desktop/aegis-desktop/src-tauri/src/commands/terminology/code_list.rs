//! Tauri command shims for the terminology code-list HTTP layer.

use tracing::Instrument;

use crate::http::client::TRACE_ID;
use crate::http::dto::ApiError;
use crate::http::terminology::code_list::{
    self, CodeListListQuery, CodeListPagedResponse, CodeListViewResponse, CreateCodeListRequest,
    UpdateCodeListRequest,
};
use crate::state::{Caller, RequestContext, SharedAppState};

#[tauri::command]
pub async fn create_code_list(
    app_state: tauri::State<'_, SharedAppState>,
    version_id: i64,
    code: String,
    extensible: bool,
    name: String,
    submission_value: String,
    synonym: String,
    definition: String,
    nci_preferred_term: String,
) -> Result<CodeListViewResponse, ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    create_code_list_impl(
        app_state.inner(),
        &req_ctx,
        version_id,
        code,
        extensible,
        name,
        submission_value,
        synonym,
        definition,
        nci_preferred_term,
    )
    .await
}

pub async fn create_code_list_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    version_id: i64,
    code: String,
    extensible: bool,
    name: String,
    submission_value: String,
    synonym: String,
    definition: String,
    nci_preferred_term: String,
) -> Result<CodeListViewResponse, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "create_code_list"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                code_list::create(
                    &app_state.http_client(),
                    CreateCodeListRequest {
                        version_id,
                        code,
                        extensible,
                        name,
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
pub async fn list_code_lists(
    app_state: tauri::State<'_, SharedAppState>,
    version_id: i64,
    fragment: Option<String>,
    offset: u32,
    limit: u32,
) -> Result<CodeListPagedResponse, ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    list_code_lists_impl(
        app_state.inner(),
        &req_ctx,
        version_id,
        fragment,
        offset,
        limit,
    )
    .await
}

pub async fn list_code_lists_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    version_id: i64,
    fragment: Option<String>,
    offset: u32,
    limit: u32,
) -> Result<CodeListPagedResponse, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "list_code_lists"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                code_list::list_paged(
                    &app_state.http_client(),
                    CodeListListQuery {
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
pub async fn get_code_list_by_id(
    app_state: tauri::State<'_, SharedAppState>,
    id: i64,
) -> Result<CodeListViewResponse, ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    get_code_list_by_id_impl(app_state.inner(), &req_ctx, id).await
}

pub async fn get_code_list_by_id_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    id: i64,
) -> Result<CodeListViewResponse, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "get_code_list_by_id"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                code_list::get_by_id(&app_state.http_client(), id).await
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
pub async fn update_code_list(
    app_state: tauri::State<'_, SharedAppState>,
    id: i64,
    body: UpdateCodeListRequest,
) -> Result<CodeListViewResponse, ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    update_code_list_impl(app_state.inner(), &req_ctx, id, body).await
}

pub async fn update_code_list_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    id: i64,
    body: UpdateCodeListRequest,
) -> Result<CodeListViewResponse, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "update_code_list"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                code_list::update(&app_state.http_client(), id, body).await
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
pub async fn delete_code_list(
    app_state: tauri::State<'_, SharedAppState>,
    id: i64,
) -> Result<(), ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    delete_code_list_impl(app_state.inner(), &req_ctx, id).await
}

pub async fn delete_code_list_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    id: i64,
) -> Result<(), ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "delete_code_list"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                code_list::delete(&app_state.http_client(), id).await
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
