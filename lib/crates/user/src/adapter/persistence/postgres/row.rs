// Row -> domain conversion for the SQLx repository.
//
// `UserRow` is the shape returned by `sqlx::query_as` (derived via the
// `FromRow` derive macro from the `sqlx` crate). The mapping from a
// row to a domain `User` is a single `TryFrom` impl: the CHECK
// constraint on `role` guarantees that only the three known values
// reach us from the database, so the repository can call `try_into()`
// and surface the `InvalidRole` error path as defensive belt-and-braces
// even though the conversion cannot fail in practice.
//
// `created_at` and `updated_at` are populated by the database
// (DEFAULT NOW() on insert, a BEFORE UPDATE trigger on update) so the
// row never carries stale timestamps.
//
// The module is `pub(crate)` (see `persistence/postgres/mod.rs`) and `UserRow`
// is NOT re-exported at the crate root: it is an internal bridge
// between SQLx's `FromRow` derive and the domain `User` type, and the
// only caller is the repository itself.

use std::convert::TryFrom;

use chrono::{DateTime, Utc};
use sqlx::FromRow;

use crate::domain::{DomainError, Role, User};

#[derive(Clone, FromRow)]
pub struct UserRow {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub role: String,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<UserRow> for User {
    type Error = DomainError;

    fn try_from(row: UserRow) -> Result<Self, Self::Error> {
        // `Role::try_from` is the single source of truth for role
        // validation. The database CHECK constraint guarantees a
        // known role, so the `Err` branch is unreachable in practice;
        // the repository's `?` keeps the conversion path uniform.
        let role = Role::try_from(row.role.as_str())?;
        Ok(User::for_repository(
            row.id,
            row.code,
            row.name,
            role,
            row.active,
            row.created_at,
            row.updated_at,
        ))
    }
}
