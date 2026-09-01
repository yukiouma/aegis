//! PostgreSQL-backed implementations of `MissionRepository` and
//! `AssigneeRepository`.
//!
//! This module intentionally uses SQLx's *runtime* query API
//! (`sqlx::query_as` and `sqlx::QueryBuilder`) rather than the
//! compile-time-checked macros, mirroring the project / user
//! crates. `MissionRepo::create` opens a transaction so the
//! mission row and every assignee row land atomically; the FK
//! `ON DELETE CASCADE` makes mission deletion a single DELETE.
//!
//! `row` is private to `postgres/`. The `MissionRow` /
//! `AssigneeRow` types are NOT re-exported at the crate root.
//!
//! The struct `new` constructors and the `MissionRow.mission_id`
//! / `AssigneeRow` fields read here only by `TryFrom` impls
//! are intentionally unreferenced until the usecase + facade
//! layer (Task 4) wires them up.

#![allow(dead_code, unused_imports)]

pub(crate) mod assignee_repo;
pub(crate) mod mission_repo;
pub(crate) mod row;
#[cfg(test)]
mod tests;

pub use assignee_repo::AssigneeRepo;
pub use mission_repo::MissionRepo;

use crate::domain::DomainError;

/// Map a `sqlx::Error` into the domain error taxonomy.
///
/// `RowNotFound` → `NotFound`. `Database` with SQLSTATE `23505`
/// (unique violation) is NOT mapped here — the call sites that
/// care about uniqueness (`MissionRepo::create`,
/// `MissionRepo::insert_assignee`, `AssigneeRepo::add`) handle
/// that variant themselves so they can build the structured
/// `DuplicateMission` / `DuplicateAssignee` variants with the
/// right context. Everything else → `Repository(driver_message)`.
fn map_db_error(e: sqlx::Error) -> DomainError {
    match e {
        sqlx::Error::RowNotFound => DomainError::NotFound,
        other => DomainError::Repository(other.to_string()),
    }
}
