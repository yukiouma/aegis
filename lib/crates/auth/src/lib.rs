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

pub use adapter::facade::in_memory::AuthServiceImpl;
pub use adapter::persistence::postgres::{DomainIdentityRepo, UserCredentialsRepo};
pub use domain::{
    DomainError, DomainIdentity, DomainIdentityRepository, Role, UserCredentials,
    UserCredentialsRepository,
};
pub use usecase::{
    AccessTokenView, AuthClaimsView, AuthUsecase, AuthUsecaseConfig, Logout, LogoutAck,
    LoginWithDomainUserInfo, LoginWithPassword, RefreshAccessToken, TokenPairView,
    UsecaseError, VerifyAccessToken,
};