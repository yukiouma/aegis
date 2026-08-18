//! Live-database integration tests for the PostgreSQL adapter.
//!
//! `#[ignore]`-gated; opt in with
//! `AEGIS_TERMINOLOGY_DATABASE_URL` set:
//!
//! ```text
//! cargo test -p terminology -- --ignored --test-threads=1
//! ```
//!
//! Each run drops and re-applies the migrations so the live DB
//! stays in a deterministic state. A failure to connect is
//! reported via a clear panic (never silently skipped).

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use sqlx::PgPool;
use terminology::{
    CodeItem, CodeItemNew, CodeItemRepo, CodeItemRepository, CodeList, CodeListNew, CodeListRepo,
    CodeListRepository, CodeListSearchQuery, CreateTerminologyVersion, DomainError,
    TerminologyKind, TerminologyUsecase, TerminologyUsecaseConfig, TerminologyVersion,
    TerminologyVersionNew, TerminologyVersionRepo, TerminologyVersionRepository,
};

async fn with_pool<F, Fut, T>(f: F) -> T
where
    F: FnOnce(PgPool) -> Fut,
    Fut: std::future::Future<Output = T>,
{
    let _ = dotenvy::dotenv();
    let url = std::env::var("AEGIS_TERMINOLOGY_DATABASE_URL").unwrap_or_else(|_| {
        panic!(
            "AEGIS_TERMINOLOGY_DATABASE_URL must be set (or present in .env \
             at the workspace root) to run --ignored tests"
        )
    });

    let pool = PgPool::connect(&url)
        .await
        .expect("connect to PostgreSQL via AEGIS_TERMINOLOGY_DATABASE_URL");

    sqlx::query("DROP TABLE IF EXISTS code_items CASCADE")
        .execute(&pool)
        .await
        .expect("drop code_items");
    sqlx::query("DROP TABLE IF EXISTS code_lists CASCADE")
        .execute(&pool)
        .await
        .expect("drop code_lists");
    sqlx::query("DROP TABLE IF EXISTS terminology_versions CASCADE")
        .execute(&pool)
        .await
        .expect("drop terminology_versions");
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

fn unique(prefix: &str) -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{prefix}-{nanos:x}-{count}")
}

