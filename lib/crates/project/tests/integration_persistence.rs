//! Live-database integration tests for the PostgreSQL adapter.
//!
//! `#[ignore]`-gated; opt in with `cargo test -p project -- --ignored`.
//! Reads the workspace-shared `AEGIS_DATABASE_URL` (same convention
//! as the `auth` and `user` crates); loads `.env` at the workspace
//! root via `dotenvy` so the variable only needs to live in `.env`.
//! Drops the live tables + `_sqlx_migrations` before each run so
//! the migration starts fresh.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use sqlx::PgPool;

use project::domain::{ProjectMember, ProjectNew, ProjectTag, ProjectUpdate};
use project::{ProjectRepo, ProjectRepository};

async fn with_pool<F, Fut, T>(f: F) -> T
where
    F: FnOnce(PgPool) -> Fut,
    Fut: std::future::Future<Output = T>,
{
    let _ = dotenvy::dotenv();
    let url = std::env::var("AEGIS_DATABASE_URL").unwrap_or_else(|_| {
        panic!(
            "AEGIS_DATABASE_URL must be set (or present in .env at the workspace root) \
             to run --ignored tests"
        )
    });
    let pool = PgPool::connect(&url)
        .await
        .expect("connect to PostgreSQL via AEGIS_DATABASE_URL");

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
#[ignore = "requires AEGIS_DATABASE_URL pointing at a live PostgreSQL"]
async fn project_create_with_no_membership_or_tags_round_trip() {
    with_pool(|pool| async move {
        let projects = ProjectRepo::new(pool.clone());
        let created = projects
            .create(ProjectNew {
                code: unique_code("proj-shell"),
                description: "".into(),
                members: None,
                unblind_members: None,
                tags: None,
            })
            .await
            .expect("create project");
        assert!(created.members.leaders.is_empty());
        assert!(created.members.workers.is_empty());
        assert!(created.unblind_members.leaders.is_empty());
        assert!(created.unblind_members.workers.is_empty());
        assert!(created.tags.is_empty());
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_DATABASE_URL pointing at a live PostgreSQL"]
async fn project_create_with_membership_then_update_replaces_it() {
    with_pool(|pool| async move {
        let projects = ProjectRepo::new(pool.clone());
        let created = projects
            .create(ProjectNew {
                code: unique_code("proj-mem"),
                description: "".into(),
                members: Some(ProjectMember {
                    leaders: vec!["u1".into()],
                    workers: vec!["u2".into()],
                }),
                unblind_members: Some(ProjectMember::default()),
                tags: None,
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
        let rows: Vec<(String,)> =
            sqlx::query_as("SELECT team_type FROM project_members WHERE project_id = $1")
                .bind(created.id)
                .fetch_all(&pool)
                .await
                .expect("query members");
        assert!(rows.iter().all(|(t,)| t != "unblind_members"));
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_DATABASE_URL pointing at a live PostgreSQL"]
async fn project_create_with_tags_round_trip() {
    with_pool(|pool| async move {
        let projects = ProjectRepo::new(pool.clone());
        let created = projects
            .create(ProjectNew {
                code: unique_code("proj-tags"),
                description: "".into(),
                members: None,
                unblind_members: None,
                tags: Some(vec![
                    ProjectTag::for_repository("Product".into(), "DEMO-001".into()),
                    ProjectTag::for_repository("Region".into(), "EU".into()),
                ]),
            })
            .await
            .expect("create project");
        assert_eq!(created.tags.len(), 2);
        assert_eq!(created.tags[0].key, "Product");
        assert_eq!(created.tags[1].value, "EU");
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_DATABASE_URL pointing at a live PostgreSQL"]
async fn project_update_replaces_tags_whole_list() {
    with_pool(|pool| async move {
        let projects = ProjectRepo::new(pool.clone());
        let created = projects
            .create(ProjectNew {
                code: unique_code("proj-tags-replace"),
                description: "".into(),
                members: None,
                unblind_members: None,
                tags: Some(vec![
                    ProjectTag::for_repository("k1".into(), "v1".into()),
                ]),
            })
            .await
            .expect("create project");
        let updated = projects
            .update(ProjectUpdate {
                id: created.id,
                tags: Some(vec![
                    ProjectTag::for_repository("k2".into(), "v2".into()),
                    ProjectTag::for_repository("k3".into(), "v3".into()),
                ]),
                ..Default::default()
            })
            .await
            .expect("update");
        assert_eq!(updated.tags.len(), 2);
        assert_eq!(updated.tags[0].key, "k2");
        assert_eq!(updated.tags[1].key, "k3");
        // Spot-check via direct JSONB query. We deserialize the JSONB
        // payload as `Vec<serde_json::Value>` rather than `Json<Vec<ProjectTag>>`
        // because the runtime `query_as` API doesn't pick up the
        // `sqlx::FromRow` impl for the latter without an explicit
        // derive on a wrapper struct.
        let raw: Vec<serde_json::Value> =
            sqlx::query_scalar("SELECT tags FROM projects WHERE id = $1")
                .bind(created.id)
                .fetch_one(&pool)
                .await
                .expect("query tags");
        assert_eq!(raw.len(), 2);
        assert_eq!(raw[0]["key"], "k2");
        assert_eq!(raw[1]["value"], "v3");
    })
    .await;
}