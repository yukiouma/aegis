//! # domain-model crate
//!
//! Workspace library providing a SQLx/PostgreSQL-backed DDD
//! repository for the CDISC SDTM domain model aggregates
//! and an async `DomainModelUsecase` that orchestrates them.

pub mod adapter;
pub mod domain;
pub mod usecase;
