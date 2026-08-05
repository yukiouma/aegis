//! Adapter layer.
//!
//! Houses the persistence adapters that implement the domain ports,
//! plus outbound-port adapters that adapt the usecase layer to
//! API-facing traits defined in other workspace crates.

pub(crate) mod facade;
pub(crate) mod persistence;

pub use facade::in_memory::AuthServiceImpl;
pub use persistence::postgres::{DomainIdentityRepo, UserCredentialsRepo};
