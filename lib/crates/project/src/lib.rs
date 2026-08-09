//! # project crate
//!
//! Workspace library providing a SQLx/PostgreSQL-backed DDD repository
//! for `Product` and `Project` aggregates and an async
//! `ProjectUsecase` that orchestrates them and adapts to the
//! `apis::project::ProjectService` port.

pub mod domain;
