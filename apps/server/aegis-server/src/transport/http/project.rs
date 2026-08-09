//! Product and project HTTP feature module.
//!
//! The `project` service trait exposes ten operations; the eight
//! code-based operations land at HTTP. Each handler is a thin adapter
//! over [`crate::transport::http::dto`] and the apis DTOs.

pub mod handlers;
pub mod router;
