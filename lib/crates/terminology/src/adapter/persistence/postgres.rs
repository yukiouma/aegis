//! PostgreSQL-backed implementations of the three terminology
//! repository ports. Each repo uses SQLx's *runtime* query API
//! (`sqlx::query_as`, `QueryBuilder`) rather than the compile-time
//! macros, mirroring the user / project crates.

pub mod terminology_version_repo;

pub use terminology_version_repo::TerminologyVersionRepo;