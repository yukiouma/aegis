//! Tauri command shims for `http::crf::domain_annotation`.

use tracing::Instrument;

use crate::http::client::TRACE_ID;
use crate::http::crf::domain_annotation::{
    self, CreateDomainAnnotationRequest, UpdateDomainAnnotationRequest,
};
use crate::http::crf::form::DomainAnnotationViewResponse;
use crate::http::dto::ApiError;
use crate::state::{Caller, RequestContext, SharedAppState};

#[tauri::command]
pub async fn create_crf_domain_annotation(
    app_state: tauri::State<'_, SharedAppState>,
    form_id: i64,
    body: CreateDomainAnnotationRequest,
) -> Result<DomainAnnotationViewResponse, ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    create_crf_domain_annotation_impl(app_state.inner(), &req_ctx, form_id, body).await
}

pub async fn create_crf_domain_annotation_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    form_id: i64,
    body: CreateDomainAnnotationRequest,
) -> Result<DomainAnnotationViewResponse, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "create_crf_domain_annotation"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                domain_annotation::create(&app_state.http_client(), form_id, body).await
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
pub async fn list_crf_domain_annotations_by_form(
    app_state: tauri::State<'_, SharedAppState>,
    form_id: i64,
) -> Result<domain_annotation::DomainAnnotationListResponse, ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    list_crf_domain_annotations_by_form_impl(app_state.inner(), &req_ctx, form_id).await
}

pub async fn list_crf_domain_annotations_by_form_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    form_id: i64,
) -> Result<domain_annotation::DomainAnnotationListResponse, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "list_crf_domain_annotations_by_form"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                domain_annotation::list_by_form(&app_state.http_client(), form_id).await
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
pub async fn update_crf_domain_annotation(
    app_state: tauri::State<'_, SharedAppState>,
    id: i64,
    body: UpdateDomainAnnotationRequest,
) -> Result<DomainAnnotationViewResponse, ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    update_crf_domain_annotation_impl(app_state.inner(), &req_ctx, id, body).await
}

pub async fn update_crf_domain_annotation_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    id: i64,
    body: UpdateDomainAnnotationRequest,
) -> Result<DomainAnnotationViewResponse, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "update_crf_domain_annotation"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                domain_annotation::update(&app_state.http_client(), id, body).await
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
pub async fn delete_crf_domain_annotation(
    app_state: tauri::State<'_, SharedAppState>,
    id: i64,
) -> Result<(), ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    delete_crf_domain_annotation_impl(app_state.inner(), &req_ctx, id).await
}

pub async fn delete_crf_domain_annotation_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    id: i64,
) -> Result<(), ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "delete_crf_domain_annotation"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                domain_annotation::delete(&app_state.http_client(), id).await
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
pub async fn search_crf_domain_annotations_by_version(
    app_state: tauri::State<'_, SharedAppState>,
    version_id: i64,
    fragment: String,
) -> Result<domain_annotation::DomainAnnotationListResponse, ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    search_crf_domain_annotations_by_version_impl(app_state.inner(), &req_ctx, version_id, fragment)
        .await
}

pub async fn search_crf_domain_annotations_by_version_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    version_id: i64,
    fragment: String,
) -> Result<domain_annotation::DomainAnnotationListResponse, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "search_crf_domain_annotations_by_version"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                domain_annotation::search_by_version(&app_state.http_client(), version_id, fragment)
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
