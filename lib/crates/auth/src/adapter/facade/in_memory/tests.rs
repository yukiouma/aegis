//! Unit tests for `AuthServiceImpl`.
//!
//! Wires the adapter on top of `MockUserCredentialsRepo` +
//! `MockDomainIdentityRepo` + `FakeUserService` so the public-facing
//! `AuthService` surface is exercised without PostgreSQL.

use std::time::Duration;

use apis::auth::{
    AuthApiError, AuthService, LoginWithDomainUserInfoRequest, LoginWithPasswordRequest,
    LogoutRequest, RefreshRequest, VerifyRequest,
};
use apis::user::Role as ApiRole;

use crate::usecase::tests::{
    hash_password, FakeUserService, MockDomainIdentityRepo, MockUserCredentialsRepo,
};

use super::service::AuthServiceImpl;

fn make_service(
    creds: MockUserCredentialsRepo,
    ids: MockDomainIdentityRepo,
    users: FakeUserService,
) -> AuthServiceImpl<MockUserCredentialsRepo, MockDomainIdentityRepo> {
    let cfg = crate::usecase::AuthUsecaseConfig {
        credentials: creds,
        identities: ids,
        user_service: std::sync::Arc::new(users),
        signing_key: b"0123456789abcdef0123456789abcdef".to_vec(),
        access_ttl: Duration::from_secs(60),
        refresh_ttl: Duration::from_secs(3600),
    };
    AuthServiceImpl::new(crate::usecase::AuthUsecase::new(cfg))
}

#[tokio::test]
async fn service_impl_can_be_constructed() {
    let creds = MockUserCredentialsRepo::default();
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    let _svc = make_service(creds, ids, users);
}

#[tokio::test]
async fn login_with_password_returns_token_pair_for_valid_credentials() {
    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", &hash_password("hunter2"), 1);
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    users.seed("u1", ApiRole::Admin, true);
    let svc = make_service(creds, ids, users);

    let pair = svc
        .login_with_password(LoginWithPasswordRequest {
            code: "u1".into(),
            password: "hunter2".into(),
        })
        .await
        .expect("login succeeds");
    assert!(!pair.access_token.is_empty());
    assert!(!pair.refresh_token.is_empty());
}

#[tokio::test]
async fn login_with_password_returns_invalid_credentials_for_wrong_password() {
    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", &hash_password("hunter2"), 1);
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    users.seed("u1", ApiRole::Admin, true);
    let svc = make_service(creds, ids, users);

    let err = svc
        .login_with_password(LoginWithPasswordRequest {
            code: "u1".into(),
            password: "WRONG".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, AuthApiError::InvalidCredentials));
}

#[tokio::test]
async fn login_with_password_returns_inactive_when_user_is_disabled() {
    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", &hash_password("hunter2"), 1);
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    users.seed("u1", ApiRole::Admin, false);
    let svc = make_service(creds, ids, users);

    let err = svc
        .login_with_password(LoginWithPasswordRequest {
            code: "u1".into(),
            password: "hunter2".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, AuthApiError::Inactive));
}

#[tokio::test]
async fn login_with_domain_user_info_returns_not_found_for_unmatched_triple() {
    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", &hash_password("hunter2"), 1);
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    users.seed("u1", ApiRole::Admin, true);
    let svc = make_service(creds, ids, users);

    let err = svc
        .login_with_domain_user_info(LoginWithDomainUserInfoRequest {
            code: "u1".into(),
            domain_name: "DOM".into(),
            hostname: "host".into(),
            sid: "S-1-5".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, AuthApiError::NotFound));
}

#[tokio::test]
async fn verify_returns_claims_for_freshly_minted_access_token() {
    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", &hash_password("hunter2"), 9);
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    users.seed("u1", ApiRole::Root, true);
    let svc = make_service(creds, ids, users);

    let pair = svc
        .login_with_password(LoginWithPasswordRequest {
            code: "u1".into(),
            password: "hunter2".into(),
        })
        .await
        .expect("login succeeds");
    let claims = svc
        .verify(VerifyRequest {
            access_token: pair.access_token,
        })
        .await
        .expect("verify succeeds");
    assert_eq!(claims.code, "u1");
    assert_eq!(claims.role, ApiRole::Root);
    assert_eq!(claims.token_version, 9);
}

#[tokio::test]
async fn refresh_returns_new_access_token() {
    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", &hash_password("hunter2"), 1);
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    users.seed("u1", ApiRole::Admin, true);
    let svc = make_service(creds, ids, users);

    let pair = svc
        .login_with_password(LoginWithPasswordRequest {
            code: "u1".into(),
            password: "hunter2".into(),
        })
        .await
        .expect("login succeeds");
    let new = svc
        .refresh(RefreshRequest {
            refresh_token: pair.refresh_token,
        })
        .await
        .expect("refresh succeeds");
    assert!(!new.access_token.is_empty());
}

#[tokio::test]
async fn logout_echoes_the_user_code() {
    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", &hash_password("hunter2"), 1);
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    users.seed("u1", ApiRole::Admin, true);
    let svc = make_service(creds, ids, users);

    let ack = svc
        .logout(LogoutRequest { code: "u1".into() })
        .await
        .expect("logout succeeds");
    assert_eq!(ack.code, "u1");
}

#[tokio::test]
async fn auth_service_impl_is_object_safe() {
    let creds = MockUserCredentialsRepo::default();
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    let svc = make_service(creds, ids, users);
    let _boxed: Box<dyn AuthService> = Box::new(svc);
}

#[tokio::test]
async fn auth_service_impl_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    let creds = MockUserCredentialsRepo::default();
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    let svc = make_service(creds, ids, users);
    assert_send_sync::<AuthServiceImpl<MockUserCredentialsRepo, MockDomainIdentityRepo>>();
    assert_send_sync::<Box<dyn AuthService>>();
    let _ = svc;
}