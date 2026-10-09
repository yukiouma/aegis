//! Tauri command shims for the SDTM variable HTTP layer.

use tracing::Instrument;

use crate::http::client::TRACE_ID;
use crate::http::domain_model::variable::{
    self, CreateSdtmVariableRequest, SdtmVariableListResponse, SdtmVariableViewResponse,
    UpdateSdtmVariableRequest,
};
use crate::http::dto::ApiError;
use crate::state::{Caller, RequestContext, SharedAppState};

#[tauri::command]
pub async fn create_sdtm_variable(
    app_state: tauri::State<'_, SharedAppState>,
    input: CreateSdtmVariableRequest,
) -> Result<SdtmVariableViewResponse, ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    create_sdtm_variable_impl(app_state.inner(), &req_ctx, input).await
}

pub async fn create_sdtm_variable_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    input: CreateSdtmVariableRequest,
) -> Result<SdtmVariableViewResponse, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "create_sdtm_variable"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                variable::create(&app_state.http_client(), input).await
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
pub async fn list_sdtm_variables_by_domain(
    app_state: tauri::State<'_, SharedAppState>,
    domain_id: i64,
) -> Result<SdtmVariableListResponse, ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    list_sdtm_variables_by_domain_impl(app_state.inner(), &req_ctx, domain_id).await
}

pub async fn list_sdtm_variables_by_domain_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    domain_id: i64,
) -> Result<SdtmVariableListResponse, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "list_sdtm_variables_by_domain"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                variable::list_by_domain(&app_state.http_client(), domain_id).await
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
pub async fn get_sdtm_variable_by_id(
    app_state: tauri::State<'_, SharedAppState>,
    id: i64,
) -> Result<SdtmVariableViewResponse, ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    get_sdtm_variable_by_id_impl(app_state.inner(), &req_ctx, id).await
}

pub async fn get_sdtm_variable_by_id_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    id: i64,
) -> Result<SdtmVariableViewResponse, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "get_sdtm_variable_by_id"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                variable::get_by_id(&app_state.http_client(), id).await
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
pub async fn update_sdtm_variable(
    app_state: tauri::State<'_, SharedAppState>,
    id: i64,
    body: UpdateSdtmVariableRequest,
) -> Result<SdtmVariableViewResponse, ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    update_sdtm_variable_impl(app_state.inner(), &req_ctx, id, body).await
}

pub async fn update_sdtm_variable_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    id: i64,
    body: UpdateSdtmVariableRequest,
) -> Result<SdtmVariableViewResponse, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "update_sdtm_variable"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                variable::update(&app_state.http_client(), id, body).await
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
pub async fn delete_sdtm_variable(
    app_state: tauri::State<'_, SharedAppState>,
    id: i64,
) -> Result<(), ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    delete_sdtm_variable_impl(app_state.inner(), &req_ctx, id).await
}

pub async fn delete_sdtm_variable_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    id: i64,
) -> Result<(), ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "delete_sdtm_variable"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                variable::delete(&app_state.http_client(), id).await
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
