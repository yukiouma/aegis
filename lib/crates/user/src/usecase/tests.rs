//! Tests for the usecase layer.
//!
//! A `MockUserRepository` stands in for the real SQLx-backed repository so
//! we can assert that the usecase hashes passwords, validates inputs, and
//! never leaks the password hash out of the persistence boundary.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::domain::{DomainError, Role, User, UserNew, UserRepository, UserUpdate};
use crate::usecase::commands::{CreateUser, UpdateUser};
use crate::usecase::error::UsecaseError;
use crate::usecase::user_usecase::{UserUsecase, UserView};

/// Captured state for the mock repository. Held behind a `Mutex` so the
/// usecase (async) and the test (sync) can both observe it.
#[derive(Default)]
struct MockState {
    /// Users "stored" by the fake repository, keyed by id.
    users: HashMap<i32, User>,
    /// Every `create` input the usecase handed to the repo.
    created: Vec<UserNew>,
    /// Every `update` input the usecase handed to the repo.
    updated: Vec<UserUpdate>,
    /// Every id the usecase passed to `deactivate`.
    deactivated: Vec<i32>,
    /// Ids passed to `find_by_id`.
    find_by_id_calls: Vec<i32>,
    /// Codes passed to `find_by_code`.
    find_by_code_calls: Vec<String>,
    /// Number of `list` calls.
    list_calls: usize,
    /// Next id the mock will hand out from `create`. Seeded to 1 by
    /// [`MockUserRepository::new`] so freshly-created rows have an id
    /// distinct from any pre-existing ones (which start at 1 too).
    next_id: i32,
}

#[derive(Clone, Default)]
struct MockUserRepository {
    state: Arc<Mutex<MockState>>,
}

impl MockUserRepository {
    fn new() -> Self {
        let mock = Self::default();
        mock.state.lock().unwrap().next_id = 1;
        mock
    }

    /// Seed the mock with a single pre-existing user.
    fn with_user(user: User) -> Self {
        let mock = Self::new();
        {
            let mut state = mock.state.lock().unwrap();
            state.users.insert(user.id, user.clone());
            state.next_id = user.id + 1;
        }
        mock
    }

    /// Seed the mock with several pre-existing users.
    fn with_users(users: Vec<User>) -> Self {
        let mock = Self::new();
        {
            let mut state = mock.state.lock().unwrap();
            let mut max_id = 0;
            for user in users {
                max_id = max_id.max(user.id);
                state.users.insert(user.id, user);
            }
            state.next_id = max_id + 1;
        }
        mock
    }
}

#[async_trait]
impl UserRepository for MockUserRepository {
    async fn create(&self, input: UserNew) -> Result<User, DomainError> {
        let mut state = self.state.lock().unwrap();
        if state
            .users
            .values()
            .any(|existing| existing.code == input.code)
        {
            return Err(DomainError::DuplicateCode(input.code.clone()));
        }
        let id = state.next_id;
        state.next_id += 1;
        let stored = User::for_repository(
            id,
            input.code.clone(),
            input.name.clone(),
            input.role,
            true,
            input.password_hash.clone(),
        );
        state.users.insert(id, stored.clone());
        state.created.push(input);
        Ok(stored)
    }

    async fn find_by_id(&self, id: i32) -> Result<User, DomainError> {
        let mut state = self.state.lock().unwrap();
        state.find_by_id_calls.push(id);
        state
            .users
            .get(&id)
            .cloned()
            .ok_or(DomainError::NotFound)
    }

    async fn find_by_code(&self, code: &str) -> Result<User, DomainError> {
        let mut state = self.state.lock().unwrap();
        state.find_by_code_calls.push(code.to_string());
        state
            .users
            .values()
            .find(|existing| existing.code == code)
            .cloned()
            .ok_or(DomainError::NotFound)
    }

    async fn list(&self) -> Result<Vec<User>, DomainError> {
        let mut state = self.state.lock().unwrap();
        state.list_calls += 1;
        Ok(state.users.values().cloned().collect())
    }

    async fn update(&self, input: UserUpdate) -> Result<User, DomainError> {
        let mut state = self.state.lock().unwrap();
        let stored = state
            .users
            .get_mut(&input.id)
            .ok_or(DomainError::NotFound)?;
        if let Some(code) = input.code.clone() {
            stored.code = code;
        }
        if let Some(name) = input.name.clone() {
            stored.name = name;
        }
        if let Some(role) = input.role {
            stored.role = role;
        }
        if let Some(active) = input.active {
            stored.active = active;
        }
        if let Some(hash) = input.password_hash.clone() {
            stored.password = hash;
        }
        let updated = stored.clone();
        state.updated.push(input);
        Ok(updated)
    }

    async fn deactivate(&self, id: i32) -> Result<User, DomainError> {
        let mut state = self.state.lock().unwrap();
        state.deactivated.push(id);
        let stored = state.users.get_mut(&id).ok_or(DomainError::NotFound)?;
        stored.active = false;
        Ok(stored.clone())
    }
}

