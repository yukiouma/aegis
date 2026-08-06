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
fn new_user_credentials_rejects_empty_code() {
    let err =
        UserCredentials::new("".into(), "hash".into(), 1, test_now(), test_now()).unwrap_err();
    assert!(matches!(err, DomainError::EmptyCode));
}

#[test]
fn new_user_credentials_rejects_empty_password_hash() {
    let err = UserCredentials::new("u1".into(), "".into(), 1, test_now(), test_now()).unwrap_err();
    assert!(matches!(err, DomainError::EmptyPasswordHash));
}

#[test]
fn new_user_credentials_accepts_valid_input() {
    let c = UserCredentials::new("u1".into(), "hash".into(), 1, test_now(), test_now())
        .expect("valid credentials should construct");
    assert_eq!(c.code, "u1");
    assert_eq!(c.token_version, 1);
}

#[test]
fn user_credentials_debug_omits_password_hash() {
    let c = UserCredentials::for_repository("u1".into(), "hash".into(), 1, test_now(), test_now());
    let dbg = format!("{c:?}");
    assert!(
        !dbg.contains("hash"),
        "Debug must not leak password hash, got: {dbg}"
    );
    assert!(dbg.contains("u1"));
}

#[test]
fn new_domain_identity_rejects_empty_user_code() {
    let err =
        DomainIdentity::new("".into(), "DOM".into(), "host".into(), "S-1-5".into()).unwrap_err();
    assert!(matches!(err, DomainError::EmptyCode));
}

#[test]
fn new_domain_identity_rejects_empty_triple_components() {
    for (domain_name, hostname, sid) in [
        ("", "host", "S-1-5"),
        ("DOM", "", "S-1-5"),
        ("DOM", "host", ""),
    ] {
        let err = DomainIdentity::new("u1".into(), domain_name.into(), hostname.into(), sid.into())
            .unwrap_err();
        assert!(matches!(err, DomainError::EmptyPasswordHash));
    }
}

#[test]
fn new_domain_identity_accepts_valid_input() {
    let id = DomainIdentity::new("u1".into(), "DOM".into(), "host".into(), "S-1-5".into())
        .expect("valid identity should construct");
    assert_eq!(id.user_code, "u1");
    assert_eq!(id.domain_name, "DOM");
}
