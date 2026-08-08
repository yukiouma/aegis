//! Unit tests for `AuthUsecase`.
//!
//! Mock repos and a `FakeUserService` (implementing the domain
//! `UserService` port) stand in for the real adapters so the usecase
//! can be exercised without PostgreSQL.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};

use argon2::PasswordVerifier;

use crate::domain::{
    DomainError, DomainIdentity, DomainIdentityRepository, Role, UserCredentials,
    UserCredentialsRepository, UserService, UserSummary,
};
use crate::usecase::commands::{
    AuthClaimsView, LoginWithDomainUserInfo, LoginWithPassword, Logout, RefreshAccessToken,
    TokenPairView, VerifyAccessToken,
};
use crate::usecase::{AuthUsecase, AuthUsecaseConfig, UsecaseError};

fn fixed_now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 7, 29, 0, 0, 0).unwrap()
}

#[derive(Default)]
struct MockCredState {
    by_code: HashMap<String, UserCredentials>,
    find_calls: usize,
    bump_calls: usize,
}

#[derive(Clone, Default)]
pub struct MockUserCredentialsRepo {
    state: Arc<Mutex<MockCredState>>,
}

impl MockUserCredentialsRepo {
    pub fn seed_hash(&self, code: &str, password_hash: &str, token_version: u32) {
        let now = fixed_now();
        self.state.lock().unwrap().by_code.insert(
            code.to_string(),
            UserCredentials::for_repository(
                code.to_string(),
                password_hash.to_string(),
                token_version,
                now,
                now,
            ),
        );
    }
}

#[async_trait]
impl UserCredentialsRepository for MockUserCredentialsRepo {
    async fn find_by_code(&self, code: &str) -> Result<UserCredentials, DomainError> {
        let mut s = self.state.lock().unwrap();
        s.find_calls += 1;
        s.by_code.get(code).cloned().ok_or(DomainError::NotFound)
    }

    async fn create(&self, credentials: UserCredentials) -> Result<UserCredentials, DomainError> {
        let mut s = self.state.lock().unwrap();
        if s.by_code.contains_key(&credentials.code) {
            return Err(DomainError::DuplicateCode(credentials.code));
        }
        s.by_code
            .insert(credentials.code.clone(), credentials.clone());
        Ok(credentials)
    }

    async fn bump_token_version(&self, code: &str) -> Result<u32, DomainError> {
        let mut s = self.state.lock().unwrap();
        s.bump_calls += 1;
        let entry = s.by_code.get_mut(code).ok_or(DomainError::NotFound)?;
        entry.token_version += 1;
        Ok(entry.token_version)
    }

    async fn update_password_hash(
        &self,
        code: &str,
        password_hash: &str,
    ) -> Result<UserCredentials, DomainError> {
        let mut s = self.state.lock().unwrap();
        let entry = s.by_code.get_mut(code).ok_or(DomainError::NotFound)?;
        entry.password_hash = password_hash.to_string();
        Ok(entry.clone())
    }

    async fn delete_by_code(&self, code: &str) -> Result<(), DomainError> {
        let mut s = self.state.lock().unwrap();
        if s.by_code.remove(code).is_none() {
            return Err(DomainError::NotFound);
        }
        Ok(())
    }
}

#[derive(Default)]
struct MockIdentityState {
    rows: Vec<DomainIdentity>,
    find_calls: usize,
}

#[derive(Clone, Default)]
pub struct MockDomainIdentityRepo {
    state: Arc<Mutex<MockIdentityState>>,
}

impl MockDomainIdentityRepo {
    pub fn seed(&self, id: DomainIdentity) {
        self.state.lock().unwrap().rows.push(id);
    }
}

#[async_trait]
impl DomainIdentityRepository for MockDomainIdentityRepo {
    async fn find(
        &self,
        user_code: &str,
        domain_name: &str,
        hostname: &str,
        sid: &str,
    ) -> Result<DomainIdentity, DomainError> {
        let mut s = self.state.lock().unwrap();
        s.find_calls += 1;
        s.rows
            .iter()
            .find(|r| {
                r.user_code == user_code
                    && r.domain_name == domain_name
                    && r.hostname == hostname
                    && r.sid == sid
            })
            .cloned()
            .ok_or(DomainError::NotFound)
    }
}

