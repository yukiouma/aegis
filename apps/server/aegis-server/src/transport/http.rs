//! HTTP transport.
//!
//! Hosts the axum `Router` composition, the auth handlers, the wire
//! DTOs, the `ErrorBody` + `ApiError` mapping, the healthz handler,
//! and the utoipa OpenAPI builder. Sub-modules re-export their
//! public surface here so consumers can write
//! `use aegis_server::transport::http::router` or
//! `use aegis_server::transport::router` (the outer re-export).

pub mod auth;
pub mod dto;
pub mod error;
pub mod healthz;
pub mod openapi;
pub mod project;
pub mod router;
pub mod terminology;
pub mod user;

pub use router::router;
