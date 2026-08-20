//! Tauri command shims for the terminology code-item HTTP layer.

use tauri::State;

use crate::http::client::HttpClient;
use crate::http::dto::ApiError;
use crate::http::terminology::code_item::{
    self, CodeItemListQuery, CodeItemPagedResponse, CodeItemViewResponse,
    CreateCodeItemRequest, UpdateCodeItemRequest,
};

#[tauri::command]
pub async fn create_code_item(
    client: State<'_, HttpClient>,
    codelist_id: i64,
    version_id: i64,
    code: String,
    submission_value: String,
    synonym: String,
    definition: String,
    nci_preferred_term: String,
) -> Result<CodeItemViewResponse, ApiError> {
    code_item::create(
        &client,
        CreateCodeItemRequest {
            codelist_id,
            version_id,
            code,
            submission_value,
            synonym,
            definition,
            nci_preferred_term,
        },
    )
    .await
}

#[tauri::command]
pub async fn list_code_items(
    client: State<'_, HttpClient>,
    codelist_id: i64,
    fragment: Option<String>,
    offset: u32,
    limit: u32,
) -> Result<CodeItemPagedResponse, ApiError> {
    code_item::list_paged(
        &client,
        CodeItemListQuery { codelist_id, fragment, offset, limit },
    )
    .await
}

#[tauri::command]
pub async fn update_code_item(
    client: State<'_, HttpClient>,
    id: i64,
    body: UpdateCodeItemRequest,
) -> Result<CodeItemViewResponse, ApiError> {
    code_item::update(&client, id, body).await
}

#[tauri::command]
pub async fn delete_code_item(
    client: State<'_, HttpClient>,
    id: i64,
) -> Result<(), ApiError> {
    code_item::delete(&client, id).await
}
