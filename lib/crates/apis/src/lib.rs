//! `apis` workspace crate.
//!
//! Hosts outbound port traits that adapters (HTTP/gRPC handlers,
//! other backends) consume. Each trait is a self-contained
//! contract: this crate does not depend on any other workspace
//! crate, so any backend can implement the traits by adapting its
//! own types to the ones defined here.
//!
//! # Wire format
//!
//! Every DTO derives `serde::Serialize` + `serde::Deserialize` and
//! serializes with camelCase field names, so the types can be used
//! directly as HTTP request / response bodies. [`user::Role`]
//! serializes lowercase (`"root"` / `"admin"` / `"general"`) to match
//! `Role::as_str()` in the `auth` and `user` crates and the Postgres
//! CHECK constraint.
//!
//! Enabling the `openapi` feature additionally derives
//! `utoipa::ToSchema` on every DTO and on both error enums. The error
//! enums get `ToSchema` only: this crate deliberately decides nothing
//! about HTTP status codes, the error response body, or routing, and
//! never depends on a web framework.
//!
//! # Security
//!
//! DTOs are serializable by default, but not every DTO is safe to
//! route. See the `# Security` note on [`auth`] for the three
//! credential types that carry a password hash.

pub mod auth;
pub mod user;
