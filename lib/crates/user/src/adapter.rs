//! Adapter layer.
//!
//! Houses the persistence adapters that implement the
//! `UserRepository` port defined in the domain layer, plus outbound
//! port adapters (e.g. the `UserService` facade) that adapt the
//! usecase layer to API-facing traits defined in other workspace
//! crates.
//!
//! Storage-specific implementations live under
//! `persistence/<backend>/`. At the moment only the PostgreSQL
//! backend exists; the layer boundary re-exports `UserRepo` so
//! external callers can name it via the crate root
//! (`user::UserRepo`). API-facing adapters live under `facade/`.

mod facade;
mod persistence;

pub use facade::UserServiceImpl;
pub use persistence::postgres::UserRepo;
