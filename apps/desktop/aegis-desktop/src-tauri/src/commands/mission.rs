use tauri::State;
use trace_id::TraceIdGenerator;

use crate::http::client::{HttpClient, TRACE_ID};
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

fn parse_issue_state(s: &str) -> Result<crate::http::mission::IssueState, ApiError> {
    serde_json::from_value(serde_json::Value::String(s.to_string())).map_err(|e| ApiError::Parse {
        message: e.to_string(),
    })
}

#[tauri::command]
pub async fn list_missions_by_project(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    project_code: String,
    kind: Option<String>,
) -> Result<Vec<MissionViewResponse>, ApiError> {
    let trace_id = generator.client_side();
    let span =
        tracing::info_span!("command", trace_id = %trace_id, command = "list_missions_by_project");
    let _enter = span.enter();
    tracing::info!("enter");

    let result = TRACE_ID
        .scope(trace_id, async {
            let kind = match kind.as_deref() {
                Some(s) => Some(parse_kind(s)?),
                None => None,
            };
            mission::list_by_project(&client, &project_code, kind).await
        })
        .await;

    match &result {
        Ok(_) => tracing::info!("success"),
        Err(e) => tracing::error!(error = %e, "failed"),
    }
    result
}

#[tauri::command]
pub async fn add_assignee(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    mission_id: i64,
    body: AssigneeDataArg,
) -> Result<AssigneeViewResponse, ApiError> {
    let trace_id = generator.client_side();
    let span = tracing::info_span!("command", trace_id = %trace_id, command = "add_assignee");
    let _enter = span.enter();
    tracing::info!("enter");

    let result = TRACE_ID
        .scope(trace_id, async {
            mission::add_assignee(&client, mission_id, body).await
        })
        .await;

    match &result {
        Ok(_) => tracing::info!("success"),
        Err(e) => tracing::error!(error = %e, "failed"),
    }
    result
}

#[tauri::command]
pub async fn remove_assignee(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    mission_id: i64,
    assignee_id: i64,
) -> Result<(), ApiError> {
    let trace_id = generator.client_side();
    let span = tracing::info_span!("command", trace_id = %trace_id, command = "remove_assignee");
    let _enter = span.enter();
    tracing::info!("enter");

    let result = TRACE_ID
        .scope(trace_id, async {
            mission::remove_assignee(&client, mission_id, assignee_id).await
        })
        .await;

    match &result {
        Ok(_) => tracing::info!("success"),
        Err(e) => tracing::error!(error = %e, "failed"),
    }
    result
}

#[tauri::command]
pub async fn create_mission(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    project_code: String,
    mission_kind: String,
    mission_code: String,
    assignees: Vec<CreateMissionAssigneeArg>,
) -> Result<MissionViewResponse, ApiError> {
    let trace_id = generator.client_side();
    let span = tracing::info_span!("command", trace_id = %trace_id, command = "create_mission");
    let _enter = span.enter();
    tracing::info!("enter");

    let result = TRACE_ID
        .scope(trace_id, async {
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
        })
        .await;

    match &result {
        Ok(_) => tracing::info!("success"),
        Err(e) => tracing::error!(error = %e, "failed"),
    }
    result
}

#[tauri::command]
pub async fn list_issues_by_mission(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    mission_id: i64,
    state: Option<String>,
) -> Result<Vec<mission::IssueViewResponse>, ApiError> {
    let trace_id = generator.client_side();
    let span =
        tracing::info_span!("command", trace_id = %trace_id, command = "list_issues_by_mission");
    let _enter = span.enter();
    tracing::info!("enter");

    let result = TRACE_ID
        .scope(trace_id, async {
            let parsed = match state.as_deref() {
                Some(s) => Some(parse_issue_state(s)?),
                None => None,
            };
            mission::list_issues_by_mission(&client, mission_id, parsed).await
        })
        .await;

    match &result {
        Ok(_) => tracing::info!("success"),
        Err(e) => tracing::error!(error = %e, "failed"),
    }
    result
}

#[tauri::command]
pub async fn create_issue(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    mission_id: i64,
    body: mission::CreateIssueRequest,
) -> Result<mission::IssueViewResponse, ApiError> {
    let trace_id = generator.client_side();
    let span = tracing::info_span!("command", trace_id = %trace_id, command = "create_issue");
    let _enter = span.enter();
    tracing::info!("enter");

    let result = TRACE_ID
        .scope(trace_id, async {
            mission::create_issue(&client, mission_id, body).await
        })
        .await;

    match &result {
        Ok(_) => tracing::info!("success"),
        Err(e) => tracing::error!(error = %e, "failed"),
    }
    result
}

