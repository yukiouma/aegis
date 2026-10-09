use tracing::Instrument;

use crate::http::client::TRACE_ID;
use crate::http::dto::ApiError;
use crate::http::project::{
    self, CreateProjectRequest, ProjectConfigurationDataRequest, ProjectMemberDataRequest,
    ProjectViewResponse, UpdateProjectRequest,
};
use crate::state::{Caller, RequestContext, SharedAppState};

#[tauri::command]
pub async fn create_project(
    app_state: tauri::State<'_, SharedAppState>,
    code: String,
    description: String,
    configurations: Option<ProjectConfigurationDataRequest>,
    members: Option<ProjectMemberDataRequest>,
    unblind_members: Option<ProjectMemberDataRequest>,
) -> Result<ProjectViewResponse, ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    create_project_impl(
        app_state.inner(),
        &req_ctx,
        code,
        description,
        configurations,
        members,
        unblind_members,
    )
    .await
}

pub async fn create_project_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    code: String,
    description: String,
    configurations: Option<ProjectConfigurationDataRequest>,
    members: Option<ProjectMemberDataRequest>,
    unblind_members: Option<ProjectMemberDataRequest>,
) -> Result<ProjectViewResponse, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "create_project"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                project::create(
                    &app_state.http_client(),
                    CreateProjectRequest {
                        code,
                        description,
                        configurations,
                        members,
                        unblind_members,
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
pub async fn list_projects(
    app_state: tauri::State<'_, SharedAppState>,
) -> Result<Vec<ProjectViewResponse>, ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    list_projects_impl(app_state.inner(), &req_ctx).await
}

pub async fn list_projects_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
) -> Result<Vec<ProjectViewResponse>, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "list_projects"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                project::list(&app_state.http_client()).await
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
pub async fn get_project_by_code(
    app_state: tauri::State<'_, SharedAppState>,
    code: String,
) -> Result<ProjectViewResponse, ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    get_project_by_code_impl(app_state.inner(), &req_ctx, code).await
}

pub async fn get_project_by_code_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    code: String,
) -> Result<ProjectViewResponse, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "get_project_by_code"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                project::get_by_code(&app_state.http_client(), &code).await
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
pub async fn update_project(
    app_state: tauri::State<'_, SharedAppState>,
    code: String,
    body: UpdateProjectRequest,
) -> Result<ProjectViewResponse, ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    update_project_impl(app_state.inner(), &req_ctx, code, body).await
}

pub async fn update_project_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    code: String,
    body: UpdateProjectRequest,
) -> Result<ProjectViewResponse, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "update_project"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                project::update(&app_state.http_client(), &code, body).await
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
