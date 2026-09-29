use tauri::State;
use trace_id::TraceIdGenerator;
use tracing::Instrument;

use crate::http::dto::ApiError;
use crate::system::identity::{self, Identity};

/// Returns the OS-level domain user tuple that backs the
/// `loginDomain` request body. Delegates to
/// `system::identity::current` — the single place that maps
/// `windows_utils::get_user_info` into the wire-shape `Identity`.
#[tauri::command]
pub fn get_domain_user_info(generator: State<'_, TraceIdGenerator>) -> Result<Identity, ApiError> {
    let trace_id = generator.client_side();
    let span =
        tracing::info_span!("command", trace_id = %trace_id, command = "get_domain_user_info");
    let _enter = span.enter();
    tracing::info!("enter");

    let result = identity::current();

    match &result {
        Ok(_) => tracing::info!("success"),
        Err(e) => tracing::error!(error = %e, "failed"),
    }
    result
}