//! HTTP transport for the user CRUD namespace.
//!
//! Hosts the four `apis::user::UserService` handlers under
//! `/api/user/*`. Every handler requires a valid
//! `Authorization: Bearer <token>` header — verification is done by
//! the [`AuthClaims`](crate::transport::http::auth::middleware::AuthClaims)
//! extractor from the sibling `auth/` module; no role-based
//! authorization is enforced at this stage.
//!
//! The router here is an `OpenApiRouter<AppState>` composed from the
//! per-handler `routes!()` registrations; the top-level
//! [`mod@crate::transport::http::router`] nests it under `/api/user`.

pub mod handlers;
pub mod router;

pub use router::router;
