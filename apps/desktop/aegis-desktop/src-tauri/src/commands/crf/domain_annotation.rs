//! Tauri command shims for `http::crf::domain_annotation`.

use tauri::State;
use trace_id::TraceIdGenerator;

use crate::http::client::{HttpClient, TRACE_ID};
use crate::http::crf::domain_annotation::{
    self, CreateDomainAnnotationRequest, UpdateDomainAnnotationRequest,
};
use crate::http::crf::form::DomainAnnotationViewResponse;
use crate::http::dto::ApiError;

#[tauri::command]
pub async fn create_crf_domain_annotation(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    form_id: i64,
    body: CreateDomainAnnotationRequest,
) -> Result<DomainAnnotationViewResponse, ApiError> {
    let trace_id = generator.client_side();
    let span = tracing::info_span!("command", trace_id = %trace_id, command = "create_crf_domain_annotation");
    let _enter = span.enter();
    tracing::info!("enter");

    let result = TRACE_ID
        .scope(trace_id, async {
            domain_annotation::create(&client, form_id, body).await
        })
        .await;

    match &result {
        Ok(_) => tracing::info!("success"),
        Err(e) => tracing::error!(error = %e, "failed"),
    }
    result
}

#[tauri::command]
pub async fn list_crf_domain_annotations_by_form(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    form_id: i64,
) -> Result<domain_annotation::DomainAnnotationListResponse, ApiError> {
    let trace_id = generator.client_side();
    let span = tracing::info_span!("command", trace_id = %trace_id, command = "list_crf_domain_annotations_by_form");
    let _enter = span.enter();
    tracing::info!("enter");

    let result = TRACE_ID
        .scope(trace_id, async {
            domain_annotation::list_by_form(&client, form_id).await
        })
        .await;

    match &result {
        Ok(_) => tracing::info!("success"),
        Err(e) => tracing::error!(error = %e, "failed"),
    }
    result
}

#[tauri::command]
pub async fn update_crf_domain_annotation(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    id: i64,
    body: UpdateDomainAnnotationRequest,
) -> Result<DomainAnnotationViewResponse, ApiError> {
    let trace_id = generator.client_side();
    let span = tracing::info_span!("command", trace_id = %trace_id, command = "update_crf_domain_annotation");
    let _enter = span.enter();
    tracing::info!("enter");

    let result = TRACE_ID
        .scope(trace_id, async {
            domain_annotation::update(&client, id, body).await
        })
        .await;

    match &result {
        Ok(_) => tracing::info!("success"),
        Err(e) => tracing::error!(error = %e, "failed"),
    }
    result
}

#[tauri::command]
pub async fn delete_crf_domain_annotation(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    id: i64,
) -> Result<(), ApiError> {
    let trace_id = generator.client_side();
    let span = tracing::info_span!("command", trace_id = %trace_id, command = "delete_crf_domain_annotation");
    let _enter = span.enter();
    tracing::info!("enter");

    let result = TRACE_ID
        .scope(trace_id, async {
            domain_annotation::delete(&client, id).await
        })
        .await;

    match &result {
        Ok(_) => tracing::info!("success"),
        Err(e) => tracing::error!(error = %e, "failed"),
    }
    result
}

#[tauri::command]
pub async fn search_crf_domain_annotations_by_version(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    version_id: i64,
    fragment: String,
) -> Result<domain_annotation::DomainAnnotationListResponse, ApiError> {
    let trace_id = generator.client_side();
    let span = tracing::info_span!("command", trace_id = %trace_id, command = "search_crf_domain_annotations_by_version");
    let _enter = span.enter();
    tracing::info!("enter");

    let result = TRACE_ID
        .scope(trace_id, async {
            domain_annotation::search_by_version(&client, version_id, fragment).await
        })
        .await;

    match &result {
        Ok(_) => tracing::info!("success"),
        Err(e) => tracing::error!(error = %e, "failed"),
    }
    result
}