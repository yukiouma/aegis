//! Live-database integration tests for the PostgreSQL adapter.
//!
//! `#[ignore]`-gated; opt in with `AEGIS_TEST_DATABASE_URL` set:
//!
//! ```text
//! cargo test -p sdtm-project -- --ignored --test-threads=1
//! ```
//!
//! Each run drops and re-applies the migrations so the live DB stays
//! in a deterministic state. A failure to connect is reported via a
//! clear panic (never silently skipped).
//!
//! ## The URL must point at a dedicated database
//!
//! SQLx keeps a single `_sqlx_migrations` table *per database*, shared
//! by every crate that migrates it. This test drops that table, so
//! pointing it at the server's database would erase the
//! applied-migration record for auth, user, project, terminology,
//! mission and crf alike. Use a database of its own.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use sdtm_project::{
    ConversionRule, DomainCategory, DomainError, FromCrf, SdtmDomainDescription,
    SdtmDomainDescriptionDetail, SdtmProjectDomainNew, SdtmProjectDomainRepoPg,
    SdtmProjectDomainRepository, SdtmProjectDomainUpdate, SdtmProjectVariableNew,
    SdtmProjectVariableRepoPg, SdtmProjectVariableRepository, SdtmProjectVariableUpdate,
    SdtmProjectVersionNew, SdtmProjectVersionRepoPg, SdtmProjectVersionRepository,
    SdtmProjectVersionUpdate, SdtmRole, SdtmVariableControlled, SdtmVariableCore, SdtmVariableType,
    VariableSource,
};
use sqlx::PgPool;

async fn with_pool<F, Fut, T>(f: F) -> T
where
    F: FnOnce(PgPool) -> Fut,
    Fut: std::future::Future<Output = T>,
{
    let _ = dotenvy::dotenv();
    let url = std::env::var("AEGIS_TEST_DATABASE_URL").unwrap_or_else(|_| {
        panic!(
            "AEGIS_TEST_DATABASE_URL must be set (or present in .env \
             at the workspace root) to run --ignored tests. It must point at a \
             DEDICATED database: the test drops the _sqlx_migrations table, \
             which sqlx shares across every crate that migrates that database."
        )
    });

    let pool = PgPool::connect(&url)
        .await
        .expect("connect to PostgreSQL via AEGIS_TEST_DATABASE_URL");

    // Drop in reverse FK order so cascading rules never matter. The
    // statements are literals rather than a formatted list: sqlx
    // requires `&'static str`, and there is nothing to interpolate.
    for stmt in [
        "DROP TABLE IF EXISTS sdtm_project_variables CASCADE",
        "DROP TABLE IF EXISTS sdtm_project_domains CASCADE",
        "DROP TABLE IF EXISTS sdtm_project_versions CASCADE",
        "DROP TABLE IF EXISTS _sqlx_migrations CASCADE",
    ] {
        sqlx::query(stmt)
            .execute(&pool)
            .await
            .unwrap_or_else(|e| panic!("drop {stmt}: {e}"));
    }

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("apply migrations");

    f(pool).await
}

/// A per-run unique value for every column with a `UNIQUE` constraint,
/// so concurrent runs cannot collide.
fn unique(prefix: &str) -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{prefix}-{nanos:x}-{count}")
}

