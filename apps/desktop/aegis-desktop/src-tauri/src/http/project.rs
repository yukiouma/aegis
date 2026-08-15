//! Project CRUD.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::client::HttpClient;
use super::dto::ApiError;
use super::product::ProductViewResponse;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectMemberDataRequest {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub leaders: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub workers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserSummaryViewResponse {
    pub code: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectMemberViewResponse {
    pub leaders: Vec<UserSummaryViewResponse>,
    pub workers: Vec<UserSummaryViewResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectViewResponse {
    pub id: i32,
    pub code: String,
    pub description: String,
    pub product: ProductViewResponse,
    pub members: ProjectMemberViewResponse,
    pub unblind_members: ProjectMemberViewResponse,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectListResponse {
    pub projects: Vec<ProjectViewResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectRequest {
    pub code: String,
    pub description: String,
    pub product_id: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub members: Option<ProjectMemberDataRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unblind_members: Option<ProjectMemberDataRequest>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub members: Option<ProjectMemberDataRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unblind_members: Option<ProjectMemberDataRequest>,
}

pub async fn create(
    c: &HttpClient,
    body: CreateProjectRequest,
) -> Result<ProjectViewResponse, ApiError> {
    c.request(reqwest::Method::POST, "/api/project", Some(&body))
        .await
}

pub async fn list(c: &HttpClient) -> Result<Vec<ProjectViewResponse>, ApiError> {
    let resp: ProjectListResponse = c
        .request(reqwest::Method::GET, "/api/project", None::<&()>)
        .await?;
    Ok(resp.projects)
}

pub async fn get_by_code(
    c: &HttpClient,
    code: &str,
) -> Result<ProjectViewResponse, ApiError> {
    c.request(
        reqwest::Method::GET,
        &format!("/api/project/{code}"),
        None::<&()>,
    )
    .await
}

pub async fn update(
    c: &HttpClient,
    code: &str,
    body: UpdateProjectRequest,
) -> Result<ProjectViewResponse, ApiError> {
    c.request(
        reqwest::Method::PATCH,
        &format!("/api/project/{code}"),
        Some(&body),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::super::client::{HttpClient, MemoryStore, TokenStore};

    #[tokio::test]
    async fn list_returns_projects() {
        let server = MockServer::start().await;
        let store = Arc::new(MemoryStore::default());
        store.set_access_token("AT").await.unwrap();
        store.set_refresh_token("RT").await.unwrap();
        server
            .register(
                Mock::given(method("GET"))
                    .and(path("/api/project"))
                    .respond_with(ResponseTemplate::new(200).set_body_json(
                        serde_json::json!({
                            "projects": [{
                                "id": 1, "code": "p", "description": "",
                                "product": {
                                    "id": 1, "code": "x", "name": "X",
                                    "description": "", "active": true,
                                    "createdAt": "2026-01-01T00:00:00Z",
                                    "updatedAt": "2026-01-02T00:00:00Z"
                                },
                                "members": { "leaders": [], "workers": [] },
                                "unblindMembers": { "leaders": [], "workers": [] },
                                "active": true,
                                "createdAt": "2026-01-01T00:00:00Z",
                                "updatedAt": "2026-01-02T00:00:00Z"
                            }]
                        }),
                    )),
            )
            .await;
        let c = HttpClient::new(server.uri(), store);
        let projects = list(&c).await.unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].code, "p");
        assert_eq!(projects[0].product.code, "x");
    }

    #[test]
    fn update_skips_none_fields() {
        let body = UpdateProjectRequest {
            active: Some(false),
            ..Default::default()
        };
        let j = serde_json::to_string(&body).unwrap();
        assert_eq!(j, r#"{"active":false}"#);
    }

    #[test]
    fn project_member_data_request_omits_empty_arrays() {
        let body = ProjectMemberDataRequest::default();
        let j = serde_json::to_string(&body).unwrap();
        assert_eq!(j, "{}");
        let body = ProjectMemberDataRequest {
            leaders: vec!["a".into()],
            workers: vec![],
        };
        let j = serde_json::to_string(&body).unwrap();
        assert_eq!(j, r#"{"leaders":["a"]}"#);
    }
}
