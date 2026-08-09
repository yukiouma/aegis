use std::convert::TryFrom;

use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::{DomainError, Product, ProductNew, ProductRepository, ProductUpdate};

use super::row::ProductRow;

/// PostgreSQL SQLSTATE for a unique-violation error.
const SQLSTATE_UNIQUE_VIOLATION: &str = "23505";

pub struct ProductRepo {
    pool: PgPool,
}

impl ProductRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ProductRepository for ProductRepo {
    async fn create(&self, input: ProductNew) -> Result<Product, DomainError> {
        const SQL: &str = "INSERT INTO products (code, name, description, active) \
             VALUES ($1, $2, $3, $4) \
             RETURNING id, code, name, description, active, created_at, updated_at";
        let row: ProductRow = sqlx::query_as(SQL)
            .bind(&input.code)
            .bind(&input.name)
            .bind(&input.description)
            .bind(true)
            .fetch_one(&self.pool)
            .await
            .map_err(map_db_error)?;
        row.try_into()
    }

    async fn find_by_id(&self, id: i32) -> Result<Product, DomainError> {
        let row: ProductRow = sqlx::QueryBuilder::new(
            "SELECT id, code, name, description, active, created_at, updated_at \
             FROM products WHERE id = ",
        )
        .push_bind(id)
        .build_query_as::<ProductRow>()
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .ok_or(DomainError::NotFound)?;
        row.try_into()
    }

    async fn find_by_code(&self, code: &str) -> Result<Product, DomainError> {
        let row: ProductRow = sqlx::QueryBuilder::new(
            "SELECT id, code, name, description, active, created_at, updated_at \
             FROM products WHERE code = ",
        )
        .push_bind(code)
        .build_query_as::<ProductRow>()
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .ok_or(DomainError::NotFound)?;
        row.try_into()
    }

    async fn list(&self) -> Result<Vec<Product>, DomainError> {
        let rows: Vec<ProductRow> = sqlx::QueryBuilder::new(
            "SELECT id, code, name, description, active, created_at, updated_at \
             FROM products ORDER BY id",
        )
        .build_query_as::<ProductRow>()
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        rows.into_iter().map(Product::try_from).collect()
    }

    async fn update(&self, input: ProductUpdate) -> Result<Product, DomainError> {
        let mut qb = sqlx::QueryBuilder::new("UPDATE products SET ");
        let mut first = true;
        let mut sep = |qb: &mut sqlx::QueryBuilder<sqlx::Postgres>| {
            if first {
                first = false;
            } else {
                qb.push(", ");
            }
        };
        if let Some(ref c) = input.code {
            sep(&mut qb);
            qb.push("code = ").push_bind(c);
        }
        if let Some(ref n) = input.name {
            sep(&mut qb);
            qb.push("name = ").push_bind(n);
        }
        if let Some(ref d) = input.description {
            sep(&mut qb);
            qb.push("description = ").push_bind(d);
        }
        if let Some(a) = input.active {
            sep(&mut qb);
            qb.push("active = ").push_bind(a);
        }
        if first {
            return self.find_by_id(input.id).await;
        }
        qb.push(" WHERE id = ").push_bind(input.id);
        qb.push(" RETURNING id, code, name, description, active, created_at, updated_at");
        let row: ProductRow = qb
            .build_query_as::<ProductRow>()
            .fetch_optional(&self.pool)
            .await
            .map_err(map_db_error)?
            .ok_or(DomainError::NotFound)?;
        row.try_into()
    }
}

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
