//! Wire-contract lock for every DTO in `apis`.
//!
//! Asserts the exact JSON each type produces: camelCase keys, absent
//! snake_case spellings, lowercase `Role`, RFC 3339 timestamps, and
//! JSON -> T -> JSON round-trip stability. Deleting a
//! `#[serde(rename_all)]` attribute fails here rather than surfacing as
//! a broken client at integration time.

use apis::user::{CreateUserRequest, Role, UpdateUserRequest, UserView};
use serde_json::json;

/// Parse an RFC 3339 literal into the `DateTime<Utc>` the DTOs carry.
fn ts(s: &str) -> chrono::DateTime<chrono::Utc> {
    s.parse().expect("test timestamp must be valid RFC 3339")
}

/// JSON -> T -> JSON must be the identity.
///
/// The comparison is on the JSON rather than on the value because
/// several DTOs deliberately do not derive `PartialEq`; this keeps one
/// round-trip helper usable for all of them.
fn assert_round_trip<T>(value: serde_json::Value)
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    let parsed: T = serde_json::from_value(value.clone()).unwrap_or_else(|e| {
        panic!("deserialize failed for {}: {e}", std::any::type_name::<T>())
    });
    assert_eq!(
        serde_json::to_value(&parsed).unwrap(),
        value,
        "round-trip changed the JSON for {}",
        std::any::type_name::<T>()
    );
}

fn sample_user_view() -> UserView {
    UserView {
        id: 1,
        code: "u1".into(),
        name: "Alice".into(),
        role: Role::General,
        active: true,
        created_at: ts("2026-08-07T12:00:00Z"),
        updated_at: ts("2026-08-07T12:30:00Z"),
    }
}

fn sample_user_view_json() -> serde_json::Value {
    json!({
        "id": 1,
        "code": "u1",
        "name": "Alice",
        "role": "general",
        "active": true,
        "createdAt": "2026-08-07T12:00:00Z",
        "updatedAt": "2026-08-07T12:30:00Z"
    })
}

#[test]
fn user_view_serializes_to_camel_case() {
    assert_eq!(
        serde_json::to_value(sample_user_view()).unwrap(),
        sample_user_view_json()
    );
}

#[test]
fn user_view_omits_snake_case_keys() {
    let s = serde_json::to_string(&sample_user_view()).unwrap();
    for leaked in ["created_at", "updated_at"] {
        assert!(!s.contains(leaked), "snake_case key `{leaked}` leaked: {s}");
    }
}

#[test]
fn user_view_round_trips() {
    assert_round_trip::<UserView>(sample_user_view_json());
}

#[test]
fn role_serializes_lowercase() {
    assert_eq!(serde_json::to_value(Role::Root).unwrap(), json!("root"));
    assert_eq!(serde_json::to_value(Role::Admin).unwrap(), json!("admin"));
    assert_eq!(serde_json::to_value(Role::General).unwrap(), json!("general"));
}

#[test]
fn role_deserializes_lowercase() {
    assert_eq!(
        serde_json::from_value::<Role>(json!("root")).unwrap(),
        Role::Root
    );
    assert_eq!(
        serde_json::from_value::<Role>(json!("admin")).unwrap(),
        Role::Admin
    );
    assert_eq!(
        serde_json::from_value::<Role>(json!("general")).unwrap(),
        Role::General
    );
}

#[test]
fn role_rejects_rust_variant_spelling() {
    // Guards the `rename_all = "lowercase"` attribute: without it the
    // wire would speak "General" and disagree with `Role::as_str()`
    // and the Postgres CHECK constraint.
    assert!(serde_json::from_value::<Role>(json!("General")).is_err());
}

#[test]
fn create_user_request_round_trips() {
    assert_round_trip::<CreateUserRequest>(json!({
        "code": "u1",
        "name": "Alice",
        "role": "general"
    }));
}

#[test]
fn update_user_request_round_trips() {
    assert_round_trip::<UpdateUserRequest>(json!({
        "id": 1,
        "code": "u2",
        "name": "Bob",
        "role": "admin",
        "active": false
    }));
}

#[test]
fn update_user_request_treats_missing_optionals_as_none() {
    // Serde defaults a missing `Option<T>` field to `None` without
    // `#[serde(default)]`, so this type already behaves as a PATCH body.
    let req: UpdateUserRequest = serde_json::from_value(json!({ "id": 1 })).unwrap();
    assert_eq!(req.id, 1);
    assert!(req.code.is_none());
    assert!(req.name.is_none());
    assert!(req.role.is_none());
    assert!(req.active.is_none());
}