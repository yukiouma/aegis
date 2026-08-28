//! Tauri command shims for `http::crf::item`.

use tauri::State;

use crate::http::client::HttpClient;
use crate::http::crf::item::{self, CrfItemListResponse, CrfItemViewResponse};
use crate::http::dto::ApiError;

#[tauri::command]
pub async fn list_crf_items_by_form(
    client: State<'_, HttpClient>,
    form_id: i64,
) -> Result<CrfItemListResponse, ApiError> {
    item::list_by_form(&client, form_id).await
}

#[tauri::command]
pub async fn get_crf_item_by_id(
    client: State<'_, HttpClient>,
    id: i64,
) -> Result<CrfItemViewResponse, ApiError> {
    item::get_by_id(&client, id).await
}
