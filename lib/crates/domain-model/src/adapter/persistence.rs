//! Persistence adapter layer.
//!
//! The SQLx runtime API is used throughout (`sqlx::query_as`,
//! `sqlx::query`, `QueryBuilder`) — see the module-level comment
//! in each `*_repo` file. The workspace has no shared
//! `sqlx-data.json` cache, so the compile-time macro API is
//! intentionally avoided.

pub mod postgres;
