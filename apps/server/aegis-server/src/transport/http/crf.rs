//! CRF (Case Report Form) HTTP feature module.
//!
//! Mounts every method on [`apis::crf::CrfService`] under
//! `/api/crf/*`. Each handler is a thin adapter over
//! [`crate::transport::http::dto`] and the apis DTOs.

pub mod handlers;
pub mod router;
