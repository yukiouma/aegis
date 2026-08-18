//! # terminology crate
//!
//! Workspace library providing a SQLx/PostgreSQL-backed DDD
//! repository for CDISC terminology aggregates and an async
//! `TerminologyUsecase` that orchestrates them.

pub mod adapter;
pub mod domain;
pub mod usecase;