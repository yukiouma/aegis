//! Telling the frontend that the status changed.

use tauri::{AppHandle, Emitter};

/// Announces a change of the update status to the frontend.
///
/// The service depends on this rather than on a Tauri `AppHandle`, so the
/// update policy stays testable without a Tauri runtime; the concrete
/// implementation is the one that emits the event.
pub trait UpdateNotifier: Send + Sync {
    /// The status moved to another phase.
    ///
    /// The phase name is the whole payload on purpose: the frontend reads
    /// the status itself through the `update_status` command, and this
    /// only says "something moved, ask again". A download reports its
    /// progress without changing the phase, so a download does not flood
    /// this channel.
    fn notify_status_changed(&self, phase: &str);
}

/// Emits `aegis:update-status` to the main window.
///
/// The main window is the only target: the dialog lives in the
/// authenticated shell of the main window, and the workspace windows
/// have their own layout.
pub struct TauriNotifier {
    handle: AppHandle,
}

impl TauriNotifier {
    pub fn new(handle: AppHandle) -> Self {
        Self { handle }
    }
}

impl UpdateNotifier for TauriNotifier {
    fn notify_status_changed(&self, phase: &str) {
        let payload = serde_json::json!({ "phase": phase });
        if let Err(e) = self.handle.emit_to("main", "aegis:update-status", payload) {
            tracing::warn!(error = %e, "failed to emit aegis:update-status");
        }
    }
}
