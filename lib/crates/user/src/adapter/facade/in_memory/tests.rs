//! Unit tests for `UserServiceImpl`.
//!
//! Wires the adapter on top of an in-memory `UserRepository` so the
//! behaviour is exercised without touching PostgreSQL.

use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{TimeZone, Utc};

use apis::user::UserService;
use apis::user::Role as ApiRole;

use crate::domain::{DomainError, User, UserNew, UserRepository, UserUpdate};
use crate::usecase::UserUsecase;

use super::UserServiceImpl;

/// Fixed `DateTime<Utc>` returned by the fake repository for every
/// row it creates. Keeps the assertions readable.
fn epoch() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
}

/// In-memory `UserRepository` used by the facade tests.
///
/// `std::sync::Mutex` is sufficient because the async methods never
/// hold the lock across an `.await`. `AtomicI32` for `next_id`
/// avoids mutating the same byte as `users` from different threads.
#[derive(Default)]
struct InMemoryRepo {
    users: Mutex<Vec<User>>,
    next_id: AtomicI32,
}

impl InMemoryRepo {
    fn new() -> Self {
        Self {
            next_id: AtomicI32::new(1),
            ..Self::default()
        }
    }
}

#[async_trait]
impl UserRepository for InMemoryRepo {
    async fn create(&self, input: UserNew) -> Result<User, DomainError> {
        // Reject duplicate codes first so the caller can distinguish
        // collisions from id exhaustion.
        {
            let users = self.users.lock().unwrap();
            if users.iter().any(|u| u.code == input.code) {
                return Err(DomainError::DuplicateCode(input.code));
            }
        }
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let now = epoch();
        let user = User::for_repository(
            id,
            input.code,
            input.name,
            input.role,
            input.active,
            now,
            now,
        );
        self.users.lock().unwrap().push(user.clone());
        Ok(user)
    }

    async fn find_by_id(&self, id: i32) -> Result<User, DomainError> {
        self.users
            .lock()
            .unwrap()
            .iter()
            .find(|u| u.id == id)
            .cloned()
            .ok_or(DomainError::NotFound)
    }

    async fn find_by_code(&self, code: &str) -> Result<User, DomainError> {
        self.users
            .lock()
            .unwrap()
            .iter()
            .find(|u| u.code == code)
            .cloned()
            .ok_or(DomainError::NotFound)
    }

    async fn list(&self) -> Result<Vec<User>, DomainError> {
        Ok(self.users.lock().unwrap().clone())
    }

    async fn update(&self, input: UserUpdate) -> Result<User, DomainError> {
        let mut users = self.users.lock().unwrap();

        // Reject collisions first so the duplicate-code check does
        // not have to share scope with the mutable borrow below.
        if let Some(ref new_code) = input.code {
            let collision = users
                .iter()
                .any(|u| u.code == *new_code && u.id != input.id);
            if collision {
                return Err(DomainError::DuplicateCode(new_code.clone()));
            }
        }

        let user = users
            .iter_mut()
            .find(|u| u.id == input.id)
            .ok_or(DomainError::NotFound)?;
        if let Some(ref new_code) = input.code {
            user.code = new_code.clone();
        }
        if let Some(ref new_name) = input.name {
            user.name = new_name.clone();
        }
        if let Some(new_role) = input.role {
            user.role = new_role;
        }
        if let Some(new_active) = input.active {
            user.active = new_active;
        }
        Ok(user.clone())
    }
}

/// Build a `UserServiceImpl` wired on top of `InMemoryRepo`.
fn service() -> UserServiceImpl<InMemoryRepo> {
    UserServiceImpl::new(UserUsecase::new(InMemoryRepo::new()))
}

/// Smoke test: the adapter can be constructed. Per-method
/// behaviour is covered by the per-method tasks that follow.
#[tokio::test]
async fn user_service_impl_can_be_constructed() {
    let _service = service();
}

#[tokio::test]
async fn create_returns_view_with_assigned_id_and_active_true() {
    let svc = service();
    let view = svc
        .create(apis::user::CreateUserRequest {
            code: "u1".into(),
            name: "Alice".into(),
            role: ApiRole::Admin,
        })
        .await
        .unwrap();
    assert_eq!(view.id, 1);
    assert_eq!(view.code, "u1");
    assert_eq!(view.name, "Alice");
    assert_eq!(view.role, ApiRole::Admin);
    assert!(view.active);
    assert_eq!(view.created_at, epoch());
    assert_eq!(view.updated_at, epoch());
}

#[tokio::test]
async fn create_rejects_empty_code_with_validation() {
    let svc = service();
    let err = svc
        .create(apis::user::CreateUserRequest {
            code: "  ".into(),
            name: "Alice".into(),
            role: ApiRole::General,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, apis::user::UserApiError::Validation(_)));
}

#[tokio::test]
async fn create_rejects_duplicate_code() {
    let svc = service();
    svc.create(apis::user::CreateUserRequest {
        code: "u1".into(),
        name: "Alice".into(),
        role: ApiRole::General,
    })
    .await
    .unwrap();
    let err = svc
        .create(apis::user::CreateUserRequest {
            code: "u1".into(),
            name: "Bob".into(),
            role: ApiRole::General,
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        apis::user::UserApiError::DuplicateCode(ref c) if c == "u1"
    ));
}

#[tokio::test]
async fn get_by_id_returns_seeded_user() {
    let svc = service();
    let created = svc
        .create(apis::user::CreateUserRequest {
            code: "u1".into(),
            name: "Alice".into(),
            role: ApiRole::Admin,
        })
        .await
        .unwrap();
    let fetched = svc.get_by_id(created.id).await.unwrap();
    assert_eq!(fetched, created);
}

#[tokio::test]
async fn get_by_id_returns_not_found_for_unknown_id() {
    let svc = service();
    let err = svc.get_by_id(999).await.unwrap_err();
    assert!(matches!(err, apis::user::UserApiError::NotFound));
}