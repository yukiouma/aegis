//! Tauri command shims for `http::crf::domain_annotation`.

use tauri::State;

use crate::http::client::HttpClient;
use crate::http::crf::domain_annotation::{
    self, CreateDomainAnnotationRequest, UpdateDomainAnnotationRequest,
};
use crate::http::crf::form::DomainAnnotationViewResponse;
use crate::http::dto::ApiError;

#[tauri::command]
pub async fn create_crf_domain_annotation(
    client: State<'_, HttpClient>,
    form_id: i64,
    body: CreateDomainAnnotationRequest,
) -> Result<DomainAnnotationViewResponse, ApiError> {
    domain_annotation::create(&client, form_id, body).await
}

#[tauri::command]
pub async fn list_crf_domain_annotations_by_form(
    client: State<'_, HttpClient>,
    form_id: i64,
) -> Result<domain_annotation::DomainAnnotationListResponse, ApiError> {
    domain_annotation::list_by_form(&client, form_id).await
}

#[tauri::command]
pub async fn update_crf_domain_annotation(
    client: State<'_, HttpClient>,
    id: i64,
    body: UpdateDomainAnnotationRequest,
) -> Result<DomainAnnotationViewResponse, ApiError> {
    domain_annotation::update(&client, id, body).await
}

#[tauri::command]
pub async fn delete_crf_domain_annotation(
    client: State<'_, HttpClient>,
    id: i64,
) -> Result<(), ApiError> {
    domain_annotation::delete(&client, id).await
}

#[tauri::command]
pub async fn search_crf_domain_annotations_by_version(
    client: State<'_, HttpClient>,
    version_id: i64,
    fragment: String,
) -> Result<domain_annotation::DomainAnnotationListResponse, ApiError> {
    domain_annotation::search_by_version(&client, version_id, fragment).await
}
