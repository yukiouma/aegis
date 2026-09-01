use std::convert::TryFrom;
use std::fs;

use chrono::{DateTime, Utc};

use crate::domain::{Assignee, Mission, MissionKind, MissionRole};

use super::row::{AssigneeRow, MissionRow};

#[test]
fn mission_row_to_domain() {
    let row = MissionRow {
        id: 1,
        project_code: "p1".into(),
        mission_kind: "crf".into(),
        mission_code: "c1".into(),
        created_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        updated_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
    };
    let m: Mission = Mission::try_from((row, vec![])).unwrap();
    assert_eq!(m.id, 1);
    assert_eq!(m.mission_kind, MissionKind::Crf);
    assert_eq!(m.mission_code, "c1");
    assert!(m.assignees.is_empty());
}

#[test]
fn assignee_row_to_domain() {
    let row = AssigneeRow {
        id: 7,
        mission_id: 1,
        user_code: "u1".into(),
        role: "qc".into(),
        created_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        updated_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
    };
    let a: Assignee = Assignee::try_from(row).unwrap();
    assert_eq!(a.id, 7);
    assert_eq!(a.role, MissionRole::Qc);
}

#[test]
fn mission_row_rejects_unknown_kind() {
    let row = MissionRow {
        id: 1,
        project_code: "p1".into(),
        mission_kind: "not-a-kind".into(),
        mission_code: "c1".into(),
        created_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        updated_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
    };
    assert!(Mission::try_from((row, vec![])).is_err());
}

#[test]
fn mission_migration_has_natural_key_unique() {
    let sql = read_migration("0001_create_missions.sql");
    assert!(sql.contains("missions_natural_key"));
    assert!(sql.contains("UNIQUE (project_code, mission_kind, mission_code)"));
    assert!(sql.contains("missions_kind_check"));
    assert!(sql.contains("CHECK (mission_kind IN ('crf', 'sdtm', 'adam', 'tfl'))"));
    assert!(sql.contains("missions_set_updated_at"));
    assert!(sql.contains("BEFORE UPDATE ON missions"));
}

#[test]
fn assignee_migration_has_per_mission_unique_and_cascade() {
    let sql = read_migration("0002_create_assignees.sql");
    assert!(sql.contains("assignees_per_mission_unique"));
    assert!(sql.contains("UNIQUE (mission_id, user_code, role)"));
    assert!(sql.contains("assignees_role_check"));
    assert!(sql.contains("CHECK (role IN ('dev', 'qc'))"));
    assert!(sql.contains("assignees_set_updated_at"));
    assert!(sql.contains("BEFORE UPDATE ON assignees"));
    assert!(sql.contains("REFERENCES missions(id) ON DELETE CASCADE"));
}

fn read_migration(name: &str) -> String {
    let path = format!("{}/migrations/{}", env!("CARGO_MANIFEST_DIR"), name);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {}: {}", path, e))
}