#[tauri::command]
pub async fn patch_issue_state(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    issue_id: i64,
    state: String,
) -> Result<mission::IssueViewResponse, ApiError> {
    let trace_id = generator.client_side();
    let span = tracing::info_span!("command", trace_id = %trace_id, command = "patch_issue_state");
    let _enter = span.enter();
    tracing::info!("enter");

    let result = TRACE_ID
        .scope(trace_id, async {
            mission::patch_issue_state(&client, issue_id, parse_issue_state(&state)?).await
        })
        .await;

    match &result {
        Ok(_) => tracing::info!("success"),
        Err(e) => tracing::error!(error = %e, "failed"),
    }
    result
}

#[tauri::command]
pub async fn update_issue_description(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    issue_id: i64,
    body: mission::UpdateIssueDescriptionRequest,
) -> Result<mission::IssueViewResponse, ApiError> {
    let trace_id = generator.client_side();
    let span =
        tracing::info_span!("command", trace_id = %trace_id, command = "update_issue_description");
    let _enter = span.enter();
    tracing::info!("enter");

    let result = TRACE_ID
        .scope(trace_id, async {
            mission::update_issue_description(&client, issue_id, body).await
        })
        .await;

    match &result {
        Ok(_) => tracing::info!("success"),
        Err(e) => tracing::error!(error = %e, "failed"),
    }
    result
}

#[tauri::command]
pub async fn append_comment(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    issue_id: i64,
    body: mission::AppendCommentRequest,
) -> Result<mission::IssueViewResponse, ApiError> {
    let trace_id = generator.client_side();
    let span = tracing::info_span!("command", trace_id = %trace_id, command = "append_comment");
    let _enter = span.enter();
    tracing::info!("enter");

    let result = TRACE_ID
        .scope(trace_id, async {
            mission::append_comment(&client, issue_id, body).await
        })
        .await;

    match &result {
        Ok(_) => tracing::info!("success"),
        Err(e) => tracing::error!(error = %e, "failed"),
    }
    result
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
    //!   list_issues_by_mission:   { missionId, state? }
    //!   create_issue:             { missionId, body: { targetItem?, description } }
    //!   patch_issue_state:        { issueId, state: "opened"|"closed" }
    //!   update_issue_description: { issueId, body: { description } }
    //!   append_comment:           { issueId, body: { content } }
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
        // The frontend always sends camelCase; if a future refactor
        // forgets the rename_all, this guard catches it before runtime.
        let raw = json!({ "user_code": "carol", "role": "qc" });
        let result: Result<CreateMissionAssigneeArg, _> = serde_json::from_value(raw);
        assert!(
            result.is_err(),
            "snake_case user_code must not parse against camelCase CreateMissionAssigneeArg"
        );
    }

    #[test]
    fn create_issue_request_deserializes_camel_case_payload() {
        // Frontend sends { missionId, body: { targetItem?, description } }.
        let raw = json!({
            "missionId": 10,
            "body": { "targetItem": "AE", "description": "missing row" }
        });
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Wrapper {
            mission_id: i64,
            body: mission::CreateIssueRequest,
        }
        let w: Wrapper = serde_json::from_value(raw).unwrap();
        assert_eq!(w.mission_id, 10);
        assert_eq!(w.body.target_item.as_deref(), Some("AE"));
        assert_eq!(w.body.description, "missing row");
    }

    #[test]
    fn create_issue_request_with_omitted_target_item_parses_as_none() {
        // Frontend may omit `targetItem` for whole-mission issues;
        // serde must accept that. Negative "rejects snake_case" test is
        // not applicable here because `target_item: Option<String>` +
        // `#[serde(default)]` make the snake_case key silently None
        // (the `rename_all = "camelCase"` rule still pins the wire to
        // `targetItem`, but unknown fields are ignored by default and
        // the optional field falls back to None).
        let raw = json!({
            "missionId": 10,
            "body": { "description": "missing row" }
        });
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Wrapper {
            mission_id: i64,
            body: mission::CreateIssueRequest,
        }
        let w: Wrapper = serde_json::from_value(raw).unwrap();
        assert_eq!(w.mission_id, 10);
        assert!(w.body.target_item.is_none());
        assert_eq!(w.body.description, "missing row");
    }
}
