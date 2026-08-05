//! Unit tests for `AuthUsecase`.
//!
//! Mock repos and a `FakeUserService` (mirroring the apis `UserService`
//! surface) stand in for the real adapters so the usecase can be
//! exercised without PostgreSQL.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};

use apis::user::{
    CreateUserRequest, Role as ApiRole, UpdateUserRequest, UserApiError, UserService,
    UserView,
};

use crate::domain::{
    DomainError, DomainIdentity, DomainIdentityRepository, Role, UserCredentials,
    UserCredentialsRepository,
};
use crate::usecase::commands::{
    AuthClaimsView, Logout, LoginWithDomainUserInfo, LoginWithPassword, RefreshAccessToken,
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

    async fn create(
        &self,
        credentials: UserCredentials,
    ) -> Result<UserCredentials, DomainError> {
        let mut s = self.state.lock().unwrap();
        if s.by_code.contains_key(&credentials.code) {
            return Err(DomainError::DuplicateCode(credentials.code));
        }
        s.by_code.insert(credentials.code.clone(), credentials.clone());
        Ok(credentials)
    }

    async fn bump_token_version(&self, code: &str) -> Result<u32, DomainError> {
        let mut s = self.state.lock().unwrap();
        s.bump_calls += 1;
        let entry = s.by_code.get_mut(code).ok_or(DomainError::NotFound)?;
        entry.token_version += 1;
        Ok(entry.token_version)
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
    by_code: Arc<Mutex<HashMap<String, UserView>>>,
}

impl FakeUserService {
    pub fn seed(&self, code: &str, role: ApiRole, active: bool) {
        let now = fixed_now();
        let view = UserView {
            id: 1,
            code: code.to_string(),
            name: code.to_string(),
            role,
            active,
            created_at: now,
            updated_at: now,
        };
        self.by_code.lock().unwrap().insert(code.to_string(), view);
    }
}

#[async_trait]
impl UserService for FakeUserService {
    async fn create(&self, _req: CreateUserRequest) -> Result<UserView, UserApiError> {
        unimplemented!()
    }
    async fn get_by_id(&self, _id: i32) -> Result<UserView, UserApiError> {
        unimplemented!()
    }
    async fn get_by_code(&self, code: &str) -> Result<UserView, UserApiError> {
        self.by_code
            .lock()
            .unwrap()
            .get(code)
            .cloned()
            .ok_or(UserApiError::NotFound)
    }
    async fn list(&self) -> Result<Vec<UserView>, UserApiError> {
        unimplemented!()
    }
    async fn update(&self, _req: UpdateUserRequest) -> Result<UserView, UserApiError> {
        unimplemented!()
    }
}

/// Build a usecase wired to the mocks + a freshly-derived HMAC key.
pub fn make_usecase(
    creds: MockUserCredentialsRepo,
    ids: MockDomainIdentityRepo,
    users: FakeUserService,
) -> AuthUsecase<MockUserCredentialsRepo, MockDomainIdentityRepo> {
    let cfg = AuthUsecaseConfig {
        credentials: creds,
        identities: ids,
        user_service: Arc::new(users),
        signing_key: b"0123456789abcdef0123456789abcdef".to_vec(),
        access_ttl: std::time::Duration::from_secs(60),
        refresh_ttl: std::time::Duration::from_secs(3600),
    };
    AuthUsecase::new(cfg)
}

/// Hash a password the same way the usecase does (argon2 default).
pub fn hash_password(plain: &str) -> String {
    use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
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
    users.seed("u1", ApiRole::Admin, true);
    let usecase = make_usecase(creds.clone(), ids.clone(), users.clone());
    (creds, ids, users, usecase)
}

#[tokio::test]
async fn login_with_password_mints_token_pair_for_valid_credentials() {
    let (_creds, _ids, _users, usecase) =
        make_seeded_usecase_for_password_login("hunter2", 1);
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
    let (_creds, _ids, _users, usecase) =
        make_seeded_usecase_for_password_login("hunter2", 1);
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
    let (_creds, _ids, _users, usecase) =
        make_seeded_usecase_for_password_login("hunter2", 1);
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
    users.seed("u1", ApiRole::Admin, false);
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
    let (_creds, _ids, _users, usecase) =
        make_seeded_usecase_for_password_login("hunter2", 1);
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
    assert!(matches!(err, UsecaseError::Repository(DomainError::NotFound)));
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
    users.seed("u1", ApiRole::Admin, true);
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
    users.seed("u1", ApiRole::Admin, true);
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
    assert!(matches!(err, UsecaseError::Repository(DomainError::NotFound)));
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
    users.seed("u1", ApiRole::Admin, false);
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