//! Product and project HTTP routes.
//!
//! The feature owns two resource routers so the top-level router can
//! mount them at independent URL prefixes (`/api/product` and
//! `/api/project`).

use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::state::AppState;
use crate::transport::http::project::handlers;

/// Build the two resource routers that back `/api/product` and
/// `/api/project`. Each handler is registered in its own
/// `routes!(...)` call because utoipa-axum 0.2 panics on multiple
/// same-method handlers in a single call.
pub fn routers() -> (OpenApiRouter<AppState>, OpenApiRouter<AppState>) {
    let product = OpenApiRouter::new()
        .routes(routes!(handlers::create_product))
        .routes(routes!(handlers::list_products))
        .routes(routes!(handlers::get_product_by_code))
        .routes(routes!(handlers::update_product));

    let project = OpenApiRouter::new()
        .routes(routes!(handlers::create_project))
        .routes(routes!(handlers::list_projects))
        .routes(routes!(handlers::get_project_by_code))
        .routes(routes!(handlers::update_project));

    (product, project)
}
