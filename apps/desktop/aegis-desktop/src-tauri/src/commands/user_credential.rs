use tauri::State;
use trace_id::TraceIdGenerator;

use crate::http::client::{HttpClient, TRACE_ID};
use crate::http::dto::ApiError;
use crate::http::user_credential::{
    self, RegisterUserRequest, RegisterUserResponse, UpdateUserCredentialRequest,
    UserCredentialViewResponse,
};

#[tauri::command]
pub async fn register_user(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    user_code: String,
    user_name: String,
    domain_name: String,
    hostname: String,
    sid: String,
    password: String,
) -> Result<RegisterUserResponse, ApiError> {
    let trace_id = generator.client_side();
    let span = tracing::info_span!("command", trace_id = %trace_id, command = "register_user");
    let _enter = span.enter();
    tracing::info!("enter");

    let result = TRACE_ID
        .scope(trace_id, async {
            user_credential::register(
                &client,
                RegisterUserRequest {
                    user_code,
                    user_name,
                    domain_name,
                    hostname,
                    sid,
                    password,
                },
            )
            .await
        })
        .await;

    match &result {
        Ok(_) => tracing::info!("success"),
        Err(e) => tracing::error!(error = %e, "failed"),
    }
    result
}

#[tauri::command]
pub async fn update_user_credential(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    user_code: String,
    password: Option<String>,
) -> Result<UserCredentialViewResponse, ApiError> {
    let trace_id = generator.client_side();
    let span =
        tracing::info_span!("command", trace_id = %trace_id, command = "update_user_credential");
    let _enter = span.enter();
    tracing::info!("enter");

    let result = TRACE_ID
        .scope(trace_id, async {
            user_credential::update(
                &client,
                UpdateUserCredentialRequest {
                    user_code,
                    password,
                },
            )
            .await
        })
        .await;

    match &result {
        Ok(_) => tracing::info!("success"),
        Err(e) => tracing::error!(error = %e, "failed"),
    }
    result
}
