//! Mission + assignee CRUD. Wire DTOs mirror the server's
//! `apps/server/aegis-server/src/transport/http/dto.rs` lines 1872–2028.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::client::HttpClient;
use super::dto::ApiError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissionKind {
    Crf,
    Sdtm,
    Adam,
    Tfl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissionRole {
    Dev,
    Qc,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssigneeDataArg {
    pub user_code: String,
    pub role: MissionRole,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssigneeViewResponse {
    pub id: i64,
    pub user_code: String,
    pub role: MissionRole,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MissionViewResponse {
    pub id: i64,
    pub project_code: String,
    pub mission_kind: MissionKind,
    pub mission_code: String,
    pub assignees: Vec<AssigneeViewResponse>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MissionListResponse {
    pub missions: Vec<MissionViewResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMissionRequest {
    pub project_code: String,
    pub mission_kind: MissionKind,
    pub mission_code: String,
    pub assignees: Vec<AssigneeDataArg>,
}

pub async fn list_by_project(
    c: &HttpClient,
    project_code: &str,
    kind: Option<MissionKind>,
) -> Result<Vec<MissionViewResponse>, ApiError> {
    // The server accepts `?kind=crf` etc.; we serialize the typed enum
    // via `serde_json` and pass it as a single query string value.
    let mut url = format!("/api/mission/by-project/{project_code}");
    if let Some(k) = kind {
        let kind_str = serde_json::to_string(&k)
            .map_err(|e| ApiError::Parse { message: e.to_string() })?
            .trim_matches('"')
            .to_string();
        url.push_str("?kind=");
        url.push_str(&kind_str);
    }
    let resp: MissionListResponse = c
        .request(reqwest::Method::GET, &url, None::<&()>)
        .await?;
    Ok(resp.missions)
}

pub async fn add_assignee(
    c: &HttpClient,
    mission_id: i64,
    body: AssigneeDataArg,
) -> Result<AssigneeViewResponse, ApiError> {
    c.request(
        reqwest::Method::POST,
        &format!("/api/mission/{mission_id}/assignee"),
        Some(&body),
    )
    .await
}

pub async fn remove_assignee(
    c: &HttpClient,
    mission_id: i64,
    assignee_id: i64,
) -> Result<(), ApiError> {
    // Use `request_bytes` + `let _ = ...` so an empty 204 body does
    // not get JSON-decoded. Every other DELETE endpoint in this
    // crate follows the same pattern (see `crf::annotation::delete`,
    // `crf::form::delete`, `domain_model::version::delete`, …).
    // Routing through `request::<Value>` would call
    // `serde_json::from_slice::<Value>(b"")` on the empty body and
    // return `decode_failed: EOF while parsing a value at line 1
    // column 0`, even though the server-side delete succeeded.
    let _ = c
        .request_bytes(
            reqwest::Method::DELETE,
            &format!("/api/mission/{mission_id}/assignee/{assignee_id}"),
            None::<&()>,
        )
        .await?;
    Ok(())
}

pub async fn create_mission(
    c: &HttpClient,
    body: CreateMissionRequest,
) -> Result<MissionViewResponse, ApiError> {
    c.request(reqwest::Method::POST, "/api/mission", Some(&body))
        .await
}

#[cfg(test)]
mod tests {
    //! Unit tests for serde shape only. The HTTP adapter round-trips
    //! are covered by the command tests in `commands/mission.rs`.
    //!
    //! The `remove_assignee_204_no_body` test below is the one
    //! exception: it round-trips the HTTP adapter against a wiremock
    //! server returning 204 No Content, because that exact failure
    //! mode (`decode_failed: EOF while parsing a value`) only
    //! manifests in the HTTP layer, not in serde.

    use super::*;
    use crate::http::client::{HttpClient, MemoryStore, TokenStore};
    use std::sync::Arc;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn remove_assignee_204_no_body() {
        // The server returns 204 No Content (empty body). The adapter
        // must NOT try to JSON-decode the empty bytes — every other
        // DELETE endpoint in the codebase uses `request_bytes` +
        // `let _ = ...; Ok(())` for exactly this reason. `remove_assignee`
        // was an outlier that used `request::<Value>`, which fails
        // with `decode_failed: EOF while parsing a value at line 1
        // column 0` on an empty body even though the DELETE
        // succeeded server-side.
        let server = MockServer::start().await;
        let store = Arc::new(MemoryStore::default());
        store.set_access_token("AT").await.unwrap();
        let m = Mock::given(method("DELETE"))
            .and(path("/api/mission/7/assignee/100"))
            .and(header("authorization", "Bearer AT"))
            .respond_with(ResponseTemplate::new(204));
        server.register(m).await;
        let c = HttpClient::new(server.uri(), store);

        let result = remove_assignee(&c, 7, 100).await;
        assert!(
            result.is_ok(),
            "remove_assignee must succeed on 204; got {result:?}"
        );
    }

    #[test]
    fn mission_kind_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&MissionKind::Crf).unwrap(),
            "\"crf\""
        );
        assert_eq!(
            serde_json::to_string(&MissionKind::Sdtm).unwrap(),
            "\"sdtm\""
        );
        assert_eq!(
            serde_json::to_string(&MissionKind::Adam).unwrap(),
            "\"adam\""
        );
        assert_eq!(
            serde_json::to_string(&MissionKind::Tfl).unwrap(),
            "\"tfl\""
        );
    }

    #[test]
    fn mission_role_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&MissionRole::Dev).unwrap(),
            "\"dev\""
        );
        assert_eq!(
            serde_json::to_string(&MissionRole::Qc).unwrap(),
            "\"qc\""
        );
    }

    #[test]
    fn create_mission_request_round_trips() {
        let body = CreateMissionRequest {
            project_code: "p1".into(),
            mission_kind: MissionKind::Crf,
            mission_code: "AE".into(),
            assignees: vec![AssigneeDataArg {
                user_code: "u1".into(),
                role: MissionRole::Dev,
            }],
        };
        let j = serde_json::to_string(&body).unwrap();
        assert!(j.contains("\"projectCode\":\"p1\""));
        assert!(j.contains("\"missionKind\":\"crf\""));
        assert!(j.contains("\"missionCode\":\"AE\""));
        assert!(j.contains("\"userCode\":\"u1\""));
        assert!(j.contains("\"role\":\"dev\""));
        let parsed: CreateMissionRequest = serde_json::from_str(&j).unwrap();
        assert_eq!(parsed, body);
    }

    #[test]
    fn mission_view_response_parses_full_wire_shape() {
        let j = r#"{
            "id": 42,
            "projectCode": "p1",
            "missionKind": "crf",
            "missionCode": "AE",
            "assignees": [
                {
                    "id": 7,
                    "userCode": "u1",
                    "role": "qc",
                    "createdAt": "2026-01-01T00:00:00Z",
                    "updatedAt": "2026-01-01T00:00:00Z"
                }
            ],
            "createdAt": "2026-01-01T00:00:00Z",
            "updatedAt": "2026-01-01T00:00:00Z"
        }"#;
        let m: MissionViewResponse = serde_json::from_str(j).unwrap();
        assert_eq!(m.id, 42);
        assert_eq!(m.mission_kind, MissionKind::Crf);
        assert_eq!(m.assignees.len(), 1);
        assert_eq!(m.assignees[0].role, MissionRole::Qc);
    }
}
