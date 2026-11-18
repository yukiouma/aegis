//! Telling the frontend that a release is ready.

use tauri::{AppHandle, Emitter};

/// Announces a downloaded, verified release to the frontend.
///
/// The service depends on this rather than on a Tauri `AppHandle`, so the
/// update policy stays testable without a Tauri runtime; the concrete
/// implementation is the one that emits the event.
pub trait UpdateNotifier: Send + Sync {
    fn notify_ready(&self, version: &str, notes: Option<&str>, date: Option<&str>);
}

/// Emits `aegis:update-ready` to the main window.
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
    fn notify_ready(&self, version: &str, notes: Option<&str>, date: Option<&str>) {
        let payload = serde_json::json!({
            "version": version,
            "notes": notes,
            "date": date,
        });
        if let Err(e) = self.handle.emit_to("main", "aegis:update-ready", payload) {
            tracing::warn!(error = %e, "failed to emit aegis:update-ready");
        }
    }
}
