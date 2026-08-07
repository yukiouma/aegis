//! OpenAPI schema lock for the `openapi` feature.
//!
//! Assertions are made against the serialized schema string rather than
//! a hand-built nested structure, so they survive utoipa's internal
//! layout changes while still pinning the two things that matter: the
//! property names, and that `chrono` timestamps are documented as
//! `date-time`.
#![cfg(feature = "openapi")]

use apis::auth::{AuthApiError, AuthClaims, TokenPair, UserCredentialView};
use apis::user::{Role, UserApiError, UserView};
use utoipa::PartialSchema;

/// Serialize a type's schema to a JSON string for substring assertions.
fn schema_of<T: PartialSchema>() -> String {
    serde_json::to_string(&T::schema()).expect("schema must serialize")
}

#[test]
fn user_view_schema_uses_camel_case_properties() {
    let s = schema_of::<UserView>();
    for expected in ["createdAt", "updatedAt"] {
        assert!(s.contains(expected), "schema missing `{expected}`: {s}");
    }
    for leaked in ["created_at", "updated_at"] {
        assert!(!s.contains(leaked), "schema leaked `{leaked}`: {s}");
    }
}

#[test]
fn user_view_schema_documents_timestamps_as_date_time() {
    // This is the test that catches a missing `utoipa/chrono` feature:
    // without it `DateTime<Utc>` renders as an empty object.
    let s = schema_of::<UserView>();
    assert!(
        s.contains("date-time"),
        "DateTime<Utc> not documented as date-time — is the `chrono` \
         feature missing from the utoipa dependency? {s}"
    );
}

#[test]
fn auth_schemas_use_camel_case_properties() {
    let token_pair = schema_of::<TokenPair>();
    assert!(token_pair.contains("accessToken"), "{token_pair}");
    assert!(token_pair.contains("refreshToken"), "{token_pair}");
    assert!(!token_pair.contains("access_token"), "{token_pair}");

    let claims = schema_of::<AuthClaims>();
    assert!(claims.contains("tokenVersion"), "{claims}");
    assert!(!claims.contains("token_version"), "{claims}");

    let cred = schema_of::<UserCredentialView>();
    assert!(cred.contains("userCode"), "{cred}");
    assert!(cred.contains("passwordHash"), "{cred}");
    assert!(!cred.contains("user_code"), "{cred}");
}

#[test]
fn role_schema_lists_lowercase_values() {
    let s = schema_of::<Role>();
    for expected in ["\"root\"", "\"admin\"", "\"general\""] {
        assert!(s.contains(expected), "schema missing {expected}: {s}");
    }
    assert!(!s.contains("\"Root\""), "schema leaked Rust spelling: {s}");
}

#[test]
fn error_enums_produce_schemas() {
    // The primary value of this test is that the derives compile; the
    // assertions guard against an empty or degenerate schema.
    let user = schema_of::<UserApiError>();
    assert!(user.contains("NotFound"), "{user}");

    let auth = schema_of::<AuthApiError>();
    assert!(auth.contains("InvalidCredentials"), "{auth}");
}