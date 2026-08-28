//! Tauri command shims for `http::crf::annotation`.

use tauri::State;

use crate::http::client::HttpClient;
use crate::http::crf::annotation::{
    self, CreateAnnotationRequest, UpdateAnnotationRequest,
};
use crate::http::crf::form::AnnotationViewResponse;
use crate::http::dto::ApiError;

#[tauri::command]
pub async fn create_crf_annotation(
    client: State<'_, HttpClient>,
    body: CreateAnnotationRequest,
) -> Result<AnnotationViewResponse, ApiError> {
    annotation::create(&client, body).await
}

#[tauri::command]
pub async fn update_crf_annotation(
    client: State<'_, HttpClient>,
    id: i64,
    body: UpdateAnnotationRequest,
) -> Result<AnnotationViewResponse, ApiError> {
    annotation::update(&client, id, body).await
}

#[tauri::command]
pub async fn delete_crf_annotation(
    client: State<'_, HttpClient>,
    id: i64,
) -> Result<(), ApiError> {
    annotation::delete(&client, id).await
}
