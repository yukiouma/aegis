use std::collections::HashSet;

use super::error::DomainError;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProjectMember {
    pub leaders: Vec<String>,
    pub workers: Vec<String>,
}

impl ProjectMember {
    /// Validating constructor used by the usecase layer (and by tests).
    ///
    /// Rules:
    /// - `leaders` must not contain duplicate codes; the first duplicate
    ///   is returned as `DuplicateLeader`.
    /// - `workers` must not contain duplicate codes; the first duplicate
    ///   is returned as `DuplicateWorker`.
    /// - The same code may appear in both `leaders` and `workers` of the
    ///   same team — a leader can also do worker work.
    /// - Either list may be empty.
    pub fn new(leaders: Vec<String>, workers: Vec<String>) -> Result<Self, DomainError> {
        let mut seen_leaders = HashSet::with_capacity(leaders.len());
        for code in &leaders {
            if !seen_leaders.insert(code.as_str()) {
                return Err(DomainError::DuplicateLeader(code.clone()));
            }
        }
        let mut seen_workers = HashSet::with_capacity(workers.len());
        for code in &workers {
            if !seen_workers.insert(code.as_str()) {
                return Err(DomainError::DuplicateWorker(code.clone()));
            }
        }
        Ok(Self { leaders, workers })
    }

    /// Bypasses validation. Reserved for the adapter layer when materialising
    /// rows from persistence; duplicates cannot occur because the
    /// `project_members` PK is `(project_id, team_type, role_type, user_code)`.
    #[allow(dead_code)]
    pub(crate) fn for_repository(leaders: Vec<String>, workers: Vec<String>) -> Self {
        Self { leaders, workers }
    }
}
