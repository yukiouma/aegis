use tauri::State;
use trace_id::TraceIdGenerator;

use crate::http::dto::ApiError;
use crate::system::identity::{self, Identity};

/// Returns the OS-level domain user tuple that backs the
/// `loginDomain` request body. Delegates to
/// `system::identity::current` — the single place that maps
/// `windows_utils::get_user_info` into the wire-shape `Identity`.
#[tauri::command]
pub fn get_domain_user_info(generator: State<'_, TraceIdGenerator>) -> Result<Identity, ApiError> {
    let trace_id = generator.client_side();
    tracing::info!(trace_id = %trace_id, "enter");

    let result = identity::current();

    match &result {
        Ok(_) => tracing::info!(trace_id = %trace_id, "success"),
        Err(e) => tracing::error!(trace_id = %trace_id, error = %e, "failed"),
    }
    result
}
