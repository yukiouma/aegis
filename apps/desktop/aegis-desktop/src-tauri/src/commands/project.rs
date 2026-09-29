use tauri::State;
use trace_id::TraceIdGenerator;

use crate::http::client::{HttpClient, TRACE_ID};
use crate::http::dto::ApiError;
use crate::http::project::{
    self, CreateProjectRequest, ProjectConfigurationDataRequest, ProjectMemberDataRequest,
    ProjectViewResponse, UpdateProjectRequest,
};

#[tauri::command]
pub async fn create_project(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    code: String,
    description: String,
    configurations: Option<ProjectConfigurationDataRequest>,
    members: Option<ProjectMemberDataRequest>,
    unblind_members: Option<ProjectMemberDataRequest>,
) -> Result<ProjectViewResponse, ApiError> {
    let trace_id = generator.client_side();
    tracing::info!(trace_id = %trace_id, "enter");

    let result = TRACE_ID
        .scope(trace_id.clone(), async {
            project::create(
                &client,
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
        Ok(_) => tracing::info!(trace_id = %trace_id, "success"),
        Err(e) => tracing::error!(trace_id = %trace_id, error = %e, "failed"),
    }
    result
}

#[tauri::command]
pub async fn list_projects(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
) -> Result<Vec<ProjectViewResponse>, ApiError> {
    let trace_id = generator.client_side();
    tracing::info!(trace_id = %trace_id, "enter");

    let result = TRACE_ID
        .scope(trace_id.clone(), async { project::list(&client).await })
        .await;

    match &result {
        Ok(_) => tracing::info!(trace_id = %trace_id, "success"),
        Err(e) => tracing::error!(trace_id = %trace_id, error = %e, "failed"),
    }
    result
}

#[tauri::command]
pub async fn get_project_by_code(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    code: String,
) -> Result<ProjectViewResponse, ApiError> {
    let trace_id = generator.client_side();
    tracing::info!(trace_id = %trace_id, "enter");

    let result = TRACE_ID
        .scope(trace_id.clone(), async {
            project::get_by_code(&client, &code).await
        })
        .await;

    match &result {
        Ok(_) => tracing::info!(trace_id = %trace_id, "success"),
        Err(e) => tracing::error!(trace_id = %trace_id, error = %e, "failed"),
    }
    result
}

#[tauri::command]
pub async fn update_project(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    code: String,
    body: UpdateProjectRequest,
) -> Result<ProjectViewResponse, ApiError> {
    let trace_id = generator.client_side();
    tracing::info!(trace_id = %trace_id, "enter");

    let result = TRACE_ID
        .scope(trace_id.clone(), async {
            project::update(&client, &code, body).await
        })
        .await;

    match &result {
        Ok(_) => tracing::info!(trace_id = %trace_id, "success"),
        Err(e) => tracing::error!(trace_id = %trace_id, error = %e, "failed"),
    }
    result
}
