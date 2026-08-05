// PostgreSQL-backed implementation of `UserRepository`.
//
// This module intentionally uses SQLx's *runtime* query API
// (`sqlx::query_as` and `sqlx::QueryBuilder`) rather than the
// `query_as!` / `query!` compile-time-checked macros. The compile-time
// macros require either a live `DATABASE_URL` or a checked-in
// `sqlx-data.json` offline metadata cache, neither of which the
// workspace build currently provides. The runtime API is type-checked
// against the bound parameters we hand it and lets the build proceed
// in any environment, at the cost of a small loss in static
// verification of the SQL itself. The migration test in
// `infrastructure::tests` covers the schema content directly.

use std::convert::TryFrom;

use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::{DomainError, User, UserNew, UserRepository, UserUpdate};

use super::row::UserRow;

/// PostgreSQL SQLSTATE for a unique-violation error.
const SQLSTATE_UNIQUE_VIOLATION: &str = "23505";

pub struct UserRepo {
    pool: PgPool,
}

impl UserRepo {
    /// Build a new `UserRepo` backed by the supplied `sqlx::PgPool`.
    ///
    /// The repository does not open any connections itself; it stores
    /// the pool and acquires connections lazily on each operation.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserRepository for UserRepo {
    async fn create(&self, input: UserNew) -> Result<User, DomainError> {
        const SQL: &str = "INSERT INTO users (code, name, role, active) \
                           VALUES ($1, $2, $3, $4) \
                           RETURNING id, code, name, role, active, created_at, updated_at";
        let row: UserRow = sqlx::query_as(SQL)
            .bind(&input.code)
            .bind(&input.name)
            .bind(input.role.as_str())
            .bind(input.active)
            .fetch_one(&self.pool)
            .await
            .map_err(map_db_error)?;

        row.try_into()
    }

    async fn find_by_id(&self, id: i32) -> Result<User, DomainError> {
        // Use `QueryBuilder` so we can construct the SELECT at
        // runtime without violating sqlx's `SqlSafeStr` bound
        // (which only permits `&'static str` to flow into
        // `query_as`). The column list is a compile-time constant
        // here; nothing user-supplied touches the SQL string.
        let row: UserRow = sqlx::QueryBuilder::new(
            "SELECT id, code, name, role, active, created_at, updated_at \
             FROM users WHERE id = ",
        )
        .push_bind(id)
        .build_query_as::<UserRow>()
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .ok_or(DomainError::NotFound)?;
        row.try_into()
    }

    async fn find_by_code(&self, code: &str) -> Result<User, DomainError> {
        let row: UserRow = sqlx::QueryBuilder::new(
            "SELECT id, code, name, role, active, created_at, updated_at \
             FROM users WHERE code = ",
        )
        .push_bind(code)
        .build_query_as::<UserRow>()
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .ok_or(DomainError::NotFound)?;
        row.try_into()
    }

    async fn list(&self) -> Result<Vec<User>, DomainError> {
        let rows: Vec<UserRow> = sqlx::QueryBuilder::new(
            "SELECT id, code, name, role, active, created_at, updated_at \
             FROM users ORDER BY id",
        )
        .build_query_as::<UserRow>()
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        rows.into_iter().map(User::try_from).collect()
    }

    async fn update(&self, input: UserUpdate) -> Result<User, DomainError> {
        // Build a dynamic UPDATE that only touches the columns whose
        // option is `Some`. `QueryBuilder` is type-checked at the bind
        // sites so we cannot smuggle an unbound value into the SQL
        // string. `updated_at` is set automatically by the
        // `users_set_updated_at` trigger; we do not bind it here.
        let mut qb = sqlx::QueryBuilder::new("UPDATE users SET ");
        let mut first = true;
        let mut separated = |qb: &mut sqlx::QueryBuilder<sqlx::Postgres>| {
            if first {
                first = false;
            } else {
                qb.push(", ");
            }
        };

        if let Some(ref code) = input.code {
            separated(&mut qb);
            qb.push("code = ").push_bind(code);
        }
        if let Some(ref name) = input.name {
            separated(&mut qb);
            qb.push("name = ").push_bind(name);
        }
        if let Some(role) = input.role {
            separated(&mut qb);
            qb.push("role = ").push_bind(role.as_str());
        }
        if let Some(active) = input.active {
            separated(&mut qb);
            qb.push("active = ").push_bind(active);
        }

        if first {
            // Nothing to update; short-circuit and return the existing
            // row, or `NotFound` if the id does not exist.
            return self.find_by_id(input.id).await;
        }

        qb.push(" WHERE id = ").push_bind(input.id);
        qb.push(" RETURNING id, code, name, role, active, created_at, updated_at");

        let row: UserRow = qb
            .build_query_as::<UserRow>()
            .fetch_optional(&self.pool)
            .await
            .map_err(map_db_error)?
            .ok_or(DomainError::NotFound)?;
        row.try_into()
    }
}

/// Map any `sqlx::Error` from a repository query into a `DomainError`.
///
/// * `RowNotFound` becomes `NotFound` (used for `fetch_optional` /
///   `RETURNING` paths that miss the row).
/// * PostgreSQL SQLSTATE `23505` (unique-violation) becomes
///   `DuplicateCode(constraint_name)`. The payload is the constraint
///   name (e.g. `users_code_unique`) rather than the offending code
///   value, because `sqlx` does not surface the bound value here. The
///   usecase layer is the only caller and it surfaces the original
///   `code` alongside the error, so the placeolder string is
///   informational only. The unique-violation branch is a no-op for
///   `SELECT`/`LIST` queries because those operations never produce
///   SQLSTATE `23505`.
/// * Any other database error is wrapped in `Repository` with the
///   driver's message so the rest of the stack can surface it.
fn map_db_error(err: sqlx::Error) -> DomainError {
    match err {
        sqlx::Error::RowNotFound => DomainError::NotFound,
        sqlx::Error::Database(db_err) => {
            if db_err.code().as_deref() == Some(SQLSTATE_UNIQUE_VIOLATION) {
                let constraint = db_err.constraint().unwrap_or("code");
                DomainError::DuplicateCode(format!("(constraint {constraint})"))
            } else {
                DomainError::Repository(db_err.message().to_string())
            }
        }
        other => DomainError::Repository(other.to_string()),
    }
}