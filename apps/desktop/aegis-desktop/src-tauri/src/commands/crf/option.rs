//! Tauri command shims for `http::crf::option`.

use tracing::Instrument;

use crate::http::client::TRACE_ID;
use crate::http::crf::option::{
    self, CrfOptionListResponse, CrfOptionViewResponse, UpdateCrfOptionRequest,
};
use crate::http::dto::ApiError;
use crate::state::{Caller, RequestContext, SharedAppState};

#[tauri::command]
pub async fn update_crf_option(
    app_state: tauri::State<'_, SharedAppState>,
    id: i64,
    body: UpdateCrfOptionRequest,
) -> Result<CrfOptionViewResponse, ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    update_crf_option_impl(app_state.inner(), &req_ctx, id, body).await
}

pub async fn update_crf_option_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    id: i64,
    body: UpdateCrfOptionRequest,
) -> Result<CrfOptionViewResponse, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "update_crf_option"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                option::update(&app_state.http_client(), id, body).await
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
pub async fn get_crf_option_by_id(
    app_state: tauri::State<'_, SharedAppState>,
    id: i64,
) -> Result<CrfOptionViewResponse, ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    get_crf_option_by_id_impl(app_state.inner(), &req_ctx, id).await
}

pub async fn get_crf_option_by_id_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    id: i64,
) -> Result<CrfOptionViewResponse, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "get_crf_option_by_id"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                option::get_by_id(&app_state.http_client(), id).await
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
pub async fn search_crf_options_by_version(
    app_state: tauri::State<'_, SharedAppState>,
    version_id: i64,
    fragment: String,
) -> Result<CrfOptionListResponse, ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    search_crf_options_by_version_impl(app_state.inner(), &req_ctx, version_id, fragment).await
}

pub async fn search_crf_options_by_version_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    version_id: i64,
    fragment: String,
) -> Result<CrfOptionListResponse, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "search_crf_options_by_version"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                option::search_by_version(&app_state.http_client(), version_id, fragment).await
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