#[tokio::test]
#[ignore = "requires AEGIS_TERMINOLOGY_DATABASE_URL"]
async fn create_then_find_round_trip_for_all_three_levels() {
    with_pool(|pool| async move {
        let v_repo = TerminologyVersionRepo::new(pool.clone());
        let l_repo = CodeListRepo::new(pool.clone());
        let i_repo = CodeItemRepo::new(pool.clone());

        let v_name = unique("v");
        let v: TerminologyVersion = v_repo
            .create(TerminologyVersionNew {
                kind: TerminologyKind::Sdtm,
                name: v_name.clone(),
            })
            .await
            .expect("version create");

        let cl: CodeList = l_repo
            .create(CodeListNew {
                version_id: v.id,
                code: unique("cl"),
                extensible: true,
                name: "AGE".into(),
                submission_value: "AGE".into(),
                synonym: "".into(),
                definition: "".into(),
                nci_preferred_term: "".into(),
            })
            .await
            .expect("code_list create");

        let item: CodeItem = i_repo
            .create(CodeItemNew {
                codelist_id: cl.id,
                version_id: v.id,
                code: unique("ci"),
                submission_value: ">0".into(),
                synonym: "".into(),
                definition: "".into(),
                nci_preferred_term: "".into(),
            })
            .await
            .expect("code_item create");

        assert_eq!(v.name, v_name);
        assert_eq!(cl.version_id, v.id);
        assert_eq!(item.codelist_id, cl.id);
        assert_eq!(item.version_id, v.id);
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_TERMINOLOGY_DATABASE_URL"]
async fn delete_version_cascades_to_children() {
    with_pool(|pool| async move {
        let v_repo = TerminologyVersionRepo::new(pool.clone());
        let l_repo = CodeListRepo::new(pool.clone());
        let i_repo = CodeItemRepo::new(pool.clone());

        let v = v_repo
            .create(TerminologyVersionNew {
                kind: TerminologyKind::Sdtm,
                name: unique("cascade-v"),
            })
            .await
            .expect("version");

        let cl = l_repo
            .create(CodeListNew {
                version_id: v.id,
                code: unique("cascade-cl"),
                extensible: false,
                name: "AGE".into(),
                submission_value: "AGE".into(),
                synonym: "".into(),
                definition: "".into(),
                nci_preferred_term: "".into(),
            })
            .await
            .expect("code_list");

        let _item = i_repo
            .create(CodeItemNew {
                codelist_id: cl.id,
                version_id: v.id,
                code: unique("cascade-ci"),
                submission_value: "".into(),
                synonym: "".into(),
                definition: "".into(),
                nci_preferred_term: "".into(),
            })
            .await
            .expect("code_item");

        v_repo.delete(v.id).await.expect("delete version");

        let err = l_repo.find_by_id(cl.id).await.expect_err("cl gone");
        assert!(
            matches!(err, DomainError::CodeListNotFound(_)),
            "expected CodeListNotFound, got {err:?}"
        );
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_TERMINOLOGY_DATABASE_URL"]
async fn search_code_lists_ranks_hits() {
    with_pool(|pool| async move {
        let v_repo = TerminologyVersionRepo::new(pool.clone());
        let l_repo = CodeListRepo::new(pool.clone());

        let v = v_repo
            .create(TerminologyVersionNew {
                kind: TerminologyKind::Sdtm,
                name: unique("search-v"),
            })
            .await
            .expect("version");

        l_repo
            .create(CodeListNew {
                version_id: v.id,
                code: unique("age-cl"),
                extensible: true,
                name: "AGE".into(),
                submission_value: "AGE".into(),
                synonym: "Age group".into(),
                definition: "Subject age".into(),
                nci_preferred_term: "Age".into(),
            })
            .await
            .expect("age cl");

        l_repo
            .create(CodeListNew {
                version_id: v.id,
                code: unique("sex-cl"),
                extensible: true,
                name: "SEX".into(),
                submission_value: "SEX".into(),
                synonym: "".into(),
                definition: "Sex".into(),
                nci_preferred_term: "Sex".into(),
            })
            .await
            .expect("sex cl");

        let hits = l_repo
            .search(CodeListSearchQuery {
                version_id: v.id,
                fragment: "age".into(),
                limit: 10,
            })
            .await
            .expect("search");

        assert!(
            !hits.is_empty(),
            "expected at least one hit for `age` in version {}",
            v.name
        );
        assert!(
            hits.iter().any(|h| h.codelist.name == "AGE"),
            "AGE row should be in the hits"
        );
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_TERMINOLOGY_DATABASE_URL"]
async fn usecase_wires_through_all_three_repos() {
    with_pool(|pool| async move {
        let v = TerminologyVersionRepo::new(pool.clone());
        let l = CodeListRepo::new(pool.clone());
        let i = CodeItemRepo::new(pool.clone());
        let usecase = TerminologyUsecase::new(TerminologyUsecaseConfig {
            version_repo: v,
            code_list_repo: l,
            code_item_repo: i,
        });

        let v_name = unique("usecase-v");
        let created_v = usecase
            .create_version(CreateTerminologyVersion {
                kind: TerminologyKind::Sdtm,
                name: v_name.clone(),
            })
            .await
            .expect("usecase create version");
        let _ = usecase
            .get_version_by_id(created_v.id)
            .await
            .expect("usecase get");
        assert_eq!(created_v.name, v_name);
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_TERMINOLOGY_DATABASE_URL"]
async fn list_code_items_by_version_and_code_end_to_end() {
    with_pool(|pool| async move {
        let v_repo = TerminologyVersionRepo::new(pool.clone());
        let l_repo = CodeListRepo::new(pool.clone());
        let i_repo = CodeItemRepo::new(pool.clone());

        let v = v_repo
            .create(TerminologyVersionNew {
                kind: TerminologyKind::Sdtm,
                name: unique("natural-v"),
            })
            .await
            .expect("version");

        let age_code = unique("natural-age");
        let sex_code = unique("natural-sex");
        let age = l_repo
            .create(CodeListNew {
                version_id: v.id,
                code: age_code.clone(),
                extensible: true,
                name: "AGE".into(),
                submission_value: "AGE".into(),
                synonym: "".into(),
                definition: "".into(),
                nci_preferred_term: "".into(),
            })
            .await
            .expect("age cl");
        let sex = l_repo
            .create(CodeListNew {
                version_id: v.id,
                code: sex_code.clone(),
                extensible: true,
                name: "SEX".into(),
                submission_value: "SEX".into(),
                synonym: "".into(),
                definition: "".into(),
                nci_preferred_term: "".into(),
            })
            .await
            .expect("sex cl");

        // Item code "C1" appears in both codelists of v.
        i_repo
            .create(CodeItemNew {
                codelist_id: age.id,
                version_id: v.id,
                code: "C1".into(),
                submission_value: "".into(),
                synonym: "".into(),
                definition: "".into(),
                nci_preferred_term: "".into(),
            })
            .await
            .expect("age C1");
        let sex_c1 = i_repo
            .create(CodeItemNew {
                codelist_id: sex.id,
                version_id: v.id,
                code: "C1".into(),
                submission_value: "".into(),
                synonym: "".into(),
                definition: "".into(),
                nci_preferred_term: "".into(),
            })
            .await
            .expect("sex C1");
        i_repo
            .create(CodeItemNew {
                codelist_id: age.id,
                version_id: v.id,
                code: "C2".into(),
                submission_value: "".into(),
                synonym: "".into(),
                definition: "".into(),
                nci_preferred_term: "".into(),
            })
            .await
            .expect("age C2");

        let usecase = TerminologyUsecase::new(TerminologyUsecaseConfig {
            version_repo: v_repo,
            code_list_repo: l_repo,
            code_item_repo: i_repo,
        });

        // (v, "C1") hits one row in each codelist.
        let c1_items = usecase
            .list_code_items_by_version_and_code(v.id, "C1")
            .await
            .expect("C1 lookup");
        assert_eq!(c1_items.len(), 2);
        assert!(
            c1_items.iter().all(|i| i.version_id == v.id),
            "all returned items carry the version_id"
        );
        assert!(
            c1_items.iter().any(|i| i.codelist_id == age.id),
            "AGE row included"
        );
        assert!(
            c1_items
                .iter()
                .any(|i| i.codelist_id == sex.id && i.id == sex_c1.id),
            "SEX row included"
        );
        assert!(
            c1_items.iter().all(|i| i.code == "C1"),
            "all returned items use the requested code"
        );

        // (v, "C2") matches only the AGE entry.
        let c2_items = usecase
            .list_code_items_by_version_and_code(v.id, "C2")
            .await
            .expect("C2 lookup");
        assert_eq!(c2_items.len(), 1);
        assert_eq!(c2_items[0].codelist_id, age.id);

        // Unknown code returns empty.
        let empty = usecase
            .list_code_items_by_version_and_code(v.id, "C99999")
            .await
            .expect("missing lookup");
        assert!(empty.is_empty());

        // The codelist NCI code is irrelevant to the lookup; we
        // only filter by `(version_id, code_items.code)`.
        let _ = &age_code;
        let _ = &sex_code;
    })
    .await;
}
