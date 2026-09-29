use tauri::State;
use trace_id::TraceIdGenerator;

use crate::http::auth::{self, LoginRequest};
use crate::http::client::{HttpClient, TRACE_ID};
use crate::http::dto::ApiError;

#[tauri::command]
pub async fn login(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    code: String,
    password: String,
) -> Result<(), ApiError> {
    let trace_id = generator.client_side();
    tracing::info!(trace_id = %trace_id, "enter");

    let result = TRACE_ID
        .scope(trace_id.clone(), async {
            auth::login(&client, LoginRequest { code, password }).await
        })
        .await;

    match &result {
        Ok(_) => tracing::info!(trace_id = %trace_id, "success"),
        Err(e) => tracing::error!(trace_id = %trace_id, error = %e, "failed"),
    }
    result
}

#[tauri::command]
pub async fn login_domain(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
) -> Result<(), ApiError> {
    let trace_id = generator.client_side();
    tracing::info!(trace_id = %trace_id, "enter");

    let result = TRACE_ID
        .scope(trace_id.clone(), async { auth::login_domain(&client).await })
        .await;

    match &result {
        Ok(_) => tracing::info!(trace_id = %trace_id, "success"),
        Err(e) => tracing::error!(trace_id = %trace_id, error = %e, "failed"),
    }
    result
}

#[tauri::command]
pub async fn is_logged_in(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
) -> Result<bool, ApiError> {
    let trace_id = generator.client_side();
    tracing::info!(trace_id = %trace_id, "enter");

    let result = client
        .tokens()
        .access_token()
        .await
        .map(|opt| opt.is_some());

    match &result {
        Ok(_) => tracing::info!(trace_id = %trace_id, "success"),
        Err(e) => tracing::error!(trace_id = %trace_id, error = %e, "failed"),
    }
    result
}

#[tauri::command]
pub async fn refresh(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
) -> Result<(), ApiError> {
    let trace_id = generator.client_side();
    tracing::info!(trace_id = %trace_id, "enter");

    let result = TRACE_ID
        .scope(trace_id.clone(), async { auth::refresh(&client).await })
        .await;

    match &result {
        Ok(_) => tracing::info!(trace_id = %trace_id, "success"),
        Err(e) => tracing::error!(trace_id = %trace_id, error = %e, "failed"),
    }
    result
}

#[tauri::command]
pub async fn logout(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
) -> Result<(), ApiError> {
    let trace_id = generator.client_side();
    tracing::info!(trace_id = %trace_id, "enter");

    let result = TRACE_ID
        .scope(trace_id.clone(), async { auth::logout(&client).await })
        .await;

    match &result {
        Ok(_) => tracing::info!(trace_id = %trace_id, "success"),
        Err(e) => tracing::error!(trace_id = %trace_id, error = %e, "failed"),
    }
    result
}