#[derive(Clone, Default)]
pub struct FakeUserService {
    by_code: Arc<Mutex<HashMap<String, UserSummary>>>,
}

impl FakeUserService {
    pub fn seed(&self, code: &str, role: Role, active: bool) {
        let summary = UserSummary {
            code: code.to_string(),
            active,
            role,
        };
        self.by_code.lock().unwrap().insert(code.to_string(), summary);
    }
}

#[async_trait]
impl UserService for FakeUserService {
    async fn get_by_code(&self, code: &str) -> Result<UserSummary, DomainError> {
        self.by_code
            .lock()
            .unwrap()
            .get(code)
            .cloned()
            .ok_or(DomainError::NotFound)
    }
}

/// Build a usecase wired to the mocks + a freshly-derived HMAC key
/// + an in-memory token-version cache.
pub fn make_usecase(
    creds: MockUserCredentialsRepo,
    ids: MockDomainIdentityRepo,
    users: FakeUserService,
) -> AuthUsecase<MockUserCredentialsRepo, MockDomainIdentityRepo> {
    let cfg = AuthUsecaseConfig {
        credentials: creds,
        identities: ids,
        user_service: Arc::new(users),
        cache: Arc::new(crate::InMemoryTokenVersionCache::new()),
        signing_key: b"0123456789abcdef0123456789abcdef".to_vec(),
        access_ttl: std::time::Duration::from_secs(60),
        refresh_ttl: std::time::Duration::from_secs(3600),
    };
    AuthUsecase::new(cfg)
}

/// Hash a password the same way the usecase does (argon2 default).
pub fn hash_password(plain: &str) -> String {
    use argon2::password_hash::{PasswordHasher, SaltString, rand_core::OsRng};
    let salt = SaltString::generate(&mut OsRng);
    argon2::Argon2::default()
        .hash_password(plain.as_bytes(), &salt)
        .expect("hash")
        .to_string()
}

// Suppress unused warnings for types that exist only for the later
// test set.
#[allow(dead_code)]
fn _ensure_exports_in_scope(_: &AuthClaimsView, _: &TokenPairView, _: &Role) {}

fn make_seeded_usecase_for_password_login(
    plain_password: &str,
    initial_token_version: u32,
) -> (
    MockUserCredentialsRepo,
    MockDomainIdentityRepo,
    FakeUserService,
    AuthUsecase<MockUserCredentialsRepo, MockDomainIdentityRepo>,
) {
    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", &hash_password(plain_password), initial_token_version);
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    users.seed("u1", Role::Admin, true);
    let usecase = make_usecase(creds.clone(), ids.clone(), users.clone());
    (creds, ids, users, usecase)
}

#[tokio::test]
async fn login_with_password_mints_token_pair_for_valid_credentials() {
    let (_creds, _ids, _users, usecase) = make_seeded_usecase_for_password_login("hunter2", 1);
    let pair = usecase
        .login_with_password(LoginWithPassword {
            code: "u1".into(),
            password: "hunter2".into(),
        })
        .await
        .expect("login succeeds");
    assert!(!pair.access_token.is_empty());
    assert!(!pair.refresh_token.is_empty());
}

#[tokio::test]
async fn login_with_password_rejects_empty_code_with_validation() {
    let (_creds, _ids, _users, usecase) = make_seeded_usecase_for_password_login("hunter2", 1);
    let err = usecase
        .login_with_password(LoginWithPassword {
            code: "  ".into(),
            password: "hunter2".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::EmptyCode)
    ));
}

#[tokio::test]
async fn login_with_password_rejects_empty_password() {
    let (_creds, _ids, _users, usecase) = make_seeded_usecase_for_password_login("hunter2", 1);
    let err = usecase
        .login_with_password(LoginWithPassword {
            code: "u1".into(),
            password: "".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::EmptyPasswordHash)
    ));
}

#[tokio::test]
async fn login_with_password_rejects_inactive_user() {
    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", &hash_password("hunter2"), 1);
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    users.seed("u1", Role::Admin, false);
    let usecase = make_usecase(creds, ids, users);

    let err = usecase
        .login_with_password(LoginWithPassword {
            code: "u1".into(),
            password: "hunter2".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::Inactive)
    ));
}

