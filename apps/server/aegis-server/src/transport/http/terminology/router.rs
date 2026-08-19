//! Routes for the terminology namespace.
//!
//! Each `routes!` call lists a single handler per HTTP verb. utoipa-axum
//! 0.2 panics if a single `routes!` call receives two handlers that
//! share the same HTTP method, so we split the surface into one call
//! per verb. The resulting `OpenApiRouter<AppState>` is then merged
//! into the API sub-tree in `transport::http::router`.

use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::state::AppState;
use crate::transport::http::terminology::handlers;

/// Compose every handler under `/`; the caller (`transport::http::router`)
/// is responsible for nesting this under `/api/terminology`.
pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        // ---- TerminologyVersion ----
        .routes(routes!(handlers::create_version))
        .routes(routes!(handlers::list_versions))
        .routes(routes!(handlers::get_version_by_id))
        .routes(routes!(handlers::update_version))
        .routes(routes!(handlers::delete_version))
        // ---- CodeList ----
        .routes(routes!(handlers::create_code_list))
        .routes(routes!(handlers::list_code_lists))
        .routes(routes!(handlers::get_code_list_by_id))
        .routes(routes!(handlers::update_code_list))
        .routes(routes!(handlers::delete_code_list))
        .routes(routes!(handlers::search_code_lists))
        // ---- CodeItem ----
        .routes(routes!(handlers::create_code_item))
        .routes(routes!(handlers::batch_create_code_items))
        .routes(routes!(handlers::list_code_items))
        .routes(routes!(handlers::list_code_items_by_version_and_code))
        .routes(routes!(handlers::update_code_item))
        .routes(routes!(handlers::delete_code_item))
        .routes(routes!(handlers::search_code_items))
}
