//! Tauri command shims for the SDTM domain HTTP layer.

use tracing::Instrument;

use crate::http::client::TRACE_ID;
use crate::http::domain_model::domain::{
    self, CreateSdtmDomainRequest, SdtmDomainListResponse, SdtmDomainViewResponse,
    UpdateSdtmDomainRequest,
};
use crate::http::dto::ApiError;
use crate::state::{Caller, RequestContext, SharedAppState};

#[tauri::command]
pub async fn create_sdtm_domain(
    app_state: tauri::State<'_, SharedAppState>,
    input: CreateSdtmDomainRequest,
) -> Result<SdtmDomainViewResponse, ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    create_sdtm_domain_impl(app_state.inner(), &req_ctx, input).await
}

pub async fn create_sdtm_domain_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    input: CreateSdtmDomainRequest,
) -> Result<SdtmDomainViewResponse, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "create_sdtm_domain"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                domain::create(&app_state.http_client(), input).await
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
pub async fn list_sdtm_domains_by_version(
    app_state: tauri::State<'_, SharedAppState>,
    version_id: i64,
) -> Result<SdtmDomainListResponse, ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    list_sdtm_domains_by_version_impl(app_state.inner(), &req_ctx, version_id).await
}

pub async fn list_sdtm_domains_by_version_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    version_id: i64,
) -> Result<SdtmDomainListResponse, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "list_sdtm_domains_by_version"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                domain::list_by_version(&app_state.http_client(), version_id).await
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
    app_state: tauri::State<'_, SharedAppState>,
    id: i64,
) -> Result<SdtmDomainViewResponse, ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    get_sdtm_domain_by_id_impl(app_state.inner(), &req_ctx, id).await
}

pub async fn get_sdtm_domain_by_id_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    id: i64,
) -> Result<SdtmDomainViewResponse, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "get_sdtm_domain_by_id"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                domain::get_by_id(&app_state.http_client(), id).await
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
pub async fn update_sdtm_domain(
    app_state: tauri::State<'_, SharedAppState>,
    id: i64,
    body: UpdateSdtmDomainRequest,
) -> Result<SdtmDomainViewResponse, ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    update_sdtm_domain_impl(app_state.inner(), &req_ctx, id, body).await
}

pub async fn update_sdtm_domain_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    id: i64,
    body: UpdateSdtmDomainRequest,
) -> Result<SdtmDomainViewResponse, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "update_sdtm_domain"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                domain::update(&app_state.http_client(), id, body).await
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
pub async fn delete_sdtm_domain(
    app_state: tauri::State<'_, SharedAppState>,
    id: i64,
) -> Result<(), ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    delete_sdtm_domain_impl(app_state.inner(), &req_ctx, id).await
}

pub async fn delete_sdtm_domain_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    id: i64,
) -> Result<(), ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "delete_sdtm_domain"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                domain::delete(&app_state.http_client(), id).await
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