#[tokio::test]
async fn login_with_password_rejects_wrong_password() {
    let (_creds, _ids, _users, usecase) = make_seeded_usecase_for_password_login("hunter2", 1);
    let err = usecase
        .login_with_password(LoginWithPassword {
            code: "u1".into(),
            password: "WRONG".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::InvalidCredentials)
    ));
}

#[tokio::test]
async fn login_with_password_rejects_unknown_user() {
    let creds = MockUserCredentialsRepo::default();
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    let usecase = make_usecase(creds, ids, users);

    let err = usecase
        .login_with_password(LoginWithPassword {
            code: "ghost".into(),
            password: "hunter2".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::NotFound)
    ));
}

#[tokio::test]
async fn login_with_domain_user_info_mints_token_pair_for_matching_identity() {
    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", &hash_password("hunter2"), 5);
    let ids = MockDomainIdentityRepo::default();
    ids.seed(DomainIdentity::for_repository(
        "u1".into(),
        "DOM".into(),
        "host".into(),
        "S-1-5".into(),
    ));
    let users = FakeUserService::default();
    users.seed("u1", Role::Admin, true);
    let usecase = make_usecase(creds, ids, users);

    let pair = usecase
        .login_with_domain_user_info(LoginWithDomainUserInfo {
            code: "u1".into(),
            domain_name: "DOM".into(),
            hostname: "host".into(),
            sid: "S-1-5".into(),
        })
        .await
        .expect("login succeeds");
    assert!(!pair.access_token.is_empty());
    assert!(!pair.refresh_token.is_empty());
}

#[tokio::test]
async fn login_with_domain_user_info_returns_not_found_for_unmatched_triple() {
    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", &hash_password("hunter2"), 1);
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    users.seed("u1", Role::Admin, true);
    let usecase = make_usecase(creds, ids, users);

    let err = usecase
        .login_with_domain_user_info(LoginWithDomainUserInfo {
            code: "u1".into(),
            domain_name: "DOM".into(),
            hostname: "host".into(),
            sid: "S-1-5".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::NotFound)
    ));
}

#[tokio::test]
async fn login_with_domain_user_info_rejects_inactive_user() {
    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", &hash_password("hunter2"), 1);
    let ids = MockDomainIdentityRepo::default();
    ids.seed(DomainIdentity::for_repository(
        "u1".into(),
        "DOM".into(),
        "host".into(),
        "S-1-5".into(),
    ));
    let users = FakeUserService::default();
    users.seed("u1", Role::Admin, false);
    let usecase = make_usecase(creds, ids, users);

    let err = usecase
        .login_with_domain_user_info(LoginWithDomainUserInfo {
            code: "u1".into(),
            domain_name: "DOM".into(),
            hostname: "host".into(),
            sid: "S-1-5".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::Inactive)
    ));
}

#[tokio::test]
async fn verify_returns_claims_for_freshly_minted_access_token() {
    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", &hash_password("hunter2"), 7);
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    users.seed("u1", Role::Admin, true);
    let usecase = make_usecase(creds, ids, users);

    let pair = usecase
        .login_with_password(LoginWithPassword {
            code: "u1".into(),
            password: "hunter2".into(),
        })
        .await
        .expect("login succeeds");
    let claims = usecase
        .verify(VerifyAccessToken {
            access_token: pair.access_token,
        })
        .await
        .expect("verify succeeds");
    assert_eq!(claims.code, "u1");
    assert_eq!(claims.role, Role::Admin);
    assert_eq!(claims.token_version, 7);
}

#[tokio::test]
async fn verify_hits_cache_after_login_without_calling_find_by_code() {
    // login_with_password warms the cache, so the first verify must
    // succeed without touching `find_by_code`.
    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", &hash_password("hunter2"), 3);
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    users.seed("u1", Role::Admin, true);
    let usecase = make_usecase(creds.clone(), ids, users);

    let pair = usecase
        .login_with_password(LoginWithPassword {
            code: "u1".into(),
            password: "hunter2".into(),
        })
        .await
        .expect("login succeeds");

    let find_calls_before = {
        let s = creds.state.lock().unwrap();
        s.find_calls
    };

    usecase
        .verify(VerifyAccessToken {
            access_token: pair.access_token.clone(),
        })
        .await
        .expect("verify succeeds");

    let s = creds.state.lock().unwrap();
    assert_eq!(
        s.find_calls, find_calls_before,
        "verify must hit the cache, not find_by_code"
    );
}

