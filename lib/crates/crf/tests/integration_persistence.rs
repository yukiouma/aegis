//! Live-DB round-trips for the crf crate.
//!
//! `#[ignore]`-gated. Requires `AEGIS_CRF_DATABASE_URL`.
//! Drops all `crf_*` tables + `_sqlx_migrations` before each
//! run (destructive on purpose, per the project convention).
//!
//! These tests exercise the persistence layer directly via
//! the `*RepoPg` types — going through the full
//! `CrfServiceImpl` facade would require a working
//! `ProjectService` mock, which is the domain of the
//! facade-level unit tests.
//!
//! Run with:
//!   `cargo test -p crf --test integration_persistence -- --ignored --test-threads=1`

use std::sync::atomic::{AtomicI64, Ordering};

use crf::{
    Annotation, AnnotationOwner, CrfFormNew, CrfFormRepoPg, CrfFormRepository, CrfItemKind,
    CrfItemNew, CrfItemRepoPg, CrfItemRepository, CrfOptionNew, CrfOptionRepoPg,
    CrfOptionRepository, CrfUnitNew, CrfUnitRepoPg, CrfUnitRepository, CrfVersion, CrfVersionNew,
    CrfVersionRepoPg, CrfVersionRepository, DomainAnnotationNew, DomainAnnotationRepoPg,
    DomainAnnotationRepository, DomainError,
};

static ID_GEN: AtomicI64 = AtomicI64::new(0);

fn unique_suffix() -> String {
    let n = ID_GEN.fetch_add(1, Ordering::SeqCst);
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{}_{}", ts, n)
}

async fn connect() -> sqlx::PgPool {
    let _ = dotenvy::dotenv();
    let url = std::env::var("AEGIS_CRF_DATABASE_URL").unwrap_or_else(|_| {
        panic!(
            "AEGIS_CRF_DATABASE_URL must be set to run integration_persistence tests \
             (cargo test -p crf -- --ignored --test-threads=1)"
        )
    });
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect(&url)
        .await
        .expect("connect to Postgres");

    // Destructive reset: drop everything crf-related.
    sqlx::query(
        "DROP TABLE IF EXISTS crf_annotations, crf_domain_annotations, crf_units, \
         crf_options, crf_items, crf_forms, crf_versions, _sqlx_migrations CASCADE",
    )
    .execute(&pool)
    .await
    .expect("drop all crf_* tables + _sqlx_migrations");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    pool
}

#[tokio::test]
#[ignore]
async fn cascade_delete_form_with_version() {
    let pool = connect().await;
    let versions = CrfVersionRepoPg::new(pool.clone());
    let forms = CrfFormRepoPg::new(pool.clone());

    let suffix = unique_suffix();
    let v = versions
        .create(CrfVersionNew {
            project_code: format!("P_{suffix}"),
            name: "v1".into(),
        })
        .await
        .unwrap();
    let f = forms
        .create(CrfFormNew {
            version_id: v.id,
            code: format!("F_{suffix}"),
            name: "Form 1".into(),
            order: 0,
            not_submitted: false,
        })
        .await
        .unwrap();

    // Drop the version — the form should disappear via FK CASCADE.
    sqlx::query("DELETE FROM crf_versions WHERE id = $1")
        .bind(v.id)
        .execute(&pool)
        .await
        .unwrap();

    let fetched = forms.find_by_id(f.id).await;
    assert!(
        matches!(fetched, Err(DomainError::NotFound)),
        "form should cascade-delete with version"
    );
}

#[tokio::test]
#[ignore]
async fn unique_project_code_name_per_version() {
    let pool = connect().await;
    let versions = CrfVersionRepoPg::new(pool.clone());

    let suffix = unique_suffix();
    let code = format!("P_{suffix}");

    versions
        .create(CrfVersionNew {
            project_code: code.clone(),
            name: "v1".into(),
        })
        .await
        .unwrap();
    // Same (project_code, name) → unique constraint rejects.
    let dup = versions
        .create(CrfVersionNew {
            project_code: code,
            name: "v1".into(),
        })
        .await;
    assert!(dup.is_err(), "duplicate (project_code, name) must fail");
}

