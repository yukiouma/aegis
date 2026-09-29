use tauri::State;
use trace_id::TraceIdGenerator;
use tracing::Instrument;

use crate::http::client::{HttpClient, TRACE_ID};
use crate::http::dto::ApiError;
use crate::http::healthz;

#[tauri::command]
pub async fn healthz(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
) -> Result<String, ApiError> {
    let trace_id = generator.client_side();
    let span =
        tracing::info_span!("command", trace_id = %trace_id, command = "healthz");
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(trace_id, async { healthz::ping(&client).await })
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