use tauri::State;

use crate::http::client::HttpClient;
use crate::http::dto::ApiError;
use crate::http::healthz;

#[tauri::command]
pub async fn healthz(client: State<'_, HttpClient>) -> Result<String, ApiError> {
    healthz::ping(&client).await
}
