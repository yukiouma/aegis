use tauri::State;

use crate::http::client::HttpClient;
use crate::http::dto::ApiError;
use crate::http::mission::{
    self, AssigneeDataArg, AssigneeViewResponse, CreateMissionRequest, MissionViewResponse,
};

// Tauri command argument shapes. The frontend calls these via
// `invoke<...>("command_name", args)`; serde decodes the JSON.
// `kind` / `mission_kind` / `role` come in as plain strings and are
// re-parsed via serde's JSON deserialize so the call sites get
// validation errors back as `ApiError::Parse`.

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListMissionsByProjectArgs {
    pub project_code: String,
    pub kind: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddAssigneeArgs {
    pub mission_id: i64,
    pub user_code: String,
    pub role: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveAssigneeArgs {
    pub mission_id: i64,
    pub assignee_id: i64,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMissionAssigneeArg {
    pub user_code: String,
    pub role: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMissionArgs {
    pub project_code: String,
    pub mission_kind: String,
    pub mission_code: String,
    pub assignees: Vec<CreateMissionAssigneeArg>,
}

fn parse_kind(s: &str) -> Result<crate::http::mission::MissionKind, ApiError> {
    serde_json::from_value(serde_json::Value::String(s.to_string()))
        .map_err(|e| ApiError::Parse { message: e.to_string() })
}

fn parse_role(s: &str) -> Result<crate::http::mission::MissionRole, ApiError> {
    serde_json::from_value(serde_json::Value::String(s.to_string()))
        .map_err(|e| ApiError::Parse { message: e.to_string() })
}

#[tauri::command]
pub async fn list_missions_by_project(
    client: State<'_, HttpClient>,
    args: ListMissionsByProjectArgs,
) -> Result<Vec<MissionViewResponse>, ApiError> {
    let kind = match args.kind.as_deref() {
        Some(s) => Some(parse_kind(s)?),
        None => None,
    };
    mission::list_by_project(&client, &args.project_code, kind).await
}

#[tauri::command]
pub async fn add_assignee(
    client: State<'_, HttpClient>,
    args: AddAssigneeArgs,
) -> Result<AssigneeViewResponse, ApiError> {
    mission::add_assignee(
        &client,
        args.mission_id,
        AssigneeDataArg {
            user_code: args.user_code,
            role: parse_role(&args.role)?,
        },
    )
    .await
}

#[tauri::command]
pub async fn remove_assignee(
    client: State<'_, HttpClient>,
    args: RemoveAssigneeArgs,
) -> Result<(), ApiError> {
    mission::remove_assignee(&client, args.mission_id, args.assignee_id).await
}

#[tauri::command]
pub async fn create_mission(
    client: State<'_, HttpClient>,
    args: CreateMissionArgs,
) -> Result<MissionViewResponse, ApiError> {
    let assignees = args
        .assignees
        .into_iter()
        .map(|a| -> Result<AssigneeDataArg, ApiError> {
            Ok(AssigneeDataArg {
                user_code: a.user_code,
                role: parse_role(&a.role)?,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    mission::create_mission(
        &client,
        CreateMissionRequest {
            project_code: args.project_code,
            mission_kind: parse_kind(&args.mission_kind)?,
            mission_code: args.mission_code,
            assignees,
        },
    )
    .await
}
