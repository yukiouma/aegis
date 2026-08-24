//! Routes for the domain-model namespace.
//!
//! Each `routes!` call lists a single handler per HTTP verb. utoipa-axum
//! 0.2 panics if a single `routes!` call receives two handlers that
//! share the same HTTP method, so we split the surface into one call
//! per verb. The resulting `OpenApiRouter<AppState>` is then merged
//! into the API sub-tree in `transport::http::router`.

use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::state::AppState;
use crate::transport::http::domain_model::handlers;

/// Compose every handler under `/`; the caller (`transport::http::router`)
/// is responsible for nesting this under `/api/domain-model`.
pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        // ---- SdtmVersion ----
        .routes(routes!(handlers::list_versions))
        .routes(routes!(handlers::create_version))
        .routes(routes!(handlers::update_version))
        .routes(routes!(handlers::delete_version))
        // ---- SdtmDomain ----
        .routes(routes!(handlers::create_domain))
        .routes(routes!(handlers::get_domain_by_id))
        .routes(routes!(handlers::list_domains_by_version))
        .routes(routes!(handlers::update_domain))
        .routes(routes!(handlers::delete_domain))
        // ---- SdtmVariable ----
        .routes(routes!(handlers::create_variable))
        .routes(routes!(handlers::get_variable_by_id))
        .routes(routes!(handlers::list_variables_by_domain))
        .routes(routes!(handlers::update_variable))
        .routes(routes!(handlers::delete_variable))
}