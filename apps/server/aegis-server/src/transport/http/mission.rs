//! Mission HTTP feature module.
//!
//! The `MissionService` trait exposes seven operations; six land
//! at HTTP. Each handler is a thin adapter over
//! [`crate::transport::http::dto`] and the apis DTOs.

pub mod handlers;
pub mod router;
