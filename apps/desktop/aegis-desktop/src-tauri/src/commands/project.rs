use tauri::State;

use crate::http::client::HttpClient;
use crate::http::dto::ApiError;
use crate::http::project::{
    self, CreateProjectRequest, ProjectMemberDataRequest, ProjectViewResponse,
    UpdateProjectRequest,
};

#[tauri::command]
pub async fn create_project(
    client: State<'_, HttpClient>,
    code: String,
    description: String,
    product_id: i32,
    members: Option<ProjectMemberDataRequest>,
    unblind_members: Option<ProjectMemberDataRequest>,
) -> Result<ProjectViewResponse, ApiError> {
    project::create(
        &client,
        CreateProjectRequest {
            code,
            description,
            product_id,
            members,
            unblind_members,
        },
    )
    .await
}

#[tauri::command]
pub async fn list_projects(
    client: State<'_, HttpClient>,
) -> Result<Vec<ProjectViewResponse>, ApiError> {
    project::list(&client).await
}

#[tauri::command]
pub async fn get_project_by_code(
    client: State<'_, HttpClient>,
    code: String,
) -> Result<ProjectViewResponse, ApiError> {
    project::get_by_code(&client, &code).await
}

#[tauri::command]
pub async fn update_project(
    client: State<'_, HttpClient>,
    code: String,
    body: UpdateProjectRequest,
) -> Result<ProjectViewResponse, ApiError> {
    project::update(&client, &code, body).await
}
