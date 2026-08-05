use super::*;

use chrono::{TimeZone, Utc};

fn test_now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 7, 29, 0, 0, 0).unwrap()
}

#[test]
fn role_as_str_maps_to_lowercase() {
    assert_eq!(Role::Root.as_str(), "root");
    assert_eq!(Role::Admin.as_str(), "admin");
    assert_eq!(Role::General.as_str(), "general");
}

#[test]
fn try_from_str_parses_known_values_lowercase() {
    assert_eq!(Role::try_from("root").unwrap(), Role::Root);
    assert_eq!(Role::try_from("admin").unwrap(), Role::Admin);
    assert_eq!(Role::try_from("general").unwrap(), Role::General);
}

#[test]
fn try_from_str_rejects_unknown_value() {
    let err = Role::try_from("superuser").unwrap_err();
    assert!(matches!(err, DomainError::InvalidRole(ref s) if s == "superuser"));
}

#[test]
fn try_from_str_rejects_empty_string() {
    let err = Role::try_from("").unwrap_err();
    assert!(matches!(err, DomainError::InvalidRole(_)));
}

#[test]
fn new_user_rejects_empty_code() {
    let err = User::new(
        1,
        "".into(),
        "Alice".into(),
        Role::Admin,
        true,
        test_now(),
        test_now(),
    )
    .unwrap_err();
    assert!(matches!(err, DomainError::EmptyCode));
}

#[test]
fn new_user_rejects_empty_name() {
    let err = User::new(
        1,
        "u1".into(),
        "".into(),
        Role::Admin,
        true,
        test_now(),
        test_now(),
    )
    .unwrap_err();
    assert!(matches!(err, DomainError::EmptyName));
}

#[test]
fn new_user_accepts_valid_input() {
    let user = User::new(
        42,
        "u42".into(),
        "Alice".into(),
        Role::Admin,
        true,
        test_now(),
        test_now(),
    )
    .expect("valid user should construct");
    assert_eq!(user.id, 42);
    assert_eq!(user.code, "u42");
    assert_eq!(user.name, "Alice");
    assert_eq!(user.role, Role::Admin);
    assert!(user.active);
}
