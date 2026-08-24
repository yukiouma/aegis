//! HTTP transport for the `domain_model` namespace.
//!
//! Handlers are thin adapters that translate wire DTOs (from
//! `crate::transport::http::dto`) into the apis DTOs and dispatch
//! to `AppState::domain_model`. DomainModelApiError is funnelled
//! through `ApiError::from` so each route returns
//! `Result<Json<T>, ApiError>`.

pub mod handlers;
pub mod router;