fn make_usecase() -> (MockUserRepository, UserUsecase<MockUserRepository>) {
    let mock = MockUserRepository::new();
    let usecase = UserUsecase::new(mock.clone());
    (mock, usecase)
}

fn make_usecase_with_user(user: User) -> (MockUserRepository, UserUsecase<MockUserRepository>) {
    let mock = MockUserRepository::with_user(user);
    let usecase = UserUsecase::new(mock.clone());
    (mock, usecase)
}

fn seed_user(id: i32, code: &str, name: &str, role: Role, active: bool, hash: &str) -> User {
    User::for_repository(id, code.into(), name.into(), role, active, hash.into())
}

#[tokio::test]
async fn create_hashes_password_before_repository() {
    let (mock, usecase) = make_usecase();

    let cmd = CreateUser {
        code: "u1".into(),
        name: "Alice".into(),
        role: Role::Admin,
        password: "hunter42".into(),
    };

    let view = usecase.create(cmd).await.expect("create succeeds");

    let expected = UserView {
        id: 1,
        code: "u1".into(),
        name: "Alice".into(),
        role: Role::Admin,
        active: true,
    };
    assert_eq!(view, expected);

    let state = mock.state.lock().unwrap();
    assert_eq!(state.created.len(), 1, "create should hit repo once");
    let captured = &state.created[0];
    assert_eq!(captured.code, "u1");
    assert_eq!(captured.name, "Alice");
    assert_eq!(captured.role, Role::Admin);
    assert!(
        captured.password_hash.starts_with("$argon2"),
        "expected argon2 PHC string, got {:?}",
        captured.password_hash
    );
    assert!(
        !captured.password_hash.contains("hunter42"),
        "plaintext password leaked to repository"
    );
}

#[tokio::test]
async fn update_with_password_re_hashes_it() {
    let stored = seed_user(7, "u7", "Bob", Role::General, true, "old-hash");
    let (mock, usecase) = make_usecase_with_user(stored);

    let cmd = UpdateUser {
        id: 7,
        password: Some("new-pass".into()),
        ..blank_update()
    };

    let view = usecase.update(cmd).await.expect("update succeeds");
    assert_eq!(view.id, 7);

    let state = mock.state.lock().unwrap();
    assert_eq!(state.updated.len(), 1);
    let captured = &state.updated[0];
    let new_hash = captured
        .password_hash
        .as_ref()
        .expect("repo receives a hash when password is updated");
    assert!(new_hash.starts_with("$argon2"));
    assert!(!new_hash.contains("new-pass"));
    assert_ne!(new_hash, "old-hash");
}

#[tokio::test]
async fn update_without_password_leaves_hash_unchanged() {
    let stored = seed_user(9, "u9", "Carol", Role::Root, true, "existing-hash");
    let (mock, usecase) = make_usecase_with_user(stored);

    let cmd = UpdateUser {
        id: 9,
        name: Some("Carol Updated".into()),
        ..blank_update()
    };

    usecase.update(cmd).await.expect("update succeeds");

    let state = mock.state.lock().unwrap();
    assert_eq!(state.updated.len(), 1);
    let captured = &state.updated[0];
    assert!(
        captured.password_hash.is_none(),
        "repo should not see a password hash when caller did not supply one"
    );
}

#[tokio::test]
async fn get_by_id_returns_view_without_password() {
    let stored = seed_user(11, "u11", "Dana", Role::Admin, true, "secret-hash");
    let (mock, usecase) = make_usecase_with_user(stored);

    let view = usecase
        .get_by_id(11)
        .await
        .expect("get_by_id succeeds");

    assert_eq!(view.id, 11);
    assert_eq!(view.code, "u11");
    assert_eq!(view.name, "Dana");
    assert_eq!(view.role, Role::Admin);
    assert!(view.active);

    let state = mock.state.lock().unwrap();
    assert_eq!(state.find_by_id_calls, vec![11]);
}

#[tokio::test]
async fn get_by_code_returns_view_without_password() {
    let stored = seed_user(12, "u12", "Eve", Role::General, false, "another-hash");
    let (mock, usecase) = make_usecase_with_user(stored);

    let view = usecase
        .get_by_code("u12")
        .await
        .expect("get_by_code succeeds");

    assert_eq!(view.code, "u12");
    assert!(!view.active);

    let state = mock.state.lock().unwrap();
    assert_eq!(state.find_by_code_calls, vec!["u12".to_string()]);
}

