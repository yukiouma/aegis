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

/// Controlled-terminology metadata carried by an SDTM variable.
///
/// Mirrors the server's `dto::SdtmVariableControlled`: the same
/// camelCase rename, and both lists default to `None` when the key is
/// absent or explicitly `null`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SdtmVariableControlled {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<Vec<String>>,
}

/// Deserialise a nullable field into `Option<Option<T>>` so all three
/// states stay distinguishable across the Tauri hop:
///
/// - a missing key is `None` — "leave the column alone"
/// - an explicit `null` is `Some(None)` — "clear the column"
/// - a value is `Some(Some(v))` — "replace the column"
///
/// A plain `Option<Option<T>>` derive collapses the first two: serde
/// maps `null` to the outer `None`, which `skip_serializing_if` then
/// drops from the outgoing request.
fn double_option<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SdtmVariableViewResponse {
    pub id: i64,
    pub domain_id: i64,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variable_controlled: Option<SdtmVariableControlled>,
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
    pub variable_controlled: Option<SdtmVariableControlled>,
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
    /// Three-state: absent = unchanged, `null` = clear, value = replace.
    ///
    /// `skip_serializing_if` keeps "unchanged" expressed as an absent
    /// key; `double_option` keeps an incoming `null` distinguishable
    /// from an absent key. The two are independent — one governs
    /// serialization, the other deserialization.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "double_option"
    )]
    pub variable_controlled: Option<Option<SdtmVariableControlled>>,
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

pub async fn get_by_id(c: &HttpClient, id: i64) -> Result<SdtmVariableViewResponse, ApiError> {
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

    #[test]
    fn view_decodes_controlled_code_and_format_lists() {
        let json = serde_json::json!({
            "id": 1, "domainId": 5, "name": "VSTESTCD",
            "variableControlled": { "code": ["VSTESTCD"], "format": ["Codelist"] },
            "variableType": "Character", "variableCore": "Req",
            "variableSequence": 1, "descriptions": [],
            "createdAt": "2026-01-01T00:00:00Z", "updatedAt": "2026-01-01T00:00:00Z"
        });
        let v: SdtmVariableViewResponse = serde_json::from_value(json).unwrap();
        assert_eq!(
            v.variable_controlled,
            Some(SdtmVariableControlled {
                code: Some(vec!["VSTESTCD".into()]),
                format: Some(vec!["Codelist".into()]),
            })
        );
    }

    #[test]
    fn view_decodes_null_controlled_as_none() {
        let json = serde_json::json!({
            "id": 1, "domainId": 5, "name": "VSTESTCD",
            "variableControlled": null,
            "variableType": "Character", "variableCore": "Req",
            "variableSequence": 1, "descriptions": [],
            "createdAt": "2026-01-01T00:00:00Z", "updatedAt": "2026-01-01T00:00:00Z"
        });
        let v: SdtmVariableViewResponse = serde_json::from_value(json).unwrap();
        assert_eq!(v.variable_controlled, None);
    }

    #[test]
    fn view_decodes_absent_format_list_as_none() {
        let json = serde_json::json!({
            "id": 1, "domainId": 5, "name": "VSTESTCD",
            "variableControlled": { "code": ["VSTESTCD"] },
            "variableType": "Character", "variableCore": "Req",
            "variableSequence": 1, "descriptions": [],
            "createdAt": "2026-01-01T00:00:00Z", "updatedAt": "2026-01-01T00:00:00Z"
        });
        let v: SdtmVariableViewResponse = serde_json::from_value(json).unwrap();
        assert_eq!(
            v.variable_controlled,
            Some(SdtmVariableControlled {
                code: Some(vec!["VSTESTCD".into()]),
                format: None,
            })
        );
    }

    #[test]
    fn update_request_decodes_null_controlled_as_clear() {
        // The Tauri payload is where the webview's `null` arrives. A plain
        // `Option<Option<T>>` derive maps it to the outer `None`, the key
        // is then dropped by `skip_serializing_if`, and the server reads
        // "unchanged" instead of "clear".
        let body: UpdateSdtmVariableRequest =
            serde_json::from_str(r#"{"variableControlled":null}"#).unwrap();
        assert_eq!(body.variable_controlled, Some(None));
    }

    #[test]
    fn update_request_decodes_absent_controlled_as_unchanged() {
        let body: UpdateSdtmVariableRequest =
            serde_json::from_str(r#"{"name":"renamed"}"#).unwrap();
        assert_eq!(body.variable_controlled, None);
    }

    #[test]
    fn update_request_decodes_controlled_object_as_replace() {
        let body: UpdateSdtmVariableRequest =
            serde_json::from_str(r#"{"variableControlled":{"code":["VSTESTCD"]}}"#).unwrap();
        assert_eq!(
            body.variable_controlled,
            Some(Some(SdtmVariableControlled {
                code: Some(vec!["VSTESTCD".into()]),
                format: None,
            }))
        );
    }

    #[test]
    fn update_request_serializes_controlled_object() {
        let body = UpdateSdtmVariableRequest {
            variable_controlled: Some(Some(SdtmVariableControlled {
                code: Some(vec!["VSTESTCD".into()]),
                format: None,
            })),
            ..Default::default()
        };
        assert_eq!(
            serde_json::to_string(&body).unwrap(),
            r#"{"variableControlled":{"code":["VSTESTCD"]}}"#
        );
    }

    #[test]
    fn update_request_serializes_controlled_null_as_clear() {
        let body = UpdateSdtmVariableRequest {
            variable_controlled: Some(None),
            ..Default::default()
        };
        assert_eq!(
            serde_json::to_string(&body).unwrap(),
            r#"{"variableControlled":null}"#
        );
    }
}
