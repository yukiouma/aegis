//! Adapter layer.
//!
//! Houses the persistence adapters that implement the domain ports,
//! plus outbound-port adapters that adapt the usecase layer to
//! API-facing traits defined in other workspace crates.

mod facade;
mod persistence;

// AuthServiceImpl lands in Task 9; uncomment when it does.
// pub use facade::in_memory::AuthServiceImpl;
pub use persistence::postgres::{DomainIdentityRepo, UserCredentialsRepo};