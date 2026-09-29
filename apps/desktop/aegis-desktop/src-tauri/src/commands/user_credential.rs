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
    tracing::info!(trace_id = %trace_id, "enter");

    let result = TRACE_ID
        .scope(trace_id.clone(), async {
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
        Ok(_) => tracing::info!(trace_id = %trace_id, "success"),
        Err(e) => tracing::error!(trace_id = %trace_id, error = %e, "failed"),
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
    tracing::info!(trace_id = %trace_id, "enter");

    let result = TRACE_ID
        .scope(trace_id.clone(), async {
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
        Ok(_) => tracing::info!(trace_id = %trace_id, "success"),
        Err(e) => tracing::error!(trace_id = %trace_id, error = %e, "failed"),
    }
    result
}
