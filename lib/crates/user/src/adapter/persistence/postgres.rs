//! PostgreSQL-backed implementation of `UserRepository`.
//!
//! This module intentionally uses SQLx's *runtime* query API
//! (`sqlx::query_as` and `sqlx::QueryBuilder`) rather than the
//! `query_as!` / `query!` compile-time-checked macros. The compile-time
//! macros require either a live `DATABASE_URL` or a checked-in
//! `sqlx-data.json` offline metadata cache, neither of which the
//! workspace build currently provides. The runtime API is type-checked
//! against the bound parameters we hand it and lets the build proceed
//! in any environment, at the cost of a small loss in static
//! verification of the SQL itself. The migration test in
//! `tests` (this module) covers the schema content directly.
//!
//! `row` is kept `pub(crate)` and is NOT re-exported at the crate
//! root. `UserRow` is an internal row shape that exists only to bridge
//! SQLx's `FromRow` derive into the domain `User` type; exposing it
//! outside the crate would leak SQLx types onto a public field-access
//! surface. The `user_repo` module is kept private inside the crate
//! for the same reason: the public surface at the crate root
//! re-exports `UserRepo` directly (see `lib.rs`), so external callers
//! never need to name the `user_repo` module. This matches the other
//! two layers (`domain`, `usecase`), which also keep their child
//! modules private.

pub(crate) mod row;
#[cfg(test)]
mod tests;
mod user_repo;

pub use user_repo::UserRepo;