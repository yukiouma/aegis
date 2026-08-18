//! Terminology HTTP feature module.
//!
//! Mounts every method on [`apis::terminology::TerminologyService`]
//! under `/api/terminology/*`. Each handler is a thin adapter over
//! [`crate::transport::http::dto`] and the apis DTOs.

pub mod handlers;
pub mod router;
