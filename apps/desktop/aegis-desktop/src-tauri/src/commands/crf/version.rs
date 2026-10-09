//! Tauri command shim for `http::crf::version::list_by_project`.

use tracing::Instrument;

use crate::http::client::TRACE_ID;
use crate::http::crf::version::{self, CrfVersionListResponse, CrfVersionViewResponse, EdcType};
use crate::http::dto::ApiError;
use crate::state::{Caller, RequestContext, SharedAppState};

#[tauri::command]
pub async fn list_crf_versions(
    app_state: tauri::State<'_, SharedAppState>,
    project_code: String,
) -> Result<CrfVersionListResponse, ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    list_crf_versions_impl(app_state.inner(), &req_ctx, project_code).await
}

pub async fn list_crf_versions_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    project_code: String,
) -> Result<CrfVersionListResponse, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "list_crf_versions"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                version::list_by_project(&app_state.http_client(), &project_code).await
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
pub async fn import_als(
    app_state: tauri::State<'_, SharedAppState>,
    name: String,
    project_code: String,
    filepath: String,
    edc_type: EdcType,
) -> Result<CrfVersionViewResponse, ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    import_als_impl(
        app_state.inner(),
        &req_ctx,
        name,
        project_code,
        filepath,
        edc_type,
    )
    .await
}

pub async fn import_als_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    name: String,
    project_code: String,
    filepath: String,
    edc_type: EdcType,
) -> Result<CrfVersionViewResponse, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "import_als"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                version::import_als(
                    &app_state.http_client(),
                    &project_code,
                    &name,
                    &filepath,
                    edc_type,
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
