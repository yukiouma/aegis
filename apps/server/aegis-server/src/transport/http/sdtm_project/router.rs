//! Routes for the sdtm-project namespace.
//!
//! Each `routes!` call lists a single handler per HTTP verb. utoipa-axum
//! 0.2 panics if a single `routes!` call receives two handlers that
//! share the same HTTP method, so we split the surface into one call
//! per verb. The resulting `OpenApiRouter<AppState>` is then merged
//! into the API sub-tree in `transport::http::router`.

use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::state::AppState;
use crate::transport::http::sdtm_project::handlers;

/// Compose every handler under `/`; the caller (`transport::http::router`)
/// is responsible for nesting this under `/api/sdtm-project`.
pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        // ---- SdtmProjectVersion ----
        .routes(routes!(handlers::list_versions_by_project))
        .routes(routes!(handlers::create_version))
        .routes(routes!(handlers::get_version_by_id))
        .routes(routes!(handlers::update_version))
        .routes(routes!(handlers::delete_version))
        // ---- SdtmProjectDomain ----
        .routes(routes!(handlers::create_domain))
        .routes(routes!(handlers::get_domain_by_id))
        .routes(routes!(handlers::list_domains_by_version))
        .routes(routes!(handlers::update_domain))
        .routes(routes!(handlers::delete_domain))
        // ---- SdtmProjectDomainOverride ----
        .routes(routes!(handlers::create_domain_override))
        .routes(routes!(handlers::get_domain_override))
        .routes(routes!(handlers::update_domain_override))
        .routes(routes!(handlers::delete_domain_override))
        // ---- SdtmProjectVariable ----
        .routes(routes!(handlers::create_variable))
        .routes(routes!(handlers::get_variable_by_id))
        .routes(routes!(handlers::list_variables_by_domain))
        .routes(routes!(handlers::update_variable))
        .routes(routes!(handlers::delete_variable))
        // ---- SdtmProjectVariableOverride ----
        .routes(routes!(handlers::create_variable_override))
        .routes(routes!(handlers::get_variable_override))
        .routes(routes!(handlers::update_variable_override))
        .routes(routes!(handlers::delete_variable_override))
}
