//! # auth crate
//!
//! Workspace library that implements the `apis::auth::AuthService` port.
//! Three DDD layers (`domain`, `usecase`, `adapter`) plus an
//! `Arc<RwLock<HashMap<String, u32>>>` token-version cache live inside the
//! usecase. Public consumers should `use auth::*;` (see the re-exports
//! below) rather than reach into the sub-modules.

pub mod adapter;
pub mod domain;
pub mod usecase;

// Re-exports for the documented public surface.
pub use domain::{
    DomainError, DomainIdentity, DomainIdentityRepository, Role, UserCredentials,
    UserCredentialsRepository,
};