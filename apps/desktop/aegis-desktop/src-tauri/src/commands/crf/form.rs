//! Tauri command shims for `http::crf::form`.

use tauri::State;

use crate::http::client::HttpClient;
use crate::http::crf::form::{
    self, CreateCrfFormRequest, CrfFormDetailResponse, CrfFormListResponse, CrfFormViewResponse,
    SetCrfApprovedRequest, UpdateCrfFormRequest,
};
use crate::http::dto::ApiError;
use crate::http::mission::{self, IssueState};

#[tauri::command]
pub async fn list_crf_forms_by_version(
    client: State<'_, HttpClient>,
    version_id: i64,
) -> Result<CrfFormListResponse, ApiError> {
    form::list_by_version(&client, version_id).await
}

#[tauri::command]
pub async fn create_crf_form(
    client: State<'_, HttpClient>,
    version_id: i64,
    body: CreateCrfFormRequest,
) -> Result<CrfFormViewResponse, ApiError> {
    form::create(&client, version_id, body).await
}

#[tauri::command]
pub async fn update_crf_form(
    client: State<'_, HttpClient>,
    id: i64,
    body: UpdateCrfFormRequest,
) -> Result<CrfFormViewResponse, ApiError> {
    form::update(&client, id, body).await
}

#[tauri::command]
pub async fn delete_crf_form(client: State<'_, HttpClient>, id: i64) -> Result<(), ApiError> {
    form::delete(&client, id).await
}

#[tauri::command]
pub async fn get_crf_form_by_id(
    client: State<'_, HttpClient>,
    id: i64,
) -> Result<CrfFormViewResponse, ApiError> {
    form::get_by_id(&client, id).await
}

#[tauri::command]
pub async fn get_crf_form_details(
    client: State<'_, HttpClient>,
    id: i64,
) -> Result<CrfFormDetailResponse, ApiError> {
    form::details(&client, id).await
}

#[tauri::command]
pub async fn search_crf_forms_by_version(
    client: State<'_, HttpClient>,
    version_id: i64,
    fragment: String,
) -> Result<CrfFormListResponse, ApiError> {
    form::search_by_version(&client, version_id, fragment).await
}

/// Toggle the `approved` flag on a CRF form. Re-fetches the
/// current open issues for the form's mission from the server
/// (the TanStack-Query-cached count is intentionally NOT
/// trusted) and rejects `approved=true` when at least one open
/// issue exists. Returns 409 `ApiError::Http` in that case; the
/// server approve endpoint is **not** called.
///
/// `approved=false` (un-approve) bypasses the issue fetch
/// entirely — un-approving has no gate.
#[tauri::command]
pub async fn set_crf_form_approved(
    client: State<'_, HttpClient>,
    id: i64,
    approved: bool,
    mission_id: i64,
) -> Result<CrfFormViewResponse, ApiError> {
    set_approved_impl(&client, id, approved, mission_id).await
}

/// Gate + proxy implementation. `#[tauri::command]` shims over
/// `tauri::State`, which is hard to construct in tests; the real
/// logic lives here so tests can drive it directly with a real
/// `HttpClient` against `wiremock`. Mirrors the codebase's
/// convention in
/// `commands::user::current_user_resolves_sub_to_user_view`.
pub async fn set_approved_impl(
    client: &HttpClient,
    id: i64,
    approved: bool,
    mission_id: i64,
) -> Result<CrfFormViewResponse, ApiError> {
    if approved {
        let issues =
            mission::list_issues_by_mission(client, mission_id, Some(IssueState::Opened)).await?;
        if !issues.is_empty() {
            return Err(ApiError::Http {
                status: 409,
                code: "crf.open_issues_blocking_approval".into(),
                message: format!("{} open issue(s) blocking approval", issues.len()),
            });
        }
    }
    form::set_approved(client, id, SetCrfApprovedRequest { approved }).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::client::{HttpClient, MemoryStore, TokenStore};
    use std::sync::Arc;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn client(server: &MockServer) -> HttpClient {
        let store = Arc::new(MemoryStore::default());
        store.set_access_token("AT").await.unwrap();
        store.set_refresh_token("RT").await.unwrap();
        HttpClient::new(server.uri(), store)
    }

    fn form_view_json(id: i64, approved: bool) -> serde_json::Value {
        serde_json::json!({
            "id": id, "versionId": 7, "code": "AE", "name": "Adverse Events",
            "order": 0, "notSubmitted": false, "approved": approved,
            "createdAt": "2026-01-01T00:00:00Z", "updatedAt": "2026-01-02T00:00:00Z"
        })
    }

    fn open_issue(id: i64, mission_id: i64) -> serde_json::Value {
        serde_json::json!({
            "id": id, "missionId": mission_id,
            "issuer": "alice", "description": "needs review",
            "state": "opened", "comments": [],
            "createdAt": "2026-01-01T00:00:00Z", "updatedAt": "2026-01-02T00:00:00Z"
        })
    }

    #[tokio::test]
    async fn gate_trips_when_open_issues_exist() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/mission/by-mission/42/issues"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "issues": [open_issue(1, 42)]
            })))
            .mount(&server)
            .await;
        // Approval POST mounted but should NEVER be hit (strict: zero calls allowed).
        Mock::given(method("POST"))
            .and(path("/api/crf/forms/11/approval"))
            .respond_with(ResponseTemplate::new(200).set_body_json(form_view_json(11, true)))
            .expect(0)
            .mount(&server)
            .await;

        let err = set_approved_impl(&client(&server).await, 11, true, 42)
            .await
            .unwrap_err();
        match err {
            ApiError::Http { status, code, .. } => {
                assert_eq!(status, 409);
                assert_eq!(code, "crf.open_issues_blocking_approval");
            }
            other => panic!("expected ApiError::Http 409, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn gate_passes_when_no_open_issues() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/mission/by-mission/42/issues"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "issues": []
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/crf/forms/11/approval"))
            .respond_with(ResponseTemplate::new(200).set_body_json(form_view_json(11, true)))
            .expect(1)
            .mount(&server)
            .await;
        let f = set_approved_impl(&client(&server).await, 11, true, 42)
            .await
            .unwrap();
        assert_eq!(f.id, 11);
        assert!(f.approved);
    }

    #[tokio::test]
    async fn unapprove_skips_issue_fetch() {
        let server = MockServer::start().await;
        // Issues endpoint mounted but MUST NOT be hit on un-approve.
        Mock::given(method("GET"))
            .and(path("/api/mission/by-mission/42/issues"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "issues": [open_issue(1, 42)]
            })))
            .expect(0)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/crf/forms/11/approval"))
            .respond_with(ResponseTemplate::new(200).set_body_json(form_view_json(11, false)))
            .expect(1)
            .mount(&server)
            .await;
        let f = set_approved_impl(&client(&server).await, 11, false, 42)
            .await
            .unwrap();
        assert!(!f.approved);
    }
}
