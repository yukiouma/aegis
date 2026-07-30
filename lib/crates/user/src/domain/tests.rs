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
        "hash".into(),
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
        "hash".into(),
    )
    .unwrap_err();
    assert!(matches!(err, DomainError::EmptyName));
}

#[test]
fn new_user_rejects_empty_password() {
    let err = User::new(
        1,
        "u1".into(),
        "Alice".into(),
        Role::Admin,
        true,
        test_now(),
        test_now(),
        "".into(),
    )
    .unwrap_err();
    assert!(matches!(err, DomainError::EmptyPassword));
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
        "argon-hash".into(),
    )
    .expect("valid user should construct");
    assert_eq!(user.id, 42);
    assert_eq!(user.code, "u42");
    assert_eq!(user.name, "Alice");
    assert_eq!(user.role, Role::Admin);
    assert!(user.active);
    assert_eq!(user.password, "argon-hash");
}

#[test]
fn password_is_not_exposed_via_public_projection() {
    let user = User::new(
        7,
        "u7".into(),
        "Bob".into(),
        Role::General,
        true,
        test_now(),
        test_now(),
        "secret-hash".into(),
    )
    .unwrap();

    // The Debug output must never include the password hash.
    let debug = format!("{user:?}");
    assert!(
        !debug.contains("secret-hash"),
        "password leaked via Debug: {debug}"
    );

    // There is no public accessor for the password field.
    // We assert this structurally: `password` is not part of the
    // pub-projection via the fact that the field is `pub(crate)`.
    // A compile-time check is exercised by the fact that no method
    // exists on `User` to fetch it from outside the crate.
}