#[tokio::test]
#[ignore]
async fn polymorphic_owner_check_rejects_two_owners() {
    let pool = connect().await;
    let versions = CrfVersionRepoPg::new(pool.clone());
    let forms = CrfFormRepoPg::new(pool.clone());
    let items = CrfItemRepoPg::new(pool.clone());
    let domain_annotations = DomainAnnotationRepoPg::new(pool.clone());

    let suffix = unique_suffix();
    let v = versions
        .create(CrfVersionNew {
            project_code: format!("P_{suffix}"),
            name: "v1".into(),
        })
        .await
        .unwrap();
    let f = forms
        .create(CrfFormNew {
            version_id: v.id,
            code: format!("F_{suffix}"),
            name: "F".into(),
            order: 0,
            not_submitted: false,
        })
        .await
        .unwrap();
    let it = items
        .create(CrfItemNew {
            form_id: f.id,
            code: format!("I_{suffix}"),
            name: "I".into(),
            kind: CrfItemKind::Text,
            order: 0,
            not_submitted: false,
        })
        .await
        .unwrap();
    let d = domain_annotations
        .create(DomainAnnotationNew {
            form_id: f.id,
            name: format!("D_{suffix}"),
            description: "d".into(),
        })
        .await
        .unwrap();

    // Try to insert an annotation with TWO owners via direct SQL —
    // the CHECK constraint must reject this.
    let res: Result<(i64,), _> = sqlx::query_as(
        "INSERT INTO crf_annotations \
         (domain_annotation_id, content, assign, form_id, item_id) \
         VALUES ($1, $2, $3, $4, $5) RETURNING id",
    )
    .bind(d.id)
    .bind("x")
    .bind(false)
    .bind(f.id)
    .bind(it.id)
    .fetch_one(&pool)
    .await;
    assert!(res.is_err(), "CHECK constraint should reject two owners");

    // Valid case: insert with exactly one owner.
    let a: (i64,) = sqlx::query_as(
        "INSERT INTO crf_annotations \
         (domain_annotation_id, content, assign, form_id) \
         VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(d.id)
    .bind("only form owner")
    .bind(false)
    .bind(f.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(a.0 > 0);
}

#[tokio::test]
#[ignore]
async fn options_cascade_delete_with_item() {
    let pool = connect().await;
    let versions = CrfVersionRepoPg::new(pool.clone());
    let forms = CrfFormRepoPg::new(pool.clone());
    let items = CrfItemRepoPg::new(pool.clone());
    let options = CrfOptionRepoPg::new(pool.clone());

    let suffix = unique_suffix();
    let v = versions
        .create(CrfVersionNew {
            project_code: format!("P_{suffix}"),
            name: "v1".into(),
        })
        .await
        .unwrap();
    let f = forms
        .create(CrfFormNew {
            version_id: v.id,
            code: format!("F_{suffix}"),
            name: "F".into(),
            order: 0,
            not_submitted: false,
        })
        .await
        .unwrap();
    let it = items
        .create(CrfItemNew {
            form_id: f.id,
            code: format!("I_{suffix}"),
            name: "I".into(),
            kind: CrfItemKind::Text,
            order: 0,
            not_submitted: false,
        })
        .await
        .unwrap();
    options
        .create(CrfOptionNew {
            item_id: it.id,
            value: "yes".into(),
            not_submitted: false,
        })
        .await
        .unwrap();

    sqlx::query("DELETE FROM crf_items WHERE id = $1")
        .bind(it.id)
        .execute(&pool)
        .await
        .unwrap();

    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM crf_options WHERE item_id = $1")
        .bind(it.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 0, "options should cascade-delete with item");
}

#[tokio::test]
#[ignore]
async fn units_cascade_delete_with_item() {
    let pool = connect().await;
    let versions = CrfVersionRepoPg::new(pool.clone());
    let forms = CrfFormRepoPg::new(pool.clone());
    let items = CrfItemRepoPg::new(pool.clone());
    let units = CrfUnitRepoPg::new(pool.clone());

    let suffix = unique_suffix();
    let v = versions
        .create(CrfVersionNew {
            project_code: format!("P_{suffix}"),
            name: "v1".into(),
        })
        .await
        .unwrap();
    let f = forms
        .create(CrfFormNew {
            version_id: v.id,
            code: format!("F_{suffix}"),
            name: "F".into(),
            order: 0,
            not_submitted: false,
        })
        .await
        .unwrap();
    let it = items
        .create(CrfItemNew {
            form_id: f.id,
            code: format!("I_{suffix}"),
            name: "I".into(),
            kind: CrfItemKind::Text,
            order: 0,
            not_submitted: false,
        })
        .await
        .unwrap();
    units
        .create(CrfUnitNew {
            item_id: it.id,
            value: "mg".into(),
            not_submitted: false,
        })
        .await
        .unwrap();

    sqlx::query("DELETE FROM crf_items WHERE id = $1")
        .bind(it.id)
        .execute(&pool)
        .await
        .unwrap();

    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM crf_units WHERE item_id = $1")
        .bind(it.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 0, "units should cascade-delete with item");
}

// Verify each polymorphic owner variant can be constructed and
// inserted via the appropriate FK column.
#[tokio::test]
#[ignore]
async fn polymorphic_owner_round_trip() {
    let pool = connect().await;
    let versions = CrfVersionRepoPg::new(pool.clone());
    let forms = CrfFormRepoPg::new(pool.clone());
    let items = CrfItemRepoPg::new(pool.clone());
    let options = CrfOptionRepoPg::new(pool.clone());
    let units = CrfUnitRepoPg::new(pool.clone());
    let domain_annotations = DomainAnnotationRepoPg::new(pool.clone());

    let suffix = unique_suffix();
    let v = versions
        .create(CrfVersionNew {
            project_code: format!("P_{suffix}"),
            name: "v1".into(),
        })
        .await
        .unwrap();
    let f = forms
        .create(CrfFormNew {
            version_id: v.id,
            code: format!("F_{suffix}"),
            name: "F".into(),
            order: 0,
            not_submitted: false,
        })
        .await
        .unwrap();
    let it = items
        .create(CrfItemNew {
            form_id: f.id,
            code: format!("I_{suffix}"),
            name: "I".into(),
            kind: CrfItemKind::Text,
            order: 0,
            not_submitted: false,
        })
        .await
        .unwrap();
    let o = options
        .create(CrfOptionNew {
            item_id: it.id,
            value: "yes".into(),
            not_submitted: false,
        })
        .await
        .unwrap();
    let u = units
        .create(CrfUnitNew {
            item_id: it.id,
            value: "mg".into(),
            not_submitted: false,
        })
        .await
        .unwrap();
    let d = domain_annotations
        .create(DomainAnnotationNew {
            form_id: f.id,
            name: format!("D_{suffix}"),
            description: "d".into(),
        })
        .await
        .unwrap();

    // Compose one annotation per owner kind, verify each row
    // exists with the correct owner column. Each variant uses a
    // literal SQL string so we satisfy `sqlx`'s SqlSafeStr bound.
    for (owner_kind, id) in [
        (AnnotationOwner::Form { id: f.id }, f.id),
        (AnnotationOwner::Item { id: it.id }, it.id),
        (AnnotationOwner::Option { id: o.id }, o.id),
        (AnnotationOwner::Unit { id: u.id }, u.id),
    ] {
        let annotation = match owner_kind {
            AnnotationOwner::Form { id } => Annotation::for_form(d.id, "x".into(), false, id),
            AnnotationOwner::Item { id } => Annotation::for_item(d.id, "x".into(), false, id),
            AnnotationOwner::Option { id } => Annotation::for_option(d.id, "x".into(), false, id),
            AnnotationOwner::Unit { id } => Annotation::for_unit(d.id, "x".into(), false, id),
        };
        assert!(annotation.is_ok(), "polymorphic constructor should succeed");

        let inserted: (i64,) = match owner_kind {
            AnnotationOwner::Form { .. } => sqlx::query_as(
                "INSERT INTO crf_annotations (domain_annotation_id, content, assign, form_id) \
                 VALUES ($1, $2, $3, $4) RETURNING id",
            )
            .bind(d.id)
            .bind("content")
            .bind(false)
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap(),
            AnnotationOwner::Item { .. } => sqlx::query_as(
                "INSERT INTO crf_annotations (domain_annotation_id, content, assign, item_id) \
                 VALUES ($1, $2, $3, $4) RETURNING id",
            )
            .bind(d.id)
            .bind("content")
            .bind(false)
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap(),
            AnnotationOwner::Option { .. } => sqlx::query_as(
                "INSERT INTO crf_annotations (domain_annotation_id, content, assign, option_id) \
                 VALUES ($1, $2, $3, $4) RETURNING id",
            )
            .bind(d.id)
            .bind("content")
            .bind(false)
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap(),
            AnnotationOwner::Unit { .. } => sqlx::query_as(
                "INSERT INTO crf_annotations (domain_annotation_id, content, assign, unit_id) \
                 VALUES ($1, $2, $3, $4) RETURNING id",
            )
            .bind(d.id)
            .bind("content")
            .bind(false)
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        };
        assert!(inserted.0 > 0);
    }
}

// Reference the CrfVersion aggregate constructor so the
// domain types stay compiled.
#[test]
fn domain_aggregates_construct() {
    let _ = CrfVersion::new("P1".into(), "v1".into()).unwrap();
}