// ---- SdtmProjectVersion ---------------------------------------------------

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL"]
async fn version_create_then_list_round_trips() {
    with_pool(|pool| async move {
        let repo = SdtmProjectVersionRepoPg::new(pool);
        let project_code = unique("p");
        let version_name = unique("v");
        let v = repo
            .create(SdtmProjectVersionNew {
                project_code: project_code.clone(),
                version_name: version_name.clone(),
            })
            .await
            .expect("version create");
        assert!(v.id > 0);
        assert!(v.created_at <= v.updated_at);
        assert_eq!(v.project_code, project_code);
        assert_eq!(v.version_name, version_name);

        let listed = repo.list_by_project(&project_code).await.expect("list");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, v.id);

        let by_id = repo.find_by_id(v.id).await.expect("find by id");
        assert_eq!(by_id.version_name, version_name);
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL"]
async fn version_update_changes_name() {
    with_pool(|pool| async move {
        let repo = SdtmProjectVersionRepoPg::new(pool);
        let v = repo
            .create(SdtmProjectVersionNew {
                project_code: unique("p"),
                version_name: unique("v"),
            })
            .await
            .unwrap();
        let new_name = unique("v");
        let updated = repo
            .update(SdtmProjectVersionUpdate {
                id: v.id,
                version_name: Some(new_name.clone()),
            })
            .await
            .expect("version update");
        assert_eq!(updated.version_name, new_name);
        assert_eq!(updated.project_code, v.project_code);
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL"]
async fn version_update_of_missing_row_is_not_found() {
    with_pool(|pool| async move {
        let repo = SdtmProjectVersionRepoPg::new(pool);
        let err = repo
            .update(SdtmProjectVersionUpdate {
                id: 999_999,
                version_name: Some("x".into()),
            })
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            DomainError::SdtmProjectVersionNotFound(999_999)
        ));
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL"]
async fn version_delete_of_missing_row_is_not_found() {
    with_pool(|pool| async move {
        let repo = SdtmProjectVersionRepoPg::new(pool);
        let err = repo.delete(999_999).await.unwrap_err();
        assert!(matches!(
            err,
            DomainError::SdtmProjectVersionNotFound(999_999)
        ));
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL"]
async fn version_unique_is_enforced_per_project() {
    with_pool(|pool| async move {
        let repo = SdtmProjectVersionRepoPg::new(pool);
        let version_name = unique("v");
        // The same version name under two different projects is fine.
        repo.create(SdtmProjectVersionNew {
            project_code: "P-a".into(),
            version_name: version_name.clone(),
        })
        .await
        .expect("first project");
        repo.create(SdtmProjectVersionNew {
            project_code: "P-b".into(),
            version_name: version_name.clone(),
        })
        .await
        .expect("second project");

        // The same pair twice is a duplicate.
        let err = repo
            .create(SdtmProjectVersionNew {
                project_code: "P-a".into(),
                version_name: version_name.clone(),
            })
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            DomainError::DuplicateSdtmProjectVersion { .. }
        ));
    })
    .await
}

// ---- SdtmProjectDomain ----------------------------------------------------

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL"]
async fn domain_create_then_list_round_trips() {
    with_pool(|pool| async move {
        let versions = SdtmProjectVersionRepoPg::new(pool.clone());
        let domains = SdtmProjectDomainRepoPg::new(pool);
        let v = versions
            .create(SdtmProjectVersionNew {
                project_code: unique("p"),
                version_name: unique("v"),
            })
            .await
            .unwrap();

        let d = domains
            .create(SdtmProjectDomainNew {
                sdtm_project_version_id: v.id,
                domain_code: "AE".into(),
                category: DomainCategory::Events,
                descriptions: Vec::new(),
                key_variables: Vec::new(),
            })
            .await
            .expect("domain create");
        assert!(d.id > 0);
        assert_eq!(d.sdtm_project_version_id, v.id);
        assert_eq!(d.domain_code, "AE");
        assert_eq!(d.category, DomainCategory::Events);
        assert!(d.descriptions.is_empty());

        let listed = domains.list_by_version(v.id).await.expect("list");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, d.id);
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL"]
async fn domain_key_variables_preserve_caller_order() {
    with_pool(|pool| async move {
        let versions = SdtmProjectVersionRepoPg::new(pool.clone());
        let domains = SdtmProjectDomainRepoPg::new(pool);
        let v = versions
            .create(SdtmProjectVersionNew {
                project_code: unique("p"),
                version_name: unique("v"),
            })
            .await
            .unwrap();

        // Deliberately not alphabetical: the caller's order is the
        // order that must survive the JSONB round trip.
        let d = domains
            .create(SdtmProjectDomainNew {
                sdtm_project_version_id: v.id,
                domain_code: "AE".into(),
                category: DomainCategory::Events,
                descriptions: Vec::new(),
                key_variables: vec!["AESEV".into(), "AETERM".into()],
            })
            .await
            .unwrap();
        assert_eq!(
            d.key_variables,
            vec!["AESEV".to_string(), "AETERM".to_string()]
        );

        let by_id = domains.find_by_id(d.id).await.unwrap();
        assert_eq!(
            by_id.key_variables,
            vec!["AESEV".to_string(), "AETERM".to_string()]
        );
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL"]
async fn domain_update_changes_code_and_key_variables() {
    with_pool(|pool| async move {
        let versions = SdtmProjectVersionRepoPg::new(pool.clone());
        let domains = SdtmProjectDomainRepoPg::new(pool);
        let v = versions
            .create(SdtmProjectVersionNew {
                project_code: unique("p"),
                version_name: unique("v"),
            })
            .await
            .unwrap();
        let d = domains
            .create(SdtmProjectDomainNew {
                sdtm_project_version_id: v.id,
                domain_code: "AE".into(),
                category: DomainCategory::Events,
                descriptions: Vec::new(),
                key_variables: Vec::new(),
            })
            .await
            .unwrap();

        let updated = domains
            .update(SdtmProjectDomainUpdate {
                id: d.id,
                domain_code: Some("AE2".into()),
                category: Some(DomainCategory::Findings),
                descriptions: None,
                key_variables: Some(vec!["AESEV".into(), "AETERM".into()]),
            })
            .await
            .expect("domain update");
        assert_eq!(updated.domain_code, "AE2");
        assert_eq!(
            updated.key_variables,
            vec!["AESEV".to_string(), "AETERM".to_string()]
        );

        // `Some(vec![])` clears the column rather than leaving it.
        let cleared = domains
            .update(SdtmProjectDomainUpdate {
                id: d.id,
                domain_code: None,
                category: None,
                descriptions: None,
                key_variables: Some(Vec::new()),
            })
            .await
            .unwrap();
        assert!(cleared.key_variables.is_empty());
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL"]
async fn domain_update_of_missing_row_is_not_found() {
    with_pool(|pool| async move {
        let domains = SdtmProjectDomainRepoPg::new(pool);
        let err = domains
            .update(SdtmProjectDomainUpdate {
                id: 999_999,
                domain_code: Some("AE".into()),
                category: None,
                descriptions: None,
                key_variables: None,
            })
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            DomainError::SdtmProjectDomainNotFound(999_999)
        ));
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL"]
async fn domain_delete_of_missing_row_is_not_found() {
    with_pool(|pool| async move {
        let domains = SdtmProjectDomainRepoPg::new(pool);
        let err = domains.delete(999_999).await.unwrap_err();
        assert!(matches!(
            err,
            DomainError::SdtmProjectDomainNotFound(999_999)
        ));
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL"]
async fn domain_create_when_version_vanished_maps_to_fk_not_found() {
    with_pool(|pool| async move {
        let versions = SdtmProjectVersionRepoPg::new(pool.clone());
        let domains = SdtmProjectDomainRepoPg::new(pool.clone());
        let v = versions
            .create(SdtmProjectVersionNew {
                project_code: unique("p"),
                version_name: unique("v"),
            })
            .await
            .unwrap();

        // Delete the version behind the repository's back, so the
        // usecase's existence check cannot catch it. The insert must
        // then fail on the foreign key — and report it as a missing
        // parent (404), not as a repository failure (500).
        sqlx::query("DELETE FROM sdtm_project_versions WHERE id = $1")
            .bind(v.id)
            .execute(&pool)
            .await
            .expect("delete version behind the repository's back");

        let err = domains
            .create(SdtmProjectDomainNew {
                sdtm_project_version_id: v.id,
                domain_code: "AE".into(),
                category: DomainCategory::Events,
                descriptions: Vec::new(),
                key_variables: Vec::new(),
            })
            .await
            .unwrap_err();
        assert!(
            matches!(err, DomainError::FkSdtmProjectVersionNotFound(id) if id == v.id),
            "expected FkSdtmProjectVersionNotFound({}), got {err:?}",
            v.id
        );
    })
    .await
}

// ---- cascade --------------------------------------------------------------

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL"]
async fn version_delete_cascades_to_the_whole_subtree() {
    with_pool(|pool| async move {
        let versions = SdtmProjectVersionRepoPg::new(pool.clone());
        let domains = SdtmProjectDomainRepoPg::new(pool.clone());
        let variables = SdtmProjectVariableRepoPg::new(pool);

        let v = versions
            .create(SdtmProjectVersionNew {
                project_code: unique("p"),
                version_name: unique("v"),
            })
            .await
            .unwrap();
        let d = domains
            .create(SdtmProjectDomainNew {
                sdtm_project_version_id: v.id,
                domain_code: "AE".into(),
                category: DomainCategory::Events,
                descriptions: Vec::new(),
                key_variables: Vec::new(),
            })
            .await
            .unwrap();
        let var = variables
            .create(SdtmProjectVariableNew {
                sdtm_project_domain_id: d.id,
                variable_code: "AETERM".into(),
                variable_controlled: None,
                variable_type: SdtmVariableType::Character,
                variable_core: SdtmVariableCore::Req,
                variable_role: Some(SdtmRole::Topic),
                variable_sequence: 1,
                descriptions: Vec::new(),
                conversion_rules: Vec::new(),
            })
            .await
            .unwrap();

        versions.delete(v.id).await.expect("delete version");

        assert!(domains.find_by_id(d.id).await.is_err());
        assert!(variables.find_by_id(var.id).await.is_err());
    })
    .await
}

// ---- SdtmProjectVariable --------------------------------------------------

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL"]
async fn variable_create_then_list_orders_by_sequence() {
    with_pool(|pool| async move {
        let versions = SdtmProjectVersionRepoPg::new(pool.clone());
        let domains = SdtmProjectDomainRepoPg::new(pool.clone());
        let variables = SdtmProjectVariableRepoPg::new(pool);
        let v = versions
            .create(SdtmProjectVersionNew {
                project_code: unique("p"),
                version_name: unique("v"),
            })
            .await
            .unwrap();
        let d = domains
            .create(SdtmProjectDomainNew {
                sdtm_project_version_id: v.id,
                domain_code: "AE".into(),
                category: DomainCategory::Events,
                descriptions: Vec::new(),
                key_variables: Vec::new(),
            })
            .await
            .unwrap();

        variables
            .create(SdtmProjectVariableNew {
                sdtm_project_domain_id: d.id,
                variable_code: "AESEV".into(),
                variable_controlled: None,
                variable_type: SdtmVariableType::Character,
                variable_core: SdtmVariableCore::Req,
                variable_role: None,
                variable_sequence: 20,
                descriptions: Vec::new(),
                conversion_rules: Vec::new(),
            })
            .await
            .unwrap();
        variables
            .create(SdtmProjectVariableNew {
                sdtm_project_domain_id: d.id,
                variable_code: "AETERM".into(),
                variable_controlled: None,
                variable_type: SdtmVariableType::Character,
                variable_core: SdtmVariableCore::Req,
                variable_role: None,
                variable_sequence: 10,
                descriptions: Vec::new(),
                conversion_rules: Vec::new(),
            })
            .await
            .unwrap();

        let listed = variables.list_by_domain(d.id).await.expect("list");
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].variable_code, "AETERM");
        assert_eq!(listed[1].variable_code, "AESEV");
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL"]
async fn variable_update_changes_sequence() {
    with_pool(|pool| async move {
        let versions = SdtmProjectVersionRepoPg::new(pool.clone());
        let domains = SdtmProjectDomainRepoPg::new(pool.clone());
        let variables = SdtmProjectVariableRepoPg::new(pool);
        let v = versions
            .create(SdtmProjectVersionNew {
                project_code: unique("p"),
                version_name: unique("v"),
            })
            .await
            .unwrap();
        let d = domains
            .create(SdtmProjectDomainNew {
                sdtm_project_version_id: v.id,
                domain_code: "AE".into(),
                category: DomainCategory::Events,
                descriptions: Vec::new(),
                key_variables: Vec::new(),
            })
            .await
            .unwrap();
        let var = variables
            .create(SdtmProjectVariableNew {
                sdtm_project_domain_id: d.id,
                variable_code: "AETERM".into(),
                variable_controlled: None,
                variable_type: SdtmVariableType::Character,
                variable_core: SdtmVariableCore::Req,
                variable_role: None,
                variable_sequence: 1,
                descriptions: Vec::new(),
                conversion_rules: Vec::new(),
            })
            .await
            .unwrap();

        let updated = variables
            .update(SdtmProjectVariableUpdate {
                id: var.id,
                variable_code: Some("AETERMX".into()),
                variable_controlled: None,
                variable_type: None,
                variable_core: None,
                variable_role: None,
                variable_sequence: Some(12),
                descriptions: None,
                conversion_rules: None,
            })
            .await
            .expect("variable update");
        assert_eq!(updated.variable_code, "AETERMX");
        assert_eq!(updated.variable_sequence, 12);
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL"]
async fn variable_update_of_missing_row_is_not_found() {
    with_pool(|pool| async move {
        let variables = SdtmProjectVariableRepoPg::new(pool);
        let err = variables
            .update(SdtmProjectVariableUpdate {
                id: 999_999,
                variable_code: Some("AETERM".into()),
                variable_controlled: None,
                variable_type: None,
                variable_core: None,
                variable_role: None,
                variable_sequence: None,
                descriptions: None,
                conversion_rules: None,
            })
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            DomainError::SdtmProjectVariableNotFound(999_999)
        ));
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL"]
async fn variable_delete_of_missing_row_is_not_found() {
    with_pool(|pool| async move {
        let variables = SdtmProjectVariableRepoPg::new(pool);
        let err = variables.delete(999_999).await.unwrap_err();
        assert!(matches!(
            err,
            DomainError::SdtmProjectVariableNotFound(999_999)
        ));
    })
    .await
}

// ---- SdtmProjectVariableOverride ------------------------------------------

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL"]
async fn domain_round_trips_category_and_descriptions() {
    with_pool(|pool| async move {
        let versions = SdtmProjectVersionRepoPg::new(pool.clone());
        let domains = SdtmProjectDomainRepoPg::new(pool);
        let v = versions
            .create(SdtmProjectVersionNew {
                project_code: unique("p"),
                version_name: unique("v"),
            })
            .await
            .unwrap();

        let description = SdtmDomainDescription {
            lang: "en".into(),
            details: SdtmDomainDescriptionDetail {
                description: "Adverse events".into(),
                structure: "One record per AE".into(),
            },
        };
        let d = domains
            .create(SdtmProjectDomainNew {
                sdtm_project_version_id: v.id,
                domain_code: "AE".into(),
                category: DomainCategory::Events,
                descriptions: vec![description.clone()],
                key_variables: Vec::new(),
            })
            .await
            .expect("domain create");
        assert_eq!(d.category, DomainCategory::Events);
        assert_eq!(d.descriptions.len(), 1);

        let by_id = domains.find_by_id(d.id).await.expect("find by id");
        assert_eq!(by_id.category, DomainCategory::Events);
        assert_eq!(by_id.descriptions.len(), 1);
        assert_eq!(by_id.descriptions[0].details.description, "Adverse events");
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL"]
async fn conversion_rules_round_trip_through_the_repository() {
    with_pool(|pool| async move {
        let versions = SdtmProjectVersionRepoPg::new(pool.clone());
        let domains = SdtmProjectDomainRepoPg::new(pool.clone());
        let variables = SdtmProjectVariableRepoPg::new(pool);
        let v = versions
            .create(SdtmProjectVersionNew {
                project_code: unique("p"),
                version_name: unique("v"),
            })
            .await
            .unwrap();
        let d = domains
            .create(SdtmProjectDomainNew {
                sdtm_project_version_id: v.id,
                domain_code: "AE".into(),
                category: DomainCategory::Events,
                descriptions: Vec::new(),
                key_variables: Vec::new(),
            })
            .await
            .unwrap();

        let rules = vec![
            ConversionRule {
                source: VariableSource::Protocol,
                rule: "AESEV".into(),
            },
            ConversionRule {
                source: VariableSource::Crf(Some(FromCrf {
                    form_code: "AE".into(),
                })),
                rule: "AETERM".into(),
            },
        ];
        let var = variables
            .create(SdtmProjectVariableNew {
                sdtm_project_domain_id: d.id,
                variable_code: "AETERM".into(),
                variable_controlled: None,
                variable_type: SdtmVariableType::Character,
                variable_core: SdtmVariableCore::Req,
                variable_role: None,
                variable_sequence: 11,
                descriptions: Vec::new(),
                conversion_rules: rules.clone(),
            })
            .await
            .expect("variable create");

        let by_id = variables.find_by_id(var.id).await.expect("find by id");
        assert_eq!(by_id.conversion_rules, rules);
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL"]
async fn variable_without_controlled_terminology_round_trips_as_null() {
    with_pool(|pool| async move {
        let versions = SdtmProjectVersionRepoPg::new(pool.clone());
        let domains = SdtmProjectDomainRepoPg::new(pool.clone());
        let variables = SdtmProjectVariableRepoPg::new(pool);
        let v = versions
            .create(SdtmProjectVersionNew {
                project_code: unique("p"),
                version_name: unique("v"),
            })
            .await
            .unwrap();
        let d = domains
            .create(SdtmProjectDomainNew {
                sdtm_project_version_id: v.id,
                domain_code: "AE".into(),
                category: DomainCategory::Events,
                descriptions: Vec::new(),
                key_variables: Vec::new(),
            })
            .await
            .unwrap();

        let var = variables
            .create(SdtmProjectVariableNew {
                sdtm_project_domain_id: d.id,
                variable_code: "AETERM".into(),
                variable_controlled: None,
                variable_type: SdtmVariableType::Character,
                variable_core: SdtmVariableCore::Req,
                variable_role: None,
                variable_sequence: 11,
                descriptions: Vec::new(),
                conversion_rules: Vec::new(),
            })
            .await
            .expect("variable create");

        let by_id = variables.find_by_id(var.id).await.expect("find by id");
        assert_eq!(by_id.variable_controlled, None);
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL"]
async fn variable_controlled_round_trips_through_the_repository() {
    with_pool(|pool| async move {
        let versions = SdtmProjectVersionRepoPg::new(pool.clone());
        let domains = SdtmProjectDomainRepoPg::new(pool.clone());
        let variables = SdtmProjectVariableRepoPg::new(pool);
        let v = versions
            .create(SdtmProjectVersionNew {
                project_code: unique("p"),
                version_name: unique("v"),
            })
            .await
            .unwrap();
        let d = domains
            .create(SdtmProjectDomainNew {
                sdtm_project_version_id: v.id,
                domain_code: "AE".into(),
                category: DomainCategory::Events,
                descriptions: Vec::new(),
                key_variables: Vec::new(),
            })
            .await
            .unwrap();

        let controlled = SdtmVariableControlled {
            code: Some(vec!["C66731".to_string()]),
            format: None,
        };
        let var = variables
            .create(SdtmProjectVariableNew {
                sdtm_project_domain_id: d.id,
                variable_code: "AETERM".into(),
                variable_controlled: Some(controlled.clone()),
                variable_type: SdtmVariableType::Character,
                variable_core: SdtmVariableCore::Req,
                variable_role: None,
                variable_sequence: 11,
                descriptions: Vec::new(),
                conversion_rules: Vec::new(),
            })
            .await
            .expect("variable create");

        let by_id = variables.find_by_id(var.id).await.expect("find by id");
        assert_eq!(by_id.variable_controlled, Some(controlled));
    })
    .await
}
