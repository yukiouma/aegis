//! OpenAPI document builder for the HTTP transport.
//!
//! `#[derive(ToSchema)]` on the wire DTOs plus `#[utoipa::path(...)]`
//! on the handlers (Task 8) supplies the per-type & per-route schema
//! data. This module stitches them together into a single
//! `utoipa::openapi::OpenApi` document so `router()` can serve
//! `/swagger-ui/` and `/api-docs/openapi.json`.
//!
//! Per-route paths are NOT listed here. The
//! `OpenApiRouter::with_openapi(...)` call in `transport::http::router`
//! wires the handlers via `routes!`, and `utoipa-axum` auto-collects
//! each handler's `#[utoipa::path]` into the document with the
//! `nest("/api/auth", ...)` prefix applied. Listing the handlers here
//! as well would double-register them under the un-prefixed relative
//! path (`/login`, `/login-domain`, `/refresh`, `/logout`).
//!
//! The `BearerAuth` security scheme is registered via a [`Modify`]
//! impl (the same pattern the utoipa-axum README uses). Routes that
//! reference it via `security(("BearerAuth" = []))` are then marked
//! as requiring an `Authorization: Bearer <token>` header in the
//! generated document; swagger-ui renders the lock icon on them.

use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};

use crate::transport::http::dto;
use crate::transport::http::error::ErrorBody;

/// OpenAPI document for the aegis-server HTTP transport.
///
/// `paths(...)` is intentionally omitted: the `OpenApiRouter` in
/// `transport::http::router` already registers every handler via
/// `routes!`, and the `nest("/api/auth", ...)` prefix is applied
/// there. The schema registry below still has to be explicit
/// because `utoipa-axum` only collects paths, not schemas. The
/// `SecurityAddon` modifier registers the `BearerAuth` security
/// scheme that `refresh` and `logout` reference via
/// `security(("BearerAuth" = []))`.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "aegis-server API",
        version = "0.1.0",
        description = "HTTP transport for the aegis auth + user services."
    ),
    modifiers(&SecurityAddon),
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
        dto::CreateUserRequest,
        dto::UpdateUserRequest,
        dto::PathCode,
        dto::UserViewResponse,
        dto::UserListResponse,
        dto::CreateUserCredentialRequest,
        dto::UpdateUserCredentialRequest,
        dto::UserCredentialViewResponse,
        dto::RemoveUserCredentialResponse,
        ErrorBody,
    )),
    tags(
        (name = "auth", description = "Authentication endpoints"),
        (name = "system", description = "Operational endpoints"),
        (name = "user", description = "User CRUD endpoints"),
        (name = "user-credential", description = "User credential management endpoints"),
    ),
)]
pub struct ApiDoc;

/// `Modify` impl that registers the `BearerAuth` HTTP security
/// scheme in the OpenAPI document's `components.securitySchemes`.
/// Refresh and logout reference it via
/// `security(("BearerAuth" = []))` from their `#[utoipa::path]`
/// annotations.
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "BearerAuth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
        }
    }
}

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
            "CreateUserRequest",
            "UpdateUserRequest",
            "PathCode",
            "UserViewResponse",
            "UserListResponse",
            "CreateUserCredentialRequest",
            "UpdateUserCredentialRequest",
            "UserCredentialViewResponse",
            "RemoveUserCredentialResponse",
        ] {
            let entry: &RefOr<_> = schemas
                .get(name)
                .unwrap_or_else(|| panic!("missing schema for {name}"));
            let _ = entry; // schema presence is the assertion
        }
    }
}