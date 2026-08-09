//! PostgreSQL-backed implementations of `ProductRepository` and
//! `ProjectRepository`.
//!
//! This module intentionally uses SQLx's *runtime* query API
//! (`sqlx::query_as` and `sqlx::QueryBuilder`) rather than the
//! compile-time-checked macros, mirroring the user crate.
//! `ProjectRepo::create` / `update` open a transaction so the project
//! row and the `project_members` rows land atomically.
//!
//! `row` is `pub(crate)` and is NOT re-exported at the crate root.

pub(crate) mod product_repo;
pub(crate) mod project_repo;
pub(crate) mod row;
#[cfg(test)]
mod tests;

// `pub use` is re-enabled after the implementation lands in Step 7/8.
pub use product_repo::ProductRepo;
pub use project_repo::ProjectRepo;
