//! Tauri command shims for the SDTM domain-model version HTTP layer.

use tracing::Instrument;

use crate::http::client::TRACE_ID;
use crate::http::domain_model::version::{
    self, CreateSdtmVersionRequest, SdtmVersionListResponse, SdtmVersionViewResponse,
    UpdateSdtmVersionRequest,
};
use crate::http::dto::ApiError;
use crate::state::{Caller, RequestContext, SharedAppState};

#[tauri::command]
pub async fn create_sdtm_version(
    app_state: tauri::State<'_, SharedAppState>,
    name: String,
) -> Result<SdtmVersionViewResponse, ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    create_sdtm_version_impl(app_state.inner(), &req_ctx, name).await
}

pub async fn create_sdtm_version_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    name: String,
) -> Result<SdtmVersionViewResponse, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "create_sdtm_version"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                version::create(&app_state.http_client(), CreateSdtmVersionRequest { name }).await
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
pub async fn list_sdtm_versions(
    app_state: tauri::State<'_, SharedAppState>,
) -> Result<SdtmVersionListResponse, ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    list_sdtm_versions_impl(app_state.inner(), &req_ctx).await
}

pub async fn list_sdtm_versions_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
) -> Result<SdtmVersionListResponse, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "list_sdtm_versions"
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
pub async fn get_sdtm_version_by_id(
    app_state: tauri::State<'_, SharedAppState>,
    id: i64,
) -> Result<SdtmVersionViewResponse, ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    get_sdtm_version_by_id_impl(app_state.inner(), &req_ctx, id).await
}

pub async fn get_sdtm_version_by_id_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    id: i64,
) -> Result<SdtmVersionViewResponse, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "get_sdtm_version_by_id"
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
pub async fn update_sdtm_version(
    app_state: tauri::State<'_, SharedAppState>,
    id: i64,
    body: UpdateSdtmVersionRequest,
) -> Result<SdtmVersionViewResponse, ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    update_sdtm_version_impl(app_state.inner(), &req_ctx, id, body).await
}

pub async fn update_sdtm_version_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    id: i64,
    body: UpdateSdtmVersionRequest,
) -> Result<SdtmVersionViewResponse, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "update_sdtm_version"
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
pub async fn delete_sdtm_version(
    app_state: tauri::State<'_, SharedAppState>,
    id: i64,
) -> Result<(), ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    delete_sdtm_version_impl(app_state.inner(), &req_ctx, id).await
}

pub async fn delete_sdtm_version_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    id: i64,
) -> Result<(), ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "delete_sdtm_version"
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
