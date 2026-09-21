//! Mission HTTP routes.
//!
//! Mounted at `/api/mission` by the top-level router.
//!
//! URL map (mounted at `/api/mission`):
//!
//! - `POST   /`                                  create_mission
//! - `GET    /{id}`                              get_mission_by_id
//! - `GET    /by-project/{project_code}`         list_missions_by_project (?kind= filter)
//! - `GET    /by-user/{user_code}`               list_missions_by_user
//! - `DELETE /{id}`                              delete_mission
//! - `POST   /{mission_id}/assignee`             add_assignee
//! - `DELETE /{mission_id}/assignee/{aid}`       remove_assignee
//! - `GET    /by-mission/{mission_id}/issues`    list_issues_by_mission (?state= filter)
//! - `POST   /by-mission/{mission_id}/issues`    create_issue
//! - `PATCH  /issues/{issue_id}/state`           patch_issue_state (?state=closed|opened)
//! - `PATCH  /issues/{issue_id}/description`     update_issue_description
//! - `POST   /issues/{issue_id}/comments`        append_comment

use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::state::AppState;
use crate::transport::http::mission::handlers;

/// Compose every handler under `/`; the caller
/// (`transport::http::router`) is responsible for nesting this under
/// `/api/mission`.
pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(handlers::create_mission))
        .routes(routes!(handlers::get_mission_by_id))
        .routes(routes!(handlers::list_missions_by_project))
        .routes(routes!(handlers::list_missions_by_user))
        .routes(routes!(handlers::delete_mission))
        .routes(routes!(handlers::add_assignee))
        .routes(routes!(handlers::remove_assignee))
        .routes(routes!(handlers::list_issues_by_mission))
        .routes(routes!(handlers::create_issue))
        .routes(routes!(handlers::patch_issue_state))
        .routes(routes!(handlers::update_issue_description))
        .routes(routes!(handlers::append_comment))
}
