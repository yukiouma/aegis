//! Live-database integration tests for the PostgreSQL adapter.
//!
//! `#[ignore]`-gated; opt in with `cargo test -p project -- --ignored`.
//! Reads `AEGIS_PROJECT_DATABASE_URL`; loads `.env` via dotenvy. Drops
//! the live tables + `_sqlx_migrations` before each run so the
//! migration starts fresh.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use sqlx::PgPool;

use project::domain::{ProductNew, ProductUpdate, ProjectMember, ProjectNew, ProjectUpdate};
use project::{ProductRepo, ProductRepository, ProjectRepo, ProjectRepository};

async fn with_pool<F, Fut, T>(f: F) -> T
where
    F: FnOnce(PgPool) -> Fut,
    Fut: std::future::Future<Output = T>,
{
    let _ = dotenvy::dotenv();
    let url = std::env::var("AEGIS_PROJECT_DATABASE_URL").unwrap_or_else(|_| {
        panic!(
            "AEGIS_PROJECT_DATABASE_URL must be set (or present in .env at the workspace root) \
             to run --ignored tests"
        )
    });
    let pool = PgPool::connect(&url)
        .await
        .expect("connect to PostgreSQL via AEGIS_PROJECT_DATABASE_URL");

    // Destructive cleanup. The integration tests own the schema; if
    // you point them at production by mistake you will lose data.
    sqlx::query("DROP TABLE IF EXISTS project_members CASCADE")
        .execute(&pool)
        .await
        .expect("drop project_members");
    sqlx::query("DROP TABLE IF EXISTS projects CASCADE")
        .execute(&pool)
        .await
        .expect("drop projects");
    sqlx::query("DROP TABLE IF EXISTS products CASCADE")
        .execute(&pool)
        .await
        .expect("drop products");
    sqlx::query("DROP TABLE IF EXISTS _sqlx_migrations CASCADE")
        .execute(&pool)
        .await
        .expect("drop sqlx_migrations bookkeeping");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("apply migrations");

    f(pool).await
}

fn unique_code(prefix: &str) -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{prefix}-{nanos:x}-{count}")
}

#[tokio::test]
#[ignore = "requires AEGIS_PROJECT_DATABASE_URL pointing at a live PostgreSQL"]
async fn product_create_find_list_round_trip() {
    with_pool(|pool| async move {
        let repo = ProductRepo::new(pool);
        let code = unique_code("prod");
        let created = repo
            .create(ProductNew {
                code: code.clone(),
                name: "Widget".into(),
                description: "".into(),
            })
            .await
            .expect("create");
        assert_eq!(created.code, code);

        let by_id = repo.find_by_id(created.id).await.expect("find_by_id");
        assert_eq!(by_id.code, code);
        let by_code = repo.find_by_code(&code).await.expect("find_by_code");
        assert_eq!(by_code.id, created.id);
        let list = repo.list().await.expect("list");
        assert!(list.iter().any(|p| p.id == created.id));
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_PROJECT_DATABASE_URL pointing at a live PostgreSQL"]
async fn product_update_flips_active_and_keeps_created_at() {
    with_pool(|pool| async move {
        let repo = ProductRepo::new(pool);
        let created = repo
            .create(ProductNew {
                code: unique_code("prod-active"),
                name: "Widget".into(),
                description: "".into(),
            })
            .await
            .expect("create");
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        let updated = repo
            .update(ProductUpdate {
                id: created.id,
                active: Some(false),
                ..Default::default()
            })
            .await
            .expect("update");
        assert!(!updated.active);
        assert!(
            updated.updated_at > created.updated_at,
            "products_set_updated_at trigger must bump updated_at"
        );
        assert_eq!(updated.created_at, created.created_at);
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_PROJECT_DATABASE_URL pointing at a live PostgreSQL"]
async fn project_create_with_no_membership_round_trip() {
    with_pool(|pool| async move {
        let products = ProductRepo::new(pool.clone());
        let projects = ProjectRepo::new(pool.clone());
        let product = products
            .create(ProductNew {
                code: unique_code("prod-shell"),
                name: "Shell".into(),
                description: "".into(),
            })
            .await
            .expect("create product");
        let created = projects
            .create(ProjectNew {
                code: unique_code("proj-shell"),
                description: "".into(),
                product_id: product.id,
                members: None,
                unblind_members: None,
            })
            .await
            .expect("create project");
        assert_eq!(created.product_id, product.id);
        assert!(created.members.leaders.is_empty());
        assert!(created.members.workers.is_empty());
        assert!(created.unblind_members.leaders.is_empty());
        assert!(created.unblind_members.workers.is_empty());
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_PROJECT_DATABASE_URL pointing at a live PostgreSQL"]
async fn project_create_with_membership_then_update_replaces_it() {
    with_pool(|pool| async move {
        let products = ProductRepo::new(pool.clone());
        let projects = ProjectRepo::new(pool.clone());
        let product = products
            .create(ProductNew {
                code: unique_code("prod-mem"),
                name: "Mem".into(),
                description: "".into(),
            })
            .await
            .expect("create product");
        let created = projects
            .create(ProjectNew {
                code: unique_code("proj-mem"),
                description: "".into(),
                product_id: product.id,
                members: Some(ProjectMember {
                    leaders: vec!["u1".into()],
                    workers: vec!["u2".into()],
                }),
                unblind_members: Some(ProjectMember::default()),
            })
            .await
            .expect("create project");
        assert_eq!(created.members.leaders, vec!["u1".to_string()]);
        assert_eq!(created.members.workers, vec!["u2".to_string()]);
        assert!(created.unblind_members.leaders.is_empty());

        let updated = projects
            .update(ProjectUpdate {
                id: created.id,
                members: Some(ProjectMember {
                    leaders: vec![],
                    workers: vec!["u3".into()],
                }),
                unblind_members: Some(ProjectMember::default()),
                ..Default::default()
            })
            .await
            .expect("update");
        assert!(updated.members.leaders.is_empty());
        assert_eq!(updated.members.workers, vec!["u3".to_string()]);
        assert!(updated.unblind_members.leaders.is_empty());
        assert!(updated.unblind_members.workers.is_empty());

        // Spot-check via direct query that no `unblind_members` rows
        // remain after the wipe.
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT team_type FROM project_members WHERE project_id = $1",
        )
        .bind(created.id)
        .fetch_all(&pool)
        .await
        .expect("query members");
        assert!(rows.iter().all(|(t,)| t != "unblind_members"));
    })
    .await;
}
