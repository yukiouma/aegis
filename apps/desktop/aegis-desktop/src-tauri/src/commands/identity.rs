use crate::system::identity::{self, Identity};

/// Returns the OS-level domain user tuple that backs the
/// `loginDomain` request body. Delegates to
/// `system::identity::current` — the single place that maps
/// `windows_utils::get_user_info` into the wire-shape `Identity`.
#[tauri::command]
pub fn get_domain_user_info() -> Result<Identity, String> {
    identity::current()
}