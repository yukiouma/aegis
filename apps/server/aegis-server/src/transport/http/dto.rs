//! Wire-level DTOs for the HTTP transport.
//!
//! Each wire DTO is a thin Rust struct with `Serialize`,
//! `Deserialize`, and `ToSchema`. Field names are `snake_case` to
//! match the apis surface. Handler code translates JSON ↔ apis DTOs
//! at the boundary; the apis crate deliberately has no serde /
//! utoipa derives.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// -- requests -------------------------------------------------------------

#[derive(Serialize, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub code: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct LoginDomainRequest {
    pub code: String,
    pub domain_name: String,
    pub hostname: String,
    pub sid: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct LogoutRequest {
    pub refresh_token: String,
}

// -- responses ------------------------------------------------------------

#[derive(Serialize, Deserialize, ToSchema)]
pub struct TokenPairResponse {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct AccessTokenResponse {
    pub access_token: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct LogoutResponse {}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct AuthClaimsResponse {
    pub code: String,
    pub role: Role,
    pub token_version: u32,
}

// -- Role -----------------------------------------------------------------

/// Wire-level mirror of `apis::user::Role`. The two enums have
/// identical variants; the conversion is a single 3-arm `match` in
/// `auth.rs`. Kept separate so the apis crate stays free of
/// serde / utoipa derives.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Root,
    Admin,
    General,
}

impl From<apis::user::Role> for Role {
    fn from(r: apis::user::Role) -> Self {
        match r {
            apis::user::Role::Root => Role::Root,
            apis::user::Role::Admin => Role::Admin,
            apis::user::Role::General => Role::General,
        }
    }
}

impl From<Role> for apis::user::Role {
    fn from(r: Role) -> Self {
        match r {
            Role::Root => apis::user::Role::Root,
            Role::Admin => apis::user::Role::Admin,
            Role::General => apis::user::Role::General,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_request_roundtrip() {
        let json = r#"{"code":"u1","password":"p"}"#;
        let req: LoginRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.code, "u1");
        assert_eq!(req.password, "p");
        let out = serde_json::to_string(&req).unwrap();
        assert_eq!(out, json);
    }

    #[test]
    fn login_domain_request_roundtrip() {
        let json = r#"{"code":"u1","domain_name":"d","hostname":"h","sid":"s"}"#;
        let req: LoginDomainRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.code, "u1");
        assert_eq!(req.domain_name, "d");
        assert_eq!(req.hostname, "h");
        assert_eq!(req.sid, "s");
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn refresh_request_roundtrip() {
        let json = r#"{"refresh_token":"r"}"#;
        let req: RefreshRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.refresh_token, "r");
    }

    #[test]
    fn logout_request_roundtrip() {
        let json = r#"{"refresh_token":"r"}"#;
        let req: LogoutRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.refresh_token, "r");
    }

    #[test]
    fn token_pair_response_roundtrip() {
        let json = r#"{"access_token":"a","refresh_token":"r"}"#;
        let res: TokenPairResponse = serde_json::from_str(json).unwrap();
        assert_eq!(res.access_token, "a");
        assert_eq!(res.refresh_token, "r");
    }

    #[test]
    fn access_token_response_roundtrip() {
        let json = r#"{"access_token":"a"}"#;
        let res: AccessTokenResponse = serde_json::from_str(json).unwrap();
        assert_eq!(res.access_token, "a");
    }

    #[test]
    fn logout_response_roundtrip() {
        let res: LogoutResponse = serde_json::from_str("{}").unwrap();
        let out = serde_json::to_string(&res).unwrap();
        assert_eq!(out, "{}");
    }

    #[test]
    fn auth_claims_response_roundtrip() {
        let json = r#"{"code":"u1","role":"admin","token_version":7}"#;
        let res: AuthClaimsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(res.code, "u1");
        assert!(matches!(res.role, Role::Admin));
        assert_eq!(res.token_version, 7);
        assert_eq!(serde_json::to_string(&res).unwrap(), json);
    }

    #[test]
    fn role_round_trip_all_variants() {
        for r in [Role::Root, Role::Admin, Role::General] {
            let s = serde_json::to_string(&r).unwrap();
            let back: Role = serde_json::from_str(&s).unwrap();
            assert_eq!(format!("{r:?}"), format!("{back:?}"));
        }
    }

    #[test]
    fn role_from_apis_role_all_variants() {
        assert!(matches!(Role::from(apis::user::Role::Root), Role::Root));
        assert!(matches!(Role::from(apis::user::Role::Admin), Role::Admin));
        assert!(matches!(Role::from(apis::user::Role::General), Role::General));
    }
}