//! Persistence adapters.
//!
//! The adapter layer keeps the storage-specific code under
//! `persistence/<backend>/` so that adding a second database backend
//! (e.g. `persistence/sqlite`) is purely additive: each backend is a
//! self-contained module that implements the `UserRepository` port
//! from the domain layer.
//!
//! At the moment only the PostgreSQL backend exists. `persistence`
//! itself is `pub(crate)` because callers reach concrete repositories
//! via the layer boundary (`adapter::UserRepo` and the crate
//! root), but the `postgres` child must be `pub` so the re-export at
//! the adapter layer is well-formed.
pub(crate) mod postgres;
