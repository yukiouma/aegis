//! Tauri command shims for the release-update path.
//!
//! Unlike the other command modules there is no `http/` twin: the
//! updater talks to the release server, not to `aegis-server`. The
//! policy lives in `crate::updater`.

use tauri::State;
use tracing::Instrument;

use crate::http::dto::ApiError;
use crate::updater::service::UpdateService;
use crate::updater::status::UpdateStatusDto;

/// The current update status.
#[tauri::command]
pub async fn update_status(service: State<'_, UpdateService>) -> Result<UpdateStatusDto, ApiError> {
    let span = tracing::info_span!("command", command = "update_status");
    async move {
        tracing::info!("enter");
        let result = Ok(service.status());
        tracing::info!("success");
        result
    }
    .instrument(span)
    .await
}

/// Runs a check now, instead of waiting for the hourly one.
#[tauri::command]
pub async fn check_for_updates(
    service: State<'_, UpdateService>,
) -> Result<UpdateStatusDto, ApiError> {
    let span = tracing::info_span!("command", command = "check_for_updates");
    async move {
        tracing::info!("enter");
        let result = Ok(service.check_now().await);
        tracing::info!("success");
        result
    }
    .instrument(span)
    .await
}

/// Installs the downloaded release. On Windows this never returns — the
/// plugin launches the installer and exits the process.
#[tauri::command]
pub async fn install_ready_update(service: State<'_, UpdateService>) -> Result<(), ApiError> {
    let span = tracing::info_span!("command", command = "install_ready_update");
    async move {
        tracing::info!("enter");
        match service.install_ready().await {
            Ok(()) => {
                tracing::info!("success");
                Ok(())
            }
            Err(e) => {
                tracing::error!(error = %e, "failed");
                Err(e.into())
            }
        }
    }
    .instrument(span)
    .await
}
