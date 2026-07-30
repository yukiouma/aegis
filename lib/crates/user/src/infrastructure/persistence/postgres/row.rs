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
// only callers are the repository itself and the in-crate test
// suite. The `password` field is `pub(crate)` for the same reason:
// it must stay accessible to the repository and the in-crate test
// suite, but not to anyone reaching for `UserRow` from outside the
// crate. The manual `Debug` impl below ensures the hash never leaks
// through `Debug` formatting even inside the crate.

use std::convert::TryFrom;
use std::fmt;

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
    pub(crate) password: String,
}

/// Hand-rolled `Debug` impl that intentionally redacts the `password`
/// column. Matches the redaction policy applied to `User` so neither
/// the domain aggregate nor its SQLx row shape leaks the hash through
/// `Debug` output (e.g. logs, panics, error chains).
impl fmt::Debug for UserRow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UserRow")
            .field("id", &self.id)
            .field("code", &self.code)
            .field("name", &self.name)
            .field("role", &self.role)
            .field("active", &self.active)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .field("password", &"<redacted>")
            .finish()
    }
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
            row.password,
        ))
    }
}
