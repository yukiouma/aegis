//! Schema + row-conversion tests for the PostgreSQL adapter.
//!
//! These tests do NOT require a live database. They read the migration
//! files and the row-bridge impls directly. Live-database round-trips
//! live in `tests/integration_persistence.rs` and are `#[ignore]`-gated.

use std::fs;
use std::path::PathBuf;

fn migration_path(name: &str) -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("migrations").join(name)
}

fn load_migration(name: &str) -> String {
    fs::read_to_string(migration_path(name))
        .unwrap_or_else(|_| panic!("migration file {name} must exist"))
}

fn create_table_block(sql: &str) -> String {
    let start = sql.find("CREATE TABLE").expect("CREATE TABLE");
    let close = sql[start..]
        .find(");")
        .expect("CREATE TABLE terminated by `);`");
    sql[start..start + close + 2].to_string()
}

#[test]
fn products_migration_creates_products_table() {
    let sql = load_migration("0001_create_products.sql");
    let block = create_table_block(&sql);
    assert!(block.contains("CREATE TABLE") && block.contains("products"));
}

#[test]
fn products_migration_has_required_columns() {
    let block = create_table_block(&load_migration("0001_create_products.sql"));
    let upper = block.to_uppercase();
    for required in [
        "ID INTEGER",
        "CODE TEXT",
        "NAME TEXT",
        "DESCRIPTION TEXT",
        "ACTIVE BOOLEAN",
        "CREATED_AT TIMESTAMPTZ NOT NULL DEFAULT NOW()",
        "UPDATED_AT TIMESTAMPTZ NOT NULL DEFAULT NOW()",
    ] {
        assert!(
            upper.contains(&required.to_uppercase()),
            "products table must include `{required}`; got:\n{block}"
        );
    }
}

#[test]
fn products_migration_makes_code_unique_and_not_null() {
    let block = create_table_block(&load_migration("0001_create_products.sql"));
    assert!(
        block.contains("UNIQUE (code)") || block.contains("UNIQUE(\"code\")"),
        "expected UNIQUE on code; got:\n{block}"
    );
    assert!(block.to_uppercase().contains("NOT NULL"));
}

#[test]
fn products_migration_has_updated_at_trigger() {
    let sql = load_migration("0001_create_products.sql");
    assert!(sql.contains("CREATE TRIGGER products_set_updated_at"));
    assert!(sql.contains("BEFORE UPDATE ON products"));
}

#[test]
fn projects_migration_creates_projects_table() {
    let sql = load_migration("0002_create_projects.sql");
    let block = create_table_block(&sql);
    assert!(block.contains("CREATE TABLE") && block.contains("projects"));
}

#[test]
fn projects_migration_references_products() {
    let block = create_table_block(&load_migration("0002_create_projects.sql"));
    let upper = block.to_uppercase();
    assert!(
        upper.contains("PRODUCT_ID INTEGER"),
        "projects.product_id must be INTEGER; got:\n{block}"
    );
    assert!(
        upper.contains("REFERENCES PRODUCTS(ID)"),
        "projects.product_id must FK to products(id); got:\n{block}"
    );
}

#[test]
fn projects_migration_has_updated_at_trigger() {
    let sql = load_migration("0002_create_projects.sql");
    assert!(sql.contains("CREATE TRIGGER projects_set_updated_at"));
    assert!(sql.contains("BEFORE UPDATE ON projects"));
}

#[test]
fn project_members_migration_has_composite_pk_and_checks() {
    let sql = load_migration("0002_create_projects.sql");
    let upper = sql.to_uppercase();
    let start = upper
        .find("CREATE TABLE PROJECT_MEMBERS")
        .expect("project_members");
    let close = upper[start..].find(");").expect("close") + start + 2;
    let block = &sql[start..close];
    let upper_block = block.to_uppercase();
    assert!(
        upper_block.contains("PRIMARY KEY (PROJECT_ID, TEAM_TYPE, ROLE_TYPE, USER_CODE)"),
        "project_members PK must be the composite; got:\n{block}"
    );
    assert!(upper_block.contains("CHECK"));
    assert!(upper_block.contains("'MEMBERS'") && upper_block.contains("'UNBLIND_MEMBERS'"));
    assert!(upper_block.contains("'LEADER'") && upper_block.contains("'WORKER'"));
}

#[test]
fn project_members_migration_cascades_on_delete() {
    let sql = load_migration("0002_create_projects.sql");
    assert!(
        sql.contains("REFERENCES projects(id) ON DELETE CASCADE"),
        "project_members FK must cascade on delete"
    );
}

#[cfg(test)]
mod row_tests {
    use chrono::{TimeZone, Utc};

    use super::super::row::{ProductRow, ProjectMemberRow, ProjectRow};
    use crate::domain::{ProjectMember, RoleType, TeamType};

    fn ts() -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 9, 0, 0, 0).unwrap()
    }

    #[test]
    fn product_row_converts_to_product() {
        let row = ProductRow {
            id: 1,
            code: "p1".into(),
            name: "Widget".into(),
            description: "desc".into(),
            active: true,
            created_at: ts(),
            updated_at: ts(),
        };
        let p: crate::domain::Product = row.try_into().expect("convert");
        assert_eq!(p.id, 1);
        assert_eq!(p.code, "p1");
    }

    #[test]
    fn project_row_converts_to_project_with_empty_members() {
        let row = ProjectRow {
            id: 1,
            code: "proj1".into(),
            description: "".into(),
            product_id: 7,
            active: true,
            created_at: ts(),
            updated_at: ts(),
        };
        let p: crate::domain::Project = row.try_into().expect("convert");
        assert_eq!(p.product_id, 7);
        assert_eq!(p.members, ProjectMember::default());
        assert_eq!(p.unblind_members, ProjectMember::default());
    }

    #[test]
    fn project_member_row_carries_team_and_role_strings() {
        let row = ProjectMemberRow {
            project_id: 1,
            team_type: "members".into(),
            role_type: "leader".into(),
            user_code: "u1".into(),
        };
        assert_eq!(
            TeamType::try_from(row.team_type.as_str()).unwrap(),
            TeamType::Members
        );
        assert_eq!(
            RoleType::try_from(row.role_type.as_str()).unwrap(),
            RoleType::Leader
        );
    }
}
