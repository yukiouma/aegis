//! Variables under `/api/domain-model/variables` and
//! `/api/domain-model/domains/{id}/variables`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::http::client::HttpClient;
use crate::http::dto::ApiError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum SdtmVariableType {
    Numeric,
    Character,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum SdtmVariableCore {
    Req,
    Exp,
    Perm,
    Supp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SdtmRole {
    Identifier,
    #[serde(rename = "Topic")]
    Topic,
    #[serde(rename = "Timing")]
    Timing,
    #[serde(rename = "Record Qualifier")]
    RecordQualifier,
    #[serde(rename = "Synonym Qualifier")]
    SynonymQualifier,
    #[serde(rename = "Variable Qualifier")]
    VariableQualifier,
    #[serde(rename = "Grouping Qualifier")]
    GroupingQualifier,
    Rule,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SdtmVariableDescriptionDetail {
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SdtmVariableDescription {
    pub lang: String,
    pub details: SdtmVariableDescriptionDetail,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SdtmVariableViewResponse {
    pub id: i64,
    pub domain_id: i64,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variable_controlled: Option<String>,
    pub variable_type: SdtmVariableType,
    pub variable_core: SdtmVariableCore,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variable_role: Option<SdtmRole>,
    pub variable_sequence: i64,
    pub descriptions: Vec<SdtmVariableDescription>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SdtmVariableListResponse {
    pub variables: Vec<SdtmVariableViewResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSdtmVariableRequest {
    pub domain_id: i64,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variable_controlled: Option<String>,
    pub variable_type: SdtmVariableType,
    pub variable_core: SdtmVariableCore,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variable_role: Option<SdtmRole>,
    pub variable_sequence: i64,
    pub descriptions: Vec<SdtmVariableDescription>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSdtmVariableRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variable_controlled: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variable_type: Option<SdtmVariableType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variable_core: Option<SdtmVariableCore>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variable_role: Option<Option<SdtmRole>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variable_sequence: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub descriptions: Option<Vec<SdtmVariableDescription>>,
}

pub async fn create(
    c: &HttpClient,
    body: CreateSdtmVariableRequest,
) -> Result<SdtmVariableViewResponse, ApiError> {
    c.request(
        reqwest::Method::POST,
        "/api/domain-model/variables",
        Some(&body),
    )
    .await
}

pub async fn list_by_domain(
    c: &HttpClient,
    domain_id: i64,
) -> Result<SdtmVariableListResponse, ApiError> {
    let resp: SdtmVariableListResponse = c
        .request(
            reqwest::Method::GET,
            &format!("/api/domain-model/domains/{domain_id}/variables"),
            None::<&()>,
        )
        .await?;
    Ok(resp)
}

pub async fn get_by_id(
    c: &HttpClient,
    id: i64,
) -> Result<SdtmVariableViewResponse, ApiError> {
    c.request(
        reqwest::Method::GET,
        &format!("/api/domain-model/variables/{id}"),
        None::<&()>,
    )
    .await
}

pub async fn update(
    c: &HttpClient,
    id: i64,
    body: UpdateSdtmVariableRequest,
) -> Result<SdtmVariableViewResponse, ApiError> {
    c.request(
        reqwest::Method::PUT,
        &format!("/api/domain-model/variables/{id}"),
        Some(&body),
    )
    .await
}

pub async fn delete(c: &HttpClient, id: i64) -> Result<(), ApiError> {
    let _ = c
        .request_bytes(
            reqwest::Method::DELETE,
            &format!("/api/domain-model/variables/{id}"),
            None::<&()>,
        )
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use crate::http::client::{HttpClient, MemoryStore, TokenStore};

    fn client(server: &MockServer) -> HttpClient {
        let store = Arc::new(MemoryStore::default());
        let _ = store.set_access_token("AT");
        let _ = store.set_refresh_token("RT");
        HttpClient::new(server.uri(), store)
    }

    #[tokio::test]
    async fn list_by_domain_returns_variables() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/domain-model/domains/5/variables"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "variables": [{
                    "id": 1, "domainId": 5, "name": "AETERM",
                    "variableType": "Character", "variableCore": "Req",
                    "variableRole": "Topic", "variableSequence": 1,
                    "descriptions": [{"lang": "en", "details": {"label": "Term"}}],
                    "createdAt": "2026-01-01T00:00:00Z",
                    "updatedAt": "2026-01-02T00:00:00Z"
                }]
            })))
            .mount(&server)
            .await;
        let variables = list_by_domain(&client(&server), 5).await.unwrap();
        assert_eq!(variables.variables.len(), 1);
        assert_eq!(variables.variables[0].name, "AETERM");
        assert_eq!(variables.variables[0].variable_sequence, 1);
    }

    #[tokio::test]
    async fn create_returns_view() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/domain-model/variables"))
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
                "id": 9, "domainId": 5, "name": "AESEV",
                "variableType": "Character", "variableCore": "Req",
                "variableRole": "Record Qualifier", "variableSequence": 2,
                "descriptions": [],
                "createdAt": "2026-02-01T00:00:00Z",
                "updatedAt": "2026-02-01T00:00:00Z"
            })))
            .mount(&server)
            .await;
        let v = create(
            &client(&server),
            CreateSdtmVariableRequest {
                domain_id: 5,
                name: "AESEV".into(),
                variable_controlled: None,
                variable_type: SdtmVariableType::Character,
                variable_core: SdtmVariableCore::Req,
                variable_role: Some(SdtmRole::RecordQualifier),
                variable_sequence: 2,
                descriptions: vec![],
            },
        )
        .await
        .unwrap();
        assert_eq!(v.id, 9);
        assert_eq!(v.name, "AESEV");
    }

    #[tokio::test]
    async fn update_returns_view() {
        let server = MockServer::start().await;
        Mock::given(method("PUT"))
            .and(path("/api/domain-model/variables/7"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": 7, "domainId": 5, "name": "renamed",
                "variableType": "Numeric", "variableCore": "Exp",
                "variableRole": null, "variableSequence": 1,
                "descriptions": [],
                "createdAt": "2025-12-01T00:00:00Z",
                "updatedAt": "2026-03-01T00:00:00Z"
            })))
            .mount(&server)
            .await;
        let v = update(
            &client(&server),
            7,
            UpdateSdtmVariableRequest {
                name: Some("renamed".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(v.name, "renamed");
    }

    #[tokio::test]
    async fn delete_succeeds() {
        let server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/api/domain-model/variables/9"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;
        delete(&client(&server), 9).await.unwrap();
    }

    #[test]
    fn update_request_skips_none_fields() {
        let body = UpdateSdtmVariableRequest {
            name: Some("renamed".into()),
            ..Default::default()
        };
        let j = serde_json::to_string(&body).unwrap();
        assert_eq!(j, r#"{"name":"renamed"}"#);
    }
}