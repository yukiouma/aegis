use chrono::{TimeZone, Utc};

use super::*;

fn test_now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 9, 0, 0, 0).unwrap()
}

#[test]
fn team_type_as_str_maps_to_lowercase() {
    assert_eq!(TeamType::Members.as_str(), "members");
    assert_eq!(TeamType::UnblindMembers.as_str(), "unblind_members");
}

#[test]
fn team_type_try_from_str_parses_known_values() {
    assert_eq!(TeamType::try_from("members").unwrap(), TeamType::Members);
    assert_eq!(
        TeamType::try_from("unblind_members").unwrap(),
        TeamType::UnblindMembers
    );
}

#[test]
fn team_type_try_from_str_rejects_unknown_value() {
    let err = TeamType::try_from("admins").unwrap_err();
    assert!(matches!(err, DomainError::UnknownTeamType(ref s) if s == "admins"));
}

#[test]
fn role_type_as_str_maps_to_lowercase() {
    assert_eq!(RoleType::Leader.as_str(), "leader");
    assert_eq!(RoleType::Worker.as_str(), "worker");
}

#[test]
fn role_type_try_from_str_parses_known_values() {
    assert_eq!(RoleType::try_from("leader").unwrap(), RoleType::Leader);
    assert_eq!(RoleType::try_from("worker").unwrap(), RoleType::Worker);
}

#[test]
fn role_type_try_from_str_rejects_unknown_value() {
    let err = RoleType::try_from("admin").unwrap_err();
    assert!(matches!(err, DomainError::UnknownRoleType(ref s) if s == "admin"));
}

#[test]
fn project_member_accepts_clean_input() {
    let m = ProjectMember::new(vec!["u1".into()], vec!["u2".into()]).unwrap();
    assert_eq!(m.leaders, vec!["u1".to_string()]);
    assert_eq!(m.workers, vec!["u2".to_string()]);
}

#[test]
fn project_member_rejects_duplicate_leader() {
    let err = ProjectMember::new(vec!["u1".into(), "u1".into()], vec![]).unwrap_err();
    assert!(matches!(err, DomainError::DuplicateLeader(ref s) if s == "u1"));
}

#[test]
fn project_member_rejects_duplicate_worker() {
    let err = ProjectMember::new(vec![], vec!["u2".into(), "u2".into()]).unwrap_err();
    assert!(matches!(err, DomainError::DuplicateWorker(ref s) if s == "u2"));
}

#[test]
fn project_member_allows_same_code_in_leaders_and_workers() {
    let m = ProjectMember::new(vec!["u1".into()], vec!["u1".into()]).unwrap();
    assert_eq!(m.leaders, vec!["u1".to_string()]);
    assert_eq!(m.workers, vec!["u1".to_string()]);
}

#[test]
fn project_member_accepts_empty_lists() {
    let m = ProjectMember::new(vec![], vec![]).unwrap();
    assert!(m.leaders.is_empty());
    assert!(m.workers.is_empty());
}

#[test]
fn project_new_rejects_empty_code() {
    let m = ProjectMember::default();
    let err = Project::new(
        1,
        "".into(),
        "desc".into(),
        m.clone(),
        m,
        vec![],
        true,
        test_now(),
        test_now(),
    )
    .unwrap_err();
    assert!(matches!(err, DomainError::EmptyCode));
}

#[test]
fn project_new_accepts_valid_input() {
    let m = ProjectMember::default();
    let p = Project::new(
        9,
        "proj9".into(),
        "desc".into(),
        m.clone(),
        m,
        vec![],
        true,
        test_now(),
        test_now(),
    )
    .unwrap();
    assert_eq!(p.id, 9);
    assert_eq!(p.tags, vec![]);
}

#[test]
fn project_tag_new_rejects_empty_key() {
    let err = ProjectTag::new("".into(), "v".into()).unwrap_err();
    assert!(matches!(err, DomainError::EmptyTagKey));
}

#[test]
fn project_tag_new_rejects_empty_value() {
    let err = ProjectTag::new("k".into(), "   ".into()).unwrap_err();
    assert!(matches!(err, DomainError::EmptyTagValue));
}

#[test]
fn project_tag_new_accepts_valid_input() {
    let t = ProjectTag::new("Product".into(), "DEMO-001".into()).unwrap();
    assert_eq!(t.key, "Product");
    assert_eq!(t.value, "DEMO-001");
}