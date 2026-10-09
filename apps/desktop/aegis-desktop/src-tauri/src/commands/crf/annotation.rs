//! Tauri command shims for `http::crf::annotation`.

use tracing::Instrument;

use crate::http::client::TRACE_ID;
use crate::http::crf::annotation::{
    self, AnnotationListResponse, CreateAnnotationRequest, UpdateAnnotationRequest,
};
use crate::http::crf::form::AnnotationViewResponse;
use crate::http::dto::ApiError;
use crate::state::{Caller, RequestContext, SharedAppState};

#[tauri::command]
pub async fn create_crf_annotation(
    app_state: tauri::State<'_, SharedAppState>,
    body: CreateAnnotationRequest,
) -> Result<AnnotationViewResponse, ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    create_crf_annotation_impl(app_state.inner(), &req_ctx, body).await
}

pub async fn create_crf_annotation_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    body: CreateAnnotationRequest,
) -> Result<AnnotationViewResponse, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "create_crf_annotation"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                annotation::create(&app_state.http_client(), body).await
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
pub async fn update_crf_annotation(
    app_state: tauri::State<'_, SharedAppState>,
    id: i64,
    body: UpdateAnnotationRequest,
) -> Result<AnnotationViewResponse, ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    update_crf_annotation_impl(app_state.inner(), &req_ctx, id, body).await
}

pub async fn update_crf_annotation_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    id: i64,
    body: UpdateAnnotationRequest,
) -> Result<AnnotationViewResponse, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "update_crf_annotation"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                annotation::update(&app_state.http_client(), id, body).await
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
pub async fn delete_crf_annotation(
    app_state: tauri::State<'_, SharedAppState>,
    id: i64,
) -> Result<(), ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    delete_crf_annotation_impl(app_state.inner(), &req_ctx, id).await
}

pub async fn delete_crf_annotation_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    id: i64,
) -> Result<(), ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "delete_crf_annotation"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                annotation::delete(&app_state.http_client(), id).await
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
pub async fn search_crf_annotations_by_version(
    app_state: tauri::State<'_, SharedAppState>,
    version_id: i64,
    fragment: String,
) -> Result<AnnotationListResponse, ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    search_crf_annotations_by_version_impl(app_state.inner(), &req_ctx, version_id, fragment).await
}

pub async fn search_crf_annotations_by_version_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    version_id: i64,
    fragment: String,
) -> Result<AnnotationListResponse, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "search_crf_annotations_by_version"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                annotation::search_by_version(&app_state.http_client(), version_id, fragment).await
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
