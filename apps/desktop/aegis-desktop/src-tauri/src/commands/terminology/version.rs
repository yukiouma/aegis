//! Tauri command shims for the terminology version HTTP layer.

use tracing::Instrument;

use crate::http::client::TRACE_ID;
use crate::http::dto::{ApiError, TerminologyKind};
use crate::http::terminology::version::{
    self, CreateTerminologyVersionRequest, TerminologyVersionViewResponse,
    UpdateTerminologyVersionRequest,
};
use crate::state::{Caller, RequestContext, SharedAppState};

#[tauri::command]
pub async fn create_terminology_version(
    app_state: tauri::State<'_, SharedAppState>,
    kind: TerminologyKind,
    name: String,
) -> Result<TerminologyVersionViewResponse, ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    create_terminology_version_impl(app_state.inner(), &req_ctx, kind, name).await
}

pub async fn create_terminology_version_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    kind: TerminologyKind,
    name: String,
) -> Result<TerminologyVersionViewResponse, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "create_terminology_version"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                version::create(
                    &app_state.http_client(),
                    CreateTerminologyVersionRequest { kind, name },
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
pub async fn list_terminology_versions(
    app_state: tauri::State<'_, SharedAppState>,
) -> Result<Vec<TerminologyVersionViewResponse>, ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    list_terminology_versions_impl(app_state.inner(), &req_ctx).await
}

pub async fn list_terminology_versions_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
) -> Result<Vec<TerminologyVersionViewResponse>, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "list_terminology_versions"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                version::list(&app_state.http_client()).await
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
pub async fn get_terminology_version_by_id(
    app_state: tauri::State<'_, SharedAppState>,
    id: i64,
) -> Result<TerminologyVersionViewResponse, ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    get_terminology_version_by_id_impl(app_state.inner(), &req_ctx, id).await
}

pub async fn get_terminology_version_by_id_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    id: i64,
) -> Result<TerminologyVersionViewResponse, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "get_terminology_version_by_id"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                version::get_by_id(&app_state.http_client(), id).await
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
pub async fn update_terminology_version(
    app_state: tauri::State<'_, SharedAppState>,
    id: i64,
    body: UpdateTerminologyVersionRequest,
) -> Result<TerminologyVersionViewResponse, ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    update_terminology_version_impl(app_state.inner(), &req_ctx, id, body).await
}

pub async fn update_terminology_version_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    id: i64,
    body: UpdateTerminologyVersionRequest,
) -> Result<TerminologyVersionViewResponse, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "update_terminology_version"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                version::update(&app_state.http_client(), id, body).await
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
pub async fn delete_terminology_version(
    app_state: tauri::State<'_, SharedAppState>,
    id: i64,
) -> Result<(), ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    delete_terminology_version_impl(app_state.inner(), &req_ctx, id).await
}

pub async fn delete_terminology_version_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    id: i64,
) -> Result<(), ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "delete_terminology_version"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                version::delete(&app_state.http_client(), id).await
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
