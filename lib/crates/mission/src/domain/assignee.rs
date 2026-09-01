use chrono::{DateTime, Utc};

use super::error::DomainError;
use super::mission_role::MissionRole;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assignee {
    pub id: i64,
    pub user_code: String,
    pub role: MissionRole,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Assignee {
    /// Validating constructor used by tests and any in-crate path
    /// that builds from raw inputs.
    pub fn new(
        id: i64,
        user_code: String,
        role: MissionRole,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if user_code.trim().is_empty() {
            return Err(DomainError::EmptyUserCode);
        }
        Ok(Self {
            id,
            user_code,
            role,
            created_at,
            updated_at,
        })
    }

    /// Bypasses validation. Reserved for the adapter row bridge.
    #[allow(dead_code)]
    pub(crate) fn for_repository(
        id: i64,
        user_code: String,
        role: MissionRole,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            user_code,
            role,
            created_at,
            updated_at,
        }
    }
}
