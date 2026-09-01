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
    // The frontend wraps the payload under `body`, matching the
    // `body: SomeStruct` convention used by create_crf_form /
    // update_crf_form / etc. Tauri maps the JSON `body` field to this
    // parameter directly.
    pub mission_id: i64,
    pub body: AssigneeDataArg,
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
    mission::add_assignee(&client, args.mission_id, args.body).await
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

#[cfg(test)]
mod tests {
    //! Wire-shape tests for the Tauri argument structs. These pin the
    //! JSON shape the frontend wrapper produces so a future
    //! restructuring can't silently desync Rust and TS (this exact
    //! drift caused `add_assignee` to break against a real Tauri
    //! runtime before this guard was added).

    use super::*;
    use crate::http::mission::MissionRole;
    use serde_json::json;

    #[test]
    fn add_assignee_args_deserialize_wire_shape_with_body_wrapper() {
        // Matches `api.addAssignee(...)` in shared/api/index.ts which
        // emits `{ missionId, body: { userCode, role } }`.
        let raw = json!({
            "missionId": 10,
            "body": { "userCode": "carol", "role": "qc" }
        });
        let args: AddAssigneeArgs = serde_json::from_value(raw).unwrap();
        assert_eq!(args.mission_id, 10);
        assert_eq!(args.body.user_code, "carol");
        assert_eq!(args.body.role, MissionRole::Qc);
    }

    #[test]
    fn remove_assignee_args_deserialize_flat_shape() {
        let raw = json!({ "missionId": 10, "assigneeId": 100 });
        let args: RemoveAssigneeArgs = serde_json::from_value(raw).unwrap();
        assert_eq!(args.mission_id, 10);
        assert_eq!(args.assignee_id, 100);
    }

    #[test]
    fn list_missions_by_project_args_deserialize_flat_shape() {
        let raw = json!({ "projectCode": "alpha", "kind": "crf" });
        let args: ListMissionsByProjectArgs = serde_json::from_value(raw).unwrap();
        assert_eq!(args.project_code, "alpha");
        assert_eq!(args.kind.as_deref(), Some("crf"));
    }

    #[test]
    fn create_mission_args_deserialize_flat_shape_with_nested_assignees() {
        let raw = json!({
            "projectCode": "alpha",
            "missionKind": "crf",
            "missionCode": "VS",
            "assignees": [{ "userCode": "carol", "role": "qc" }]
        });
        let args: CreateMissionArgs = serde_json::from_value(raw).unwrap();
        assert_eq!(args.project_code, "alpha");
        assert_eq!(args.mission_kind, "crf");
        assert_eq!(args.mission_code, "VS");
        assert_eq!(args.assignees.len(), 1);
        assert_eq!(args.assignees[0].user_code, "carol");
        assert_eq!(args.assignees[0].role, "qc");
    }

    #[test]
    fn add_assignee_args_rejects_flat_user_code_role_shape() {
        // Regression guard: the *old* `AddAssigneeArgs` flattened the
        // body into top-level `user_code` / `role` fields, which would
        // succeed against the old struct but does NOT match what the
        // frontend actually sends. With the new `body:` wrapper, this
        // shape must fail to deserialize.
        let raw = json!({
            "missionId": 10,
            "userCode": "carol",
            "role": "qc"
        });
        let result: Result<AddAssigneeArgs, _> = serde_json::from_value(raw);
        assert!(
            result.is_err(),
            "flat userCode/role shape must not parse against the body-wrapped AddAssigneeArgs"
        );
    }
}
