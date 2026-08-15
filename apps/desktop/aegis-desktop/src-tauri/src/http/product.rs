//! Product CRUD.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::client::HttpClient;
use super::dto::ApiError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductViewResponse {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub description: String,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductListResponse {
    pub products: Vec<ProductViewResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProductRequest {
    pub code: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProductRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
}

pub async fn create(
    c: &HttpClient,
    body: CreateProductRequest,
) -> Result<ProductViewResponse, ApiError> {
    c.request(reqwest::Method::POST, "/api/product", Some(&body))
        .await
}

pub async fn list(c: &HttpClient) -> Result<Vec<ProductViewResponse>, ApiError> {
    let resp: ProductListResponse = c
        .request(reqwest::Method::GET, "/api/product", None::<&()>)
        .await?;
    Ok(resp.products)
}

pub async fn get_by_code(
    c: &HttpClient,
    code: &str,
) -> Result<ProductViewResponse, ApiError> {
    c.request(
        reqwest::Method::GET,
        &format!("/api/product/{code}"),
        None::<&()>,
    )
    .await
}

pub async fn update(
    c: &HttpClient,
    code: &str,
    body: UpdateProductRequest,
) -> Result<ProductViewResponse, ApiError> {
    c.request(
        reqwest::Method::PATCH,
        &format!("/api/product/{code}"),
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
    async fn list_returns_products() {
        let server = MockServer::start().await;
        let store = Arc::new(MemoryStore::default());
        store.set_access_token("AT").await.unwrap();
        store.set_refresh_token("RT").await.unwrap();
        server
            .register(
                Mock::given(method("GET"))
                    .and(path("/api/product"))
                    .respond_with(ResponseTemplate::new(200).set_body_json(
                        serde_json::json!({
                            "products": [{
                                "id": 1, "code": "x", "name": "X",
                                "description": "", "active": true,
                                "createdAt": "2026-01-01T00:00:00Z",
                                "updatedAt": "2026-01-02T00:00:00Z"
                            }]
                        }),
                    )),
            )
            .await;
        let c = HttpClient::new(server.uri(), store);
        let products = list(&c).await.unwrap();
        assert_eq!(products.len(), 1);
        assert_eq!(products[0].code, "x");
    }

    #[test]
    fn update_skips_none() {
        let body = UpdateProductRequest {
            active: Some(false),
            ..Default::default()
        };
        let j = serde_json::to_string(&body).unwrap();
        assert_eq!(j, r#"{"active":false}"#);
    }
}
