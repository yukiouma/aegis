//! PostgreSQL-backed implementations of `UserCredentialsRepository` and
//! `DomainIdentityRepository`.
//!
//! This module intentionally uses SQLx's *runtime* query API
//! (`sqlx::query_as` and `sqlx::QueryBuilder`) rather than the
//! `query_as!` / `query!` compile-time-checked macros. The compile-time
//! macros require either a live `DATABASE_URL` or a checked-in
//! `sqlx-data.json` offline metadata cache, neither of which the
//! workspace build currently provides. The runtime API is type-checked
//! against the bound parameters we hand it and lets the build proceed
//! in any environment, at the cost of a small loss in static
//! verification of the SQL itself. The migration tests in `tests`
//! cover the schema content directly.
//!
//! `row` is kept private and `UserRow` / `DomainIdentityRow` are NOT
//! re-exported at the crate root. They are internal bridges between
//! SQLx's `FromRow` derive and the domain types, and the only callers
//! are the repository implementations in this module.

mod auth_repo;
mod row;

#[cfg(test)]
mod tests;

pub use auth_repo::{DomainIdentityRepo, UserCredentialsRepo};