#[tokio::test]
async fn verify_falls_back_to_repo_on_cache_miss_and_populates_cache() {
    // Fresh usecase with an empty cache: the first verify hits the
    // repo for the version; the second hits the cache.
    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", &hash_password("hunter2"), 3);
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    users.seed("u1", Role::Admin, true);

    // Build a usecase, mint a token by logging in through it, then
    // build a SECOND usecase that shares the same repo but starts
    // with an empty cache (simulating a cold restart in the same
    // process). The first verify through the cold usecase must hit
    // the repo; the second must hit the cache.
    let warm_usecase = make_usecase(creds.clone(), ids.clone(), users.clone());
    let pair = warm_usecase
        .login_with_password(LoginWithPassword {
            code: "u1".into(),
            password: "hunter2".into(),
        })
        .await
        .expect("login succeeds");

    let cold_usecase = make_usecase(creds.clone(), ids, users);

    let find_calls_before = {
        let s = creds.state.lock().unwrap();
        s.find_calls
    };

    cold_usecase
        .verify(VerifyAccessToken {
            access_token: pair.access_token.clone(),
        })
        .await
        .expect("verify succeeds after cold restart");

    let find_calls_after_first = {
        let s = creds.state.lock().unwrap();
        s.find_calls
    };
    assert!(
        find_calls_after_first > find_calls_before,
        "first verify after cold restart must hit the repo"
    );

    cold_usecase
        .verify(VerifyAccessToken {
            access_token: pair.access_token,
        })
        .await
        .expect("verify succeeds again");

    let find_calls_after_second = {
        let s = creds.state.lock().unwrap();
        s.find_calls
    };
    assert_eq!(
        find_calls_after_second, find_calls_after_first,
        "second verify must hit the cache"
    );
}

#[tokio::test]
async fn logout_invalidates_cached_token_version() {
    // After logout, the cache holds the bumped version. A token
    // minted before the logout must fail verify because the
    // cached version > jwt.ver.
    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", &hash_password("hunter2"), 1);
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    users.seed("u1", Role::Admin, true);
    let usecase = make_usecase(creds, ids, users);

    let pair = usecase
        .login_with_password(LoginWithPassword {
            code: "u1".into(),
            password: "hunter2".into(),
        })
        .await
        .expect("login succeeds");

    usecase
        .verify(VerifyAccessToken {
            access_token: pair.access_token.clone(),
        })
        .await
        .expect("pre-logout verify passes");

    usecase
        .logout(crate::usecase::Logout {
            refresh_token: pair.refresh_token,
        })
        .await
        .expect("logout succeeds");

    let err = usecase
        .verify(VerifyAccessToken {
            access_token: pair.access_token,
        })
        .await
        .unwrap_err();
    assert!(
        matches!(err, UsecaseError::Verification(_)),
        "post-logout verify must reject; got {err:?}"
    );
}

#[tokio::test]
async fn verify_rejects_refresh_token_presented_as_access_token() {
    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", &hash_password("hunter2"), 1);
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    users.seed("u1", Role::Admin, true);
    let usecase = make_usecase(creds, ids, users);

    let pair = usecase
        .login_with_password(LoginWithPassword {
            code: "u1".into(),
            password: "hunter2".into(),
        })
        .await
        .expect("login succeeds");
    let err = usecase
        .verify(VerifyAccessToken {
            access_token: pair.refresh_token,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, UsecaseError::Verification(_)));
}

#[tokio::test]
async fn verify_rejects_inactive_user() {
    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", &hash_password("hunter2"), 1);
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    users.seed("u1", Role::Admin, true);
    let usecase = make_usecase(creds, ids, users.clone());

    let pair = usecase
        .login_with_password(LoginWithPassword {
            code: "u1".into(),
            password: "hunter2".into(),
        })
        .await
        .expect("login succeeds");

    // Flip the user to inactive and verify fails.
    users.seed("u1", Role::Admin, false);
    let err = usecase
        .verify(VerifyAccessToken {
            access_token: pair.access_token,
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::Inactive)
    ));
}

