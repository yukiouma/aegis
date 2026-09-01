use tauri::State;

use crate::http::client::HttpClient;
use crate::http::dto::ApiError;
use crate::http::mission::{
    self, AssigneeDataArg, AssigneeViewResponse, CreateMissionRequest, MissionViewResponse,
};

// Tauri command argument conventions:
// Every other command in this crate (see commands/crf/form.rs,
// commands/user.rs, etc.) takes its arguments as separate function
// parameters — Tauri maps each JSON key in the args object to a
// parameter by camelCase name. The `kind` / `mission_kind` / `role`
// strings are re-parsed via serde's JSON deserialize so the call sites
// get validation errors back as `ApiError::Parse` instead of an
// opaque Tauri decode error.

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMissionAssigneeArg {
    pub user_code: String,
    pub role: String,
}

fn parse_kind(s: &str) -> Result<crate::http::mission::MissionKind, ApiError> {
    serde_json::from_value(serde_json::Value::String(s.to_string())).map_err(|e| ApiError::Parse {
        message: e.to_string(),
    })
}

fn parse_role(s: &str) -> Result<crate::http::mission::MissionRole, ApiError> {
    serde_json::from_value(serde_json::Value::String(s.to_string())).map_err(|e| ApiError::Parse {
        message: e.to_string(),
    })
}

#[tauri::command]
pub async fn list_missions_by_project(
    client: State<'_, HttpClient>,
    project_code: String,
    kind: Option<String>,
) -> Result<Vec<MissionViewResponse>, ApiError> {
    let kind = match kind.as_deref() {
        Some(s) => Some(parse_kind(s)?),
        None => None,
    };
    mission::list_by_project(&client, &project_code, kind).await
}

#[tauri::command]
pub async fn add_assignee(
    client: State<'_, HttpClient>,
    mission_id: i64,
    body: AssigneeDataArg,
) -> Result<AssigneeViewResponse, ApiError> {
    mission::add_assignee(&client, mission_id, body).await
}

#[tauri::command]
pub async fn remove_assignee(
    client: State<'_, HttpClient>,
    mission_id: i64,
    assignee_id: i64,
) -> Result<(), ApiError> {
    mission::remove_assignee(&client, mission_id, assignee_id).await
}

#[tauri::command]
pub async fn create_mission(
    client: State<'_, HttpClient>,
    project_code: String,
    mission_kind: String,
    mission_code: String,
    assignees: Vec<CreateMissionAssigneeArg>,
) -> Result<MissionViewResponse, ApiError> {
    let assignees = assignees
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
            project_code,
            mission_kind: parse_kind(&mission_kind)?,
            mission_code,
            assignees,
        },
    )
    .await
}

#[cfg(test)]
mod tests {
    //! Wire-shape tests. Tauri routes JSON keys to function parameters
    //! by camelCase name, so the frontend wrappers must send exactly:
    //!
    //!   list_missions_by_project: { projectCode, kind? }
    //!   add_assignee:             { missionId, body: { userCode, role } }
    //!   remove_assignee:          { missionId, assigneeId }
    //!   create_mission:           { projectCode, missionKind, missionCode,
    //!                              assignees: [{ userCode, role }] }
    //!
    //! These tests pin the deserialization shape of the only nested
    //! struct (`CreateMissionAssigneeArg`) and assert the camelCase
    //! convention holds, so a future rename can't silently desync
    //! Rust and TS.

    use super::*;
    use serde_json::json;

    #[test]
    fn create_mission_assignee_arg_deserializes_camel_case_payload() {
        let raw = json!({ "userCode": "carol", "role": "qc" });
        let a: CreateMissionAssigneeArg = serde_json::from_value(raw).unwrap();
        assert_eq!(a.user_code, "carol");
        assert_eq!(a.role, "qc");
    }

    #[test]
    fn create_mission_assignee_arg_rejects_snake_case_payload() {
        // The frontend always emits camelCase; if a future refactor
        // forgets the rename_all, this guard catches it before runtime.
        let raw = json!({ "user_code": "carol", "role": "qc" });
        let result: Result<CreateMissionAssigneeArg, _> = serde_json::from_value(raw);
        assert!(
            result.is_err(),
            "snake_case user_code must not parse against camelCase CreateMissionAssigneeArg"
        );
    }
}
