use tauri::State;

use crate::http::client::HttpClient;
use crate::http::dto::ApiError;
use crate::http::product::{
    self, CreateProductRequest, ProductViewResponse, UpdateProductRequest,
};

#[tauri::command]
pub async fn create_product(
    client: State<'_, HttpClient>,
    code: String,
    name: String,
    description: String,
) -> Result<ProductViewResponse, ApiError> {
    product::create(
        &client,
        CreateProductRequest {
            code,
            name,
            description,
        },
    )
    .await
}

#[tauri::command]
pub async fn list_products(
    client: State<'_, HttpClient>,
) -> Result<Vec<ProductViewResponse>, ApiError> {
    product::list(&client).await
}

#[tauri::command]
pub async fn get_product_by_code(
    client: State<'_, HttpClient>,
    code: String,
) -> Result<ProductViewResponse, ApiError> {
    product::get_by_code(&client, &code).await
}

#[tauri::command]
pub async fn update_product(
    client: State<'_, HttpClient>,
    code: String,
    body: UpdateProductRequest,
) -> Result<ProductViewResponse, ApiError> {
    product::update(&client, &code, body).await
}
