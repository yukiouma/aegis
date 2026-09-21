//! PostgreSQL-backed implementations of `MissionRepository`,
//! `AssigneeRepository`, and `MissionIssueRepository`.
//!
//! This module intentionally uses SQLx's *runtime* query API
//! (`sqlx::query_as` and `sqlx::QueryBuilder`) rather than the
//! compile-time-checked macros, mirroring the project / user
//! crates. `MissionRepo::create` opens a transaction so the
//! mission row and every assignee row land atomically; the FK
//! `ON DELETE CASCADE` makes mission deletion a single DELETE.
//! `IssueRepo::append_comment` reads the existing row and
//! rewrites the whole `comments` jsonb array — the Postgres
//! `||` operator is intentionally avoided because it merges
//! by index, not by append.
//!
//! `row` is private to `postgres/`. The `MissionRow` /
//! `AssigneeRow` / `IssueRow` types are NOT re-exported at the
//! crate root.

#![allow(dead_code, unused_imports)]

pub(crate) mod assignee_repo;
pub(crate) mod issue_repo;
pub(crate) mod mission_repo;
pub(crate) mod row;
#[cfg(test)]
mod tests;

pub use assignee_repo::AssigneeRepo;
pub use issue_repo::IssueRepo;
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
