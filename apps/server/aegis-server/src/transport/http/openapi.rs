//! OpenAPI document builder for the HTTP transport.
//!
//! `#[derive(ToSchema)]` on the wire DTOs plus `#[utoipa::path(...)]`
//! on the handlers (Task 8) supplies the per-type & per-route schema
//! data. This module stitches them together into a single
//! `utoipa::openapi::OpenApi` document so `router()` can serve
//! `/swagger-ui/` and `/api-docs/openapi.json`.
//!
//! Each route will subsequently be registered under
//! `#[utoipa::path(...)]` next to its handler. The full
//! `#[derive(OpenApi)]` struct lives here so the route attributes
//! stay close to the handler bodies.

use utoipa::OpenApi;

use crate::transport::http::dto;
use crate::transport::http::error::ErrorBody;

/// OpenAPI document for the aegis-server HTTP transport.
///
/// New routes must be added to `paths = [...]` here. New DTOs and
/// `ErrorBody` are picked up automatically through `components.schemas`.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "aegis-server API",
        version = "0.1.0",
        description = "HTTP transport for the aegis auth + user services."
    ),
    paths(
        // Each route's `#[utoipa::path]` is bound here by path once
        // Task 8 lands. healthz stays inline; the auth endpoints
        // (login / login_domain / refresh / logout) are added in
        // Task 8 alongside their handlers.
    ),
    components(schemas(
        dto::LoginRequest,
        dto::LoginDomainRequest,
        dto::RefreshRequest,
        dto::LogoutRequest,
        dto::TokenPairResponse,
        dto::AccessTokenResponse,
        dto::LogoutResponse,
        dto::AuthClaimsResponse,
        dto::Role,
        ErrorBody,
    )),
    tags(
        (name = "auth", description = "Authentication endpoints"),
        (name = "system", description = "Operational endpoints"),
    ),
)]
pub struct ApiDoc;

/// Build the OpenAPI document used by both the swagger-ui and the
/// `/api-docs/openapi.json` endpoint.
pub fn openapi() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
}

#[cfg(test)]
mod tests {
    use super::*;
    use utoipa::openapi::RefOr;

    #[test]
    fn openapi_reports_expected_title_and_version() {
        let doc = openapi();
        assert_eq!(doc.info.title, "aegis-server API");
        assert_eq!(doc.info.version, "0.1.0");
    }

    #[test]
    fn openapi_registers_error_body_schema() {
        let doc = openapi();
        let schemas = &doc
            .components
            .as_ref()
            .expect("components should be present")
            .schemas;
        assert!(
            schemas.contains_key("ErrorBody"),
            "ErrorBody schema missing: {:?}",
            schemas.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn openapi_registers_wire_dto_schemas() {
        let doc = openapi();
        let schemas = &doc
            .components
            .as_ref()
            .unwrap()
            .schemas;
        for name in [
            "LoginRequest",
            "LoginDomainRequest",
            "RefreshRequest",
            "LogoutRequest",
            "TokenPairResponse",
            "AccessTokenResponse",
            "LogoutResponse",
            "AuthClaimsResponse",
            "Role",
        ] {
            let entry: &RefOr<_> = schemas
                .get(name)
                .unwrap_or_else(|| panic!("missing schema for {name}"));
            let _ = entry; // schema presence is the assertion
        }
    }
}