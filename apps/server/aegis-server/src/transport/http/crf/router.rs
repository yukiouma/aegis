//! Routes for the CRF namespace.
//!
//! Each `routes!` call lists a single handler per HTTP verb.
//! utoipa-axum 0.2 panics if a single `routes!` call receives two
//! handlers that share the same HTTP method, so we split the
//! surface into one call per verb. The resulting
//! `OpenApiRouter<AppState>` is then merged into the API sub-tree
//! in `transport::http::router`.
//!
//!
// URL map (mounted at `/api/crf` by `transport::http::router`):
//!
//! - `POST   /projects/{project_code}/versions`            create_version
//! - `GET    /projects/{project_code}/versions`            list_versions_by_project
//! - `GET    /versions/{id}`                               get_version_by_id
//! - `PATCH  /versions/{id}`                               update_version
//! - `DELETE /versions/{id}`                               delete_version
//! - `POST   /versions/{version_id}/forms`                 create_form
//! - `POST   /versions/{version_id}/forms/bulk`            bulk_create_form
//! - `GET    /versions/{version_id}/forms`                 list_forms_by_version
//! - `GET    /versions/{version_id}/forms/search`          search_forms_by_version
//! - `GET    /forms/{id}`                                  get_form_by_id
//! - `GET    /forms/{id}/details`                          get_form_details
//! - `PATCH  /forms/{id}`                                  update_form
//! - `DELETE /forms/{id}`                                  delete_form
//! - `POST   /forms/{form_id}/items`                       create_item
//! - `GET    /forms/{form_id}/items`                       list_items_by_form
//! - `GET    /forms/{form_id}/items/search`                search_items_by_version
//! - `GET    /items/{id}`                                  get_item_by_id
//! - `PATCH  /items/{id}`                                  update_item
//! - `DELETE /items/{id}`                                  delete_item
//! - `POST   /items/{item_id}/options`                     create_option
//! - `GET    /items/{item_id}/options`                     list_options_by_item
//! - `GET    /items/{item_id}/options/search`              search_options_by_version
//! - `GET    /options/{id}`                                get_option_by_id
//! - `PATCH  /options/{id}`                                update_option
//! - `DELETE /options/{id}`                                delete_option
//! - `POST   /items/{item_id}/units`                       create_unit
//! - `GET    /items/{item_id}/units`                       list_units_by_item
//! - `GET    /items/{item_id}/units/search`                search_units_by_version
//! - `GET    /units/{id}`                                  get_unit_by_id
//! - `PATCH  /units/{id}`                                  update_unit
//! - `DELETE /units/{id}`                                  delete_unit
//! - `POST   /forms/{form_id}/domain-annotations`          create_domain_annotation
//! - `GET    /forms/{form_id}/domain-annotations`          list_domain_annotations_by_form
//! - `GET    /versions/{version_id}/domain-annotations/search`  search_domain_annotations_by_version
//! - `GET    /domain-annotations/{id}`                     get_domain_annotation_by_id
//! - `PATCH  /domain-annotations/{id}`                     update_domain_annotation
//! - `DELETE /domain-annotations/{id}`                     delete_domain_annotation
//! - `POST   /annotations`                                 create_annotation
//! - `GET    /forms/{form_id}/annotations`                 list_annotations_by_form
//! - `GET    /items/{item_id}/annotations`                 list_annotations_by_item
//! - `GET    /options/{option_id}/annotations`             list_annotations_by_option
//! - `GET    /units/{unit_id}/annotations`                 list_annotations_by_unit
//! - `GET    /versions/{version_id}/annotations/search`    search_annotations_by_version
//! - `GET    /annotations/{id}`                            get_annotation_by_id
//! - `PATCH  /annotations/{id}`                            update_annotation
//! - `DELETE /annotations/{id}`                            delete_annotation

use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::state::AppState;
use crate::transport::http::crf::handlers;

/// Compose every handler under `/`; the caller
/// (`transport::http::router`) is responsible for nesting this under
/// `/api/crf`.
pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        // ---- CrfVersion ----
        .routes(routes!(handlers::create_version))
        .routes(routes!(handlers::list_versions_by_project))
        .routes(routes!(handlers::get_version_by_id))
        .routes(routes!(handlers::update_version))
        .routes(routes!(handlers::delete_version))
        // ---- CrfForm ----
        .routes(routes!(handlers::create_form))
        .routes(routes!(handlers::bulk_create_form))
        .routes(routes!(handlers::list_forms_by_version))
        .routes(routes!(handlers::search_forms_by_version))
        .routes(routes!(handlers::get_form_by_id))
        .routes(routes!(handlers::get_form_details))
        .routes(routes!(handlers::update_form))
        .routes(routes!(handlers::delete_form))
        // ---- CrfItem ----
        .routes(routes!(handlers::create_item))
        .routes(routes!(handlers::list_items_by_form))
        .routes(routes!(handlers::search_items_by_version))
        .routes(routes!(handlers::get_item_by_id))
        .routes(routes!(handlers::update_item))
        .routes(routes!(handlers::delete_item))
        // ---- CrfOption ----
        .routes(routes!(handlers::create_option))
        .routes(routes!(handlers::list_options_by_item))
        .routes(routes!(handlers::search_options_by_version))
        .routes(routes!(handlers::get_option_by_id))
        .routes(routes!(handlers::update_option))
        .routes(routes!(handlers::delete_option))
        // ---- CrfUnit ----
        .routes(routes!(handlers::create_unit))
        .routes(routes!(handlers::list_units_by_item))
        .routes(routes!(handlers::search_units_by_version))
        .routes(routes!(handlers::get_unit_by_id))
        .routes(routes!(handlers::update_unit))
        .routes(routes!(handlers::delete_unit))
        // ---- DomainAnnotation ----
        .routes(routes!(handlers::create_domain_annotation))
        .routes(routes!(handlers::list_domain_annotations_by_form))
        .routes(routes!(handlers::search_domain_annotations_by_version))
        .routes(routes!(handlers::get_domain_annotation_by_id))
        .routes(routes!(handlers::update_domain_annotation))
        .routes(routes!(handlers::delete_domain_annotation))
        // ---- Annotation ----
        .routes(routes!(handlers::create_annotation))
        .routes(routes!(handlers::list_annotations_by_form))
        .routes(routes!(handlers::list_annotations_by_item))
        .routes(routes!(handlers::list_annotations_by_option))
        .routes(routes!(handlers::list_annotations_by_unit))
        .routes(routes!(handlers::search_annotations_by_version))
        .routes(routes!(handlers::get_annotation_by_id))
        .routes(routes!(handlers::update_annotation))
        .routes(routes!(handlers::delete_annotation))
}
