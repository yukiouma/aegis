//! Infrastructure layer.
//!
//! Houses the persistence adapters that implement the
//! `UserRepository` port defined in the domain layer. Storage-specific
//! implementations live under `persistence/<backend>/`. At the moment
//! only the PostgreSQL backend exists; the layer boundary re-exports
//! `UserRepo` so external callers can name it via the crate root
//! (`user::UserRepo`).

mod persistence;

pub use persistence::postgres::UserRepo;
