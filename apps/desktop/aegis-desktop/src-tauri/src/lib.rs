// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use std::sync::Arc;

use tauri::Manager;
use tauri_plugin_store::StoreExt;

mod commands;
mod http;
mod system;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            // auth
            commands::auth::login,
            commands::auth::login_domain,
            commands::auth::is_logged_in,
            commands::auth::refresh,
            commands::auth::logout,
            // identity
            commands::identity::get_domain_user_info,
            // user-credential
            commands::user_credential::register_user,
            commands::user_credential::update_user_credential,
            // user
            commands::user::create_user,
            commands::user::list_users,
            commands::user::get_user_by_code,
            commands::user::current_user,
            commands::user::update_user,
            // project
            commands::project::create_project,
            commands::project::list_projects,
            commands::project::get_project_by_code,
            commands::project::update_project,
            // terminology
            commands::terminology::version::create_terminology_version,
            commands::terminology::version::list_terminology_versions,
            commands::terminology::version::get_terminology_version_by_id,
            commands::terminology::version::update_terminology_version,
            commands::terminology::version::delete_terminology_version,
            commands::terminology::code_list::create_code_list,
            commands::terminology::code_list::list_code_lists,
            commands::terminology::code_list::update_code_list,
            commands::terminology::code_list::delete_code_list,
            commands::terminology::code_list::search_code_lists,
            commands::terminology::code_item::create_code_item,
            commands::terminology::code_item::list_code_items,
            commands::terminology::code_item::update_code_item,
            commands::terminology::code_item::delete_code_item,
            commands::terminology::code_item::search_code_items,
            // health
            commands::healthz::healthz,
            // legacy greet (kept for the existing test)
            greet,
        ])
        .setup(|app| {
            let store = app
                .store("auth.bin")
                .map_err(|e| format!("failed to open auth.bin store: {e}"))?;
            let tokens = Arc::new(http::client::TauriStore::new(store));
            let client = http::client::HttpClient::new(
                http::config::BASE_URL.to_string(),
                tokens,
            );
            app.manage(client);
            Ok(())
        })
        .build(tauri::generate_context!())?;

    app.run(|_app_handle, _event| {});
    Ok(())
}
