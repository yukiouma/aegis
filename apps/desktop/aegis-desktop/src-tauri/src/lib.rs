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
        .plugin(tauri_plugin_dialog::init())
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
            commands::terminology::code_list::get_code_list_by_id,
            commands::terminology::code_list::update_code_list,
            commands::terminology::code_list::delete_code_list,
            commands::terminology::code_item::create_code_item,
            commands::terminology::code_item::list_code_items,
            commands::terminology::code_item::update_code_item,
            commands::terminology::code_item::delete_code_item,
            commands::terminology::code_item::list_code_items_by_version_and_code,
            commands::terminology::import::import_terminology,
            // domain-model
            commands::domain_model::version::create_sdtm_version,
            commands::domain_model::version::list_sdtm_versions,
            commands::domain_model::version::get_sdtm_version_by_id,
            commands::domain_model::version::update_sdtm_version,
            commands::domain_model::version::delete_sdtm_version,
            commands::domain_model::domain::create_sdtm_domain,
            commands::domain_model::domain::list_sdtm_domains_by_version,
            commands::domain_model::domain::get_sdtm_domain_by_id,
            commands::domain_model::domain::update_sdtm_domain,
            commands::domain_model::domain::delete_sdtm_domain,
            commands::domain_model::variable::create_sdtm_variable,
            commands::domain_model::variable::list_sdtm_variables_by_domain,
            commands::domain_model::variable::get_sdtm_variable_by_id,
            commands::domain_model::variable::update_sdtm_variable,
            commands::domain_model::variable::delete_sdtm_variable,
            // crf
            commands::crf::version::list_crf_versions,
            commands::crf::version::import_als,
            commands::crf::form::list_crf_forms_by_version,
            commands::crf::form::create_crf_form,
            commands::crf::form::update_crf_form,
            commands::crf::form::delete_crf_form,
            commands::crf::form::get_crf_form_by_id,
            commands::crf::form::get_crf_form_details,
            commands::crf::form::search_crf_forms_by_version,
            commands::crf::item::list_crf_items_by_form,
            commands::crf::item::get_crf_item_by_id,
            commands::crf::item::update_crf_item,
            commands::crf::item::search_crf_items_by_version,
            commands::crf::option::update_crf_option,
            commands::crf::option::get_crf_option_by_id,
            commands::crf::option::search_crf_options_by_version,
            commands::crf::unit::update_crf_unit,
            commands::crf::unit::get_crf_unit_by_id,
            commands::crf::unit::search_crf_units_by_version,
            commands::crf::annotation::create_crf_annotation,
            commands::crf::annotation::update_crf_annotation,
            commands::crf::annotation::delete_crf_annotation,
            commands::crf::annotation::search_crf_annotations_by_version,
            commands::crf::domain_annotation::create_crf_domain_annotation,
            commands::crf::domain_annotation::list_crf_domain_annotations_by_form,
            commands::crf::domain_annotation::update_crf_domain_annotation,
            commands::crf::domain_annotation::delete_crf_domain_annotation,
            commands::crf::domain_annotation::search_crf_domain_annotations_by_version,
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
            let client = http::client::HttpClient::new(http::config::BASE_URL.to_string(), tokens);
            app.manage(client);
            Ok(())
        })
        .build(tauri::generate_context!())?;

    app.run(|_app_handle, _event| {});
    Ok(())
}
