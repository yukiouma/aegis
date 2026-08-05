//! Persistence adapters.
//!
//! `persistence` itself is `pub(crate)` because callers reach concrete
//! repositories via the layer boundary (`adapter::UserCredentialsRepo`,
//! `adapter::DomainIdentityRepo`, and the crate root). The `postgres`
//! child must be `pub` so the re-exports at the adapter layer are
//! well-formed.

pub(crate) mod postgres;