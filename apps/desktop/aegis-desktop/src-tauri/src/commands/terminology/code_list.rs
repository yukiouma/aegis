//! Tauri command shims for the terminology code-list HTTP layer.

use tauri::State;

use crate::http::client::HttpClient;
use crate::http::dto::ApiError;
use crate::http::terminology::code_list::{
    self, CodeListSearchQuery, CodeListViewResponse, CreateCodeListRequest,
    UpdateCodeListRequest,
};

#[tauri::command]
pub async fn create_code_list(
    client: State<'_, HttpClient>,
    version_id: i64,
    code: String,
    extensible: bool,
    name: String,
    submission_value: String,
    synonym: String,
    definition: String,
    nci_preferred_term: String,
) -> Result<CodeListViewResponse, ApiError> {
    code_list::create(
        &client,
        CreateCodeListRequest {
            version_id,
            code,
            extensible,
            name,
            submission_value,
            synonym,
            definition,
            nci_preferred_term,
        },
    )
    .await
}

#[tauri::command]
pub async fn list_code_lists(
    client: State<'_, HttpClient>,
    version_id: i64,
) -> Result<Vec<CodeListViewResponse>, ApiError> {
    code_list::list(&client, version_id).await
}

#[tauri::command]
pub async fn update_code_list(
    client: State<'_, HttpClient>,
    id: i64,
    body: UpdateCodeListRequest,
) -> Result<CodeListViewResponse, ApiError> {
    code_list::update(&client, id, body).await
}

#[tauri::command]
pub async fn delete_code_list(
    client: State<'_, HttpClient>,
    id: i64,
) -> Result<(), ApiError> {
    code_list::delete(&client, id).await
}

#[tauri::command]
pub async fn search_code_lists(
    client: State<'_, HttpClient>,
    version_id: i64,
    fragment: String,
    limit: u32,
) -> Result<Vec<code_list::CodeListSearchHitResponse>, ApiError> {
    code_list::search(
        &client,
        CodeListSearchQuery {
            version_id,
            fragment,
            limit,
        },
    )
    .await
}