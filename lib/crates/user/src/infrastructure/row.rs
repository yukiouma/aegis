// Row -> domain conversion for the SQLx repository.
//
// `UserRow` is the shape returned by `sqlx::query_as` (derived via the
// `FromRow` derive macro from the `sqlx` crate). The mapping from a
// row to a domain `User` is split into two pieces:
//
// - `impl From<UserRow> for User` is the infallible conversion used
//   for rows that have already been validated by the database CHECK
//   constraint. It panics on an unknown role because, by construction,
//   those rows cannot exist in the table.
// - `pub(crate) fn try_from_row` is the fallible variant used in
//   tests, and is available to other infrastructure code that may
//   need to validate a row without panicking.

use std::convert::TryFrom;

use sqlx::FromRow;

use crate::domain::{DomainError, Role, User};

#[derive(Debug, Clone, FromRow)]
pub struct UserRow {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub role: String,
    pub active: bool,
    pub password: String,
}

impl From<UserRow> for User {
    fn from(row: UserRow) -> Self {
        // The CHECK constraint on `role` guarantees that only the
        // three known values reach us, so the `try_from` cannot fail
        // for a row read out of the database.
        let role = Role::try_from(row.role.as_str())
            .expect("database CHECK constraint guarantees a known role");
        User::for_repository(row.id, row.code, row.name, role, row.active, row.password)
    }
}

/// Fallible variant of the `From` conversion. Returns
/// `DomainError::InvalidRole` if the row's role string is not one of
/// the three known values.
pub(crate) fn try_from_row(row: UserRow) -> Result<User, DomainError> {
    let role = Role::try_from(row.role.as_str())?;
    Ok(User::for_repository(
        row.id,
        row.code,
        row.name,
        role,
        row.active,
        row.password,
    ))
}