#[tokio::test]
async fn list_returns_views_without_passwords() {
    let users = vec![
        seed_user(1, "u1", "Alice", Role::Admin, true, "hash-1"),
        seed_user(2, "u2", "Bob", Role::General, true, "hash-2"),
        seed_user(3, "u3", "Carol", Role::Root, false, "hash-3"),
    ];
    let (mock, usecase) = make_usecase_with_users(users);

    let views = usecase.list().await.expect("list succeeds");

    assert_eq!(views.len(), 3);
    let codes: Vec<&str> = views.iter().map(|v| v.code.as_str()).collect();
    assert!(codes.contains(&"u1"));
    assert!(codes.contains(&"u2"));
    assert!(codes.contains(&"u3"));

    let state = mock.state.lock().unwrap();
    assert_eq!(state.list_calls, 1);
}

#[tokio::test]
async fn deactivate_forwards_id_and_returns_inactive_view() {
    let stored = seed_user(20, "u20", "Frank", Role::Admin, true, "h");
    let (mock, usecase) = make_usecase_with_user(stored);

    let view = usecase
        .deactivate(20)
        .await
        .expect("deactivate succeeds");

    assert_eq!(view.id, 20);
    assert!(!view.active, "usecase returns the inactive user");

    let state = mock.state.lock().unwrap();
    assert_eq!(state.deactivated, vec![20], "usecase must forward the id");
}

#[tokio::test]
async fn empty_create_inputs_are_rejected_before_hitting_repo() {
    let (mock, usecase) = make_usecase();

    let cmd = CreateUser {
        code: "   ".into(),
        name: "Alice".into(),
        role: Role::Admin,
        password: "hunter42".into(),
    };
    let err = usecase.create(cmd).await.expect_err("blank code rejected");
    assert!(
        matches!(err, UsecaseError::Validation(DomainError::EmptyCode)),
        "expected Validation(EmptyCode), got {err:?}"
    );

    let cmd = CreateUser {
        code: "u1".into(),
        name: "".into(),
        role: Role::Admin,
        password: "hunter42".into(),
    };
    let err = usecase.create(cmd).await.expect_err("blank name rejected");
    assert!(matches!(
        err,
        UsecaseError::Validation(DomainError::EmptyName)
    ));

    let cmd = CreateUser {
        code: "u1".into(),
        name: "Alice".into(),
        role: Role::Admin,
        password: "".into(),
    };
    let err = usecase.create(cmd).await.expect_err("blank password rejected");
    assert!(matches!(
        err,
        UsecaseError::Validation(DomainError::EmptyPassword)
    ));

    let state = mock.state.lock().unwrap();
    assert!(
        state.created.is_empty(),
        "repository must not be touched for invalid input"
    );
}

#[tokio::test]
async fn empty_update_inputs_are_rejected_before_hitting_repo() {
    let stored = seed_user(30, "u30", "Greta", Role::Admin, true, "h");
    let (mock, usecase) = make_usecase_with_user(stored);

    let cmd = UpdateUser {
        id: 30,
        code: Some("".into()),
        ..blank_update()
    };
    let err = usecase.update(cmd).await.expect_err("blank code rejected");
    assert!(matches!(err, UsecaseError::Validation(DomainError::EmptyCode)));

    let cmd = UpdateUser {
        id: 30,
        password: Some("".into()),
        ..blank_update()
    };
    let err = usecase.update(cmd).await.expect_err("blank password rejected");
    assert!(matches!(
        err,
        UsecaseError::Validation(DomainError::EmptyPassword)
    ));

    let state = mock.state.lock().unwrap();
    assert!(
        state.updated.is_empty(),
        "repository must not be touched for invalid input"
    );
}

#[tokio::test]
async fn repository_errors_propagate_as_usecase_repository_error() {
    let (mock, usecase) = make_usecase();
    let cmd = CreateUser {
        code: "u1".into(),
        name: "Alice".into(),
        role: Role::Admin,
        password: "hunter42".into(),
    };
    // Drive the usecase to create the first user.
    usecase.create(cmd).await.expect("first create works");

    // The mock will reject a duplicate code with DomainError::DuplicateCode.
    let cmd = CreateUser {
        code: "u1".into(),
        name: "Alice".into(),
        role: Role::Admin,
        password: "hunter42".into(),
    };
    let err = usecase.create(cmd).await.expect_err("duplicate rejected");
    assert!(
        matches!(err, UsecaseError::Repository(DomainError::DuplicateCode(ref c)) if c == "u1"),
        "expected Repository(DuplicateCode(\"u1\")), got {err:?}"
    );

    // The mock only records successful creates; the duplicate was rejected
    // before it could be persisted, so exactly one row should remain in
    // the captured log.
    let state = mock.state.lock().unwrap();
    assert_eq!(state.created.len(), 1);
}

fn make_usecase_with_users(users: Vec<User>) -> (MockUserRepository, UserUsecase<MockUserRepository>) {
    let mock = MockUserRepository::with_users(users);
    let usecase = UserUsecase::new(mock.clone());
    (mock, usecase)
}

const fn blank_update() -> UpdateUser {
    UpdateUser {
        id: 0,
        code: None,
        name: None,
        role: None,
        active: None,
        password: None,
    }
}