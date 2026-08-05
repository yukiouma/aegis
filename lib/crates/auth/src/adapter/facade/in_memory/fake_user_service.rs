//! Test-only fake `apis::user::UserService` used by the facade tests.

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{TimeZone, Utc};

use apis::user::{
    CreateUserRequest, Role as ApiRole, UpdateUserRequest, UserApiError, UserService,
    UserView,
};

pub struct FakeUserService {
    by_code: Mutex<HashMap<String, UserView>>,
}

impl FakeUserService {
    pub fn new() -> Self {
        Self {
            by_code: Mutex::new(HashMap::new()),
        }
    }

    pub fn seed(&self, code: &str, role: ApiRole, active: bool) {
        let now = Utc.with_ymd_and_hms(2026, 7, 29, 0, 0, 0).unwrap();
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

impl Default for FakeUserService {
    fn default() -> Self {
        Self::new()
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