#[tokio::test]
async fn refresh_mints_new_access_token_with_current_version() {
    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", &hash_password("hunter2"), 4);
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    users.seed("u1", Role::Admin, true);
    let usecase = make_usecase(creds, ids, users);

    let pair = usecase
        .login_with_password(LoginWithPassword {
            code: "u1".into(),
            password: "hunter2".into(),
        })
        .await
        .expect("login succeeds");
    let new = usecase
        .refresh(RefreshAccessToken {
            refresh_token: pair.refresh_token,
        })
        .await
        .expect("refresh succeeds");
    assert!(!new.access_token.is_empty());

    // The new access token must verify.
    let claims = usecase
        .verify(VerifyAccessToken {
            access_token: new.access_token,
        })
        .await
        .expect("new access token verifies");
    assert_eq!(claims.code, "u1");
    assert_eq!(claims.token_version, 4);
}

#[tokio::test]
async fn refresh_rejects_access_token_presented_as_refresh_token() {
    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", &hash_password("hunter2"), 1);
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    users.seed("u1", Role::Admin, true);
    let usecase = make_usecase(creds, ids, users);

    let pair = usecase
        .login_with_password(LoginWithPassword {
            code: "u1".into(),
            password: "hunter2".into(),
        })
        .await
        .expect("login succeeds");
    let err = usecase
        .refresh(RefreshAccessToken {
            refresh_token: pair.access_token,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, UsecaseError::Verification(_)));
}

#[tokio::test]
async fn logout_bumps_token_version_and_invalidates_outstanding_tokens() {
    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", &hash_password("hunter2"), 1);
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    users.seed("u1", Role::Admin, true);
    let usecase = make_usecase(creds, ids, users);

    let pair = usecase
        .login_with_password(LoginWithPassword {
            code: "u1".into(),
            password: "hunter2".into(),
        })
        .await
        .expect("login succeeds");

    // Pre-logout verify passes.
    usecase
        .verify(VerifyAccessToken {
            access_token: pair.access_token.clone(),
        })
        .await
        .expect("pre-logout verify passes");

    let ack = usecase
        .logout(Logout {
            refresh_token: pair.refresh_token.clone(),
        })
        .await
        .expect("logout succeeds");
    assert_eq!(ack, crate::usecase::LogoutAck {});

    // Idempotent: a second logout with the same (now stale) refresh
    // token still succeeds.
    usecase
        .logout(Logout {
            refresh_token: pair.refresh_token,
        })
        .await
        .expect("second logout succeeds (idempotent)");

    // Post-logout verify rejects.
    let err = usecase
        .verify(VerifyAccessToken {
            access_token: pair.access_token,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, UsecaseError::Verification(_)));
}

#[tokio::test]
async fn logout_with_empty_refresh_token_is_verification_error() {
    let creds = MockUserCredentialsRepo::default();
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    let usecase = make_usecase(creds, ids, users);

    let err = usecase
        .logout(Logout {
            refresh_token: "  ".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, UsecaseError::Verification(_)));
}

#[tokio::test]
async fn logout_with_garbage_refresh_token_is_verification_error() {
    let creds = MockUserCredentialsRepo::default();
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    let usecase = make_usecase(creds, ids, users);

    let err = usecase
        .logout(Logout {
            refresh_token: "not.a.real.jwt".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, UsecaseError::Verification(_)));
}

// -- credential management --------------------------------------------

#[tokio::test]
async fn find_user_credential_returns_view_for_known_code() {
    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", "hash", 3);
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    let usecase = make_usecase(creds, ids, users);

    let view = usecase
        .find_user_credential(crate::usecase::FindUserCredential { code: "u1".into() })
        .await
        .expect("find succeeds");
    assert_eq!(view.code, "u1");
    assert_eq!(view.password_hash, "hash");
    assert_eq!(view.token_version, 3);
}

#[tokio::test]
async fn find_user_credential_returns_not_found_for_unknown_code() {
    let creds = MockUserCredentialsRepo::default();
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    let usecase = make_usecase(creds, ids, users);

    let err = usecase
        .find_user_credential(crate::usecase::FindUserCredential {
            code: "ghost".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::NotFound)
    ));
}

#[tokio::test]
async fn create_user_credential_hashes_raw_password_before_persisting() {
    let creds = MockUserCredentialsRepo::default();
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    let usecase = make_usecase(creds, ids, users);

    let view = usecase
        .create_user_credential(crate::usecase::CreateUserCredential {
            code: "u1".into(),
            password: "hunter2".into(),
        })
        .await
        .expect("create succeeds");
    assert_eq!(view.code, "u1");
    assert_eq!(view.token_version, 0);
    assert_ne!(
        view.password_hash, "hunter2",
        "raw password must not round-trip into the view"
    );
    let parsed = argon2::PasswordHash::new(&view.password_hash).expect("valid phc string");
    argon2::Argon2::default()
        .verify_password(b"hunter2", &parsed)
        .expect("stored hash must verify against the original password");
}

#[tokio::test]
async fn create_user_credential_rejects_empty_code() {
    let creds = MockUserCredentialsRepo::default();
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    let usecase = make_usecase(creds, ids, users);

    let err = usecase
        .create_user_credential(crate::usecase::CreateUserCredential {
            code: "  ".into(),
            password: "hunter2".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::EmptyCode)
    ));
}

#[tokio::test]
async fn create_user_credential_rejects_empty_password() {
    let creds = MockUserCredentialsRepo::default();
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    let usecase = make_usecase(creds, ids, users);

    let err = usecase
        .create_user_credential(crate::usecase::CreateUserCredential {
            code: "u1".into(),
            password: "".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::EmptyPasswordHash)
    ));
}

#[tokio::test]
async fn update_user_credential_hashes_raw_password_when_some() {
    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", "old", 1);
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    let usecase = make_usecase(creds, ids, users);

    let view = usecase
        .update_user_credential(crate::usecase::UpdateUserCredential {
            code: "u1".into(),
            password: Some("new".into()),
        })
        .await
        .expect("update succeeds");
    assert_ne!(
        view.password_hash, "new",
        "raw password must not round-trip into the view"
    );
    let parsed = argon2::PasswordHash::new(&view.password_hash).expect("valid phc string");
    argon2::Argon2::default()
        .verify_password(b"new", &parsed)
        .expect("stored hash must verify against the original password");
    assert_eq!(view.token_version, 1, "update must not bump token_version");
}

#[tokio::test]
async fn update_user_credential_returns_unchanged_view_when_no_fields_set() {
    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", "hash", 5);
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    let usecase = make_usecase(creds, ids, users);

    let view = usecase
        .update_user_credential(crate::usecase::UpdateUserCredential {
            code: "u1".into(),
            ..Default::default()
        })
        .await
        .expect("update succeeds");
    assert_eq!(view.password_hash, "hash");
    assert_eq!(view.token_version, 5);
}

#[tokio::test]
async fn update_user_credential_returns_not_found_for_unknown_code() {
    let creds = MockUserCredentialsRepo::default();
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    let usecase = make_usecase(creds, ids, users);

    let err = usecase
        .update_user_credential(crate::usecase::UpdateUserCredential {
            code: "ghost".into(),
            ..Default::default()
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::NotFound)
    ));
}

#[tokio::test]
async fn remove_user_credential_deletes_the_row() {
    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", "hash", 1);
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    let usecase = make_usecase(creds, ids, users);

    usecase
        .remove_user_credential(crate::usecase::RemoveUserCredential { code: "u1".into() })
        .await
        .expect("remove succeeds");

    // Subsequent find returns NotFound.
    let err = usecase
        .find_user_credential(crate::usecase::FindUserCredential { code: "u1".into() })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::NotFound)
    ));
}

#[tokio::test]
async fn remove_user_credential_returns_not_found_for_unknown_code() {
    let creds = MockUserCredentialsRepo::default();
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    let usecase = make_usecase(creds, ids, users);

    let err = usecase
        .remove_user_credential(crate::usecase::RemoveUserCredential {
            code: "ghost".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::NotFound)
    ));
}
