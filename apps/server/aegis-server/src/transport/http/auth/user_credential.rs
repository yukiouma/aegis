//! HTTP sub-router for the user-credential management namespace.
//!
//! Houses the handler that translates to / from the
//! [`apis::auth::AuthService`] user-credential methods
//! ([`find_user_credential_by_code`](apis::auth::AuthService::find_user_credential_by_code),
//! [`update_user_credential`](apis::auth::AuthService::update_user_credential),
//! [`remove_user_credential`](apis::auth::AuthService::remove_user_credential)).
//!
//! Composed into the parent `/api/auth/*` router via
//! [`crate::transport::http::auth::router`].
//!
//! Live under `/api/auth/user-credential` as the URL surface — the
//! apis trait names these "user_credential" methods, and the
//! singular noun matches the per-user scope (each user owns at most
//! one credential row).
//!
//! **Only `PATCH` is exposed at HTTP.** Credential creation happens
//! out of band; the apis `AuthService::create_user_credential`
//! method does not exist.

pub mod handlers;
pub mod router;

pub use router::router;
