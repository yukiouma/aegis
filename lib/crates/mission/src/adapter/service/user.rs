use std::sync::Arc;

use async_trait::async_trait;

use apis::user::{UserApiError, UserService};

use crate::domain::{DomainError, UserLookup};

/// Adapter that maps the apis `UserService` port onto the narrow
/// domain `UserLookup` port.
pub struct UserLookupImpl {
    users: Arc<dyn UserService>,
}

impl UserLookupImpl {
    pub fn new(users: Arc<dyn UserService>) -> Self {
        Self { users }
    }
}

#[async_trait]
impl UserLookup for UserLookupImpl {
    async fn get_by_code(&self, code: &str) -> Result<(), DomainError> {
        match self.users.get_by_code(code).await {
            Ok(_) => Ok(()),
            Err(UserApiError::NotFound) => Err(DomainError::UserNotFound(code.to_string())),
            Err(e) => Err(DomainError::Repository(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests;