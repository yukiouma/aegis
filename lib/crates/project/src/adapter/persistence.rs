//! Persistence adapters.
//!
//! Storage-specific code lives under `persistence/<backend>/`. At the
//! moment only the PostgreSQL backend exists; the layer boundary
//! re-exports `ProductRepo` and `ProjectRepo` so external callers can
//! name them via the crate root.

pub(crate) mod postgres;
