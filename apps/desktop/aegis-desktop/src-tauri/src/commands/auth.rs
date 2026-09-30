use logging_utils::TraceIdGenerator;
use tauri::State;
use tracing::Instrument;

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
    let span = tracing::info_span!("command", trace_id = %trace_id, command = "login");
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(trace_id, async {
                auth::login(&client, LoginRequest { code, password }).await
            })
            .await;
        match &result {
            Ok(_) => tracing::info!("success"),
            Err(e) => tracing::error!(error = %e, "failed"),
        }
        result
    }
    .instrument(span)
    .await
}

#[tauri::command]
pub async fn login_domain(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
) -> Result<(), ApiError> {
    let trace_id = generator.client_side();
    let span = tracing::info_span!("command", trace_id = %trace_id, command = "login_domain");
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(trace_id, async { auth::login_domain(&client).await })
            .await;
        match &result {
            Ok(_) => tracing::info!("success"),
            Err(e) => tracing::error!(error = %e, "failed"),
        }
        result
    }
    .instrument(span)
    .await
}

#[tauri::command]
pub async fn is_logged_in(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
) -> Result<bool, ApiError> {
    let trace_id = generator.client_side();
    let span = tracing::info_span!("command", trace_id = %trace_id, command = "is_logged_in");
    async move {
        tracing::info!("enter");
        let result = client
            .tokens()
            .access_token()
            .await
            .map(|opt| opt.is_some());
        match &result {
            Ok(_) => tracing::info!("success"),
            Err(e) => tracing::error!(error = %e, "failed"),
        }
        result
    }
    .instrument(span)
    .await
}

#[tauri::command]
pub async fn refresh(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
) -> Result<(), ApiError> {
    let trace_id = generator.client_side();
    let span = tracing::info_span!("command", trace_id = %trace_id, command = "refresh");
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(trace_id, async { auth::refresh(&client).await })
            .await;
        match &result {
            Ok(_) => tracing::info!("success"),
            Err(e) => tracing::error!(error = %e, "failed"),
        }
        result
    }
    .instrument(span)
    .await
}

#[tauri::command]
pub async fn logout(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
) -> Result<(), ApiError> {
    let trace_id = generator.client_side();
    let span = tracing::info_span!("command", trace_id = %trace_id, command = "logout");
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(trace_id, async { auth::logout(&client).await })
            .await;
        match &result {
            Ok(_) => tracing::info!("success"),
            Err(e) => tracing::error!(error = %e, "failed"),
        }
        result
    }
    .instrument(span)
    .await
}
