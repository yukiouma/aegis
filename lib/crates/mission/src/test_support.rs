//! Shared test fakes for the mission crate. Used by
//! `usecase::tests` and `adapter::facade::in_memory::tests` so
//! the fake definitions live in one place.

#![allow(dead_code)]

use std::sync::Mutex;
use std::sync::atomic::{AtomicI32, Ordering};

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::domain::{
    Assignee, AssigneeNew, AssigneeRepository, DomainError, Mission, MissionKind, MissionNew,
    MissionRepository, ProjectLookup, UserLookup,
};

#[derive(Default)]
pub struct FakeMissionRepo {
    pub next_id: AtomicI32,
    pub missions: Mutex<Vec<Mission>>,
}

#[async_trait]
impl MissionRepository for FakeMissionRepo {
    async fn create(&self, input: MissionNew) -> Result<Mission, DomainError> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst) as i64;
        let now: DateTime<Utc> = Utc::now();
        let assignees = input
            .assignees
            .iter()
            .enumerate()
            .map(|(idx, a)| {
                Assignee::for_repository(
                    id * 1000 + idx as i64,
                    a.user_code.clone(),
                    a.role,
                    now,
                    now,
                )
            })
            .collect();
        let m = Mission::for_repository(
            id,
            input.project_code,
            input.mission_kind,
            input.mission_code,
            assignees,
            now,
            now,
        );
        self.missions.lock().unwrap().push(m.clone());
        Ok(m)
    }
    async fn find_by_id(&self, id: i64) -> Result<Mission, DomainError> {
        self.missions
            .lock()
            .unwrap()
            .iter()
            .find(|m| m.id == id)
            .cloned()
            .ok_or(DomainError::NotFound)
    }
    async fn list_by_project(
        &self,
        project_code: &str,
        kind: Option<MissionKind>,
    ) -> Result<Vec<Mission>, DomainError> {
        Ok(self
            .missions
            .lock()
            .unwrap()
            .iter()
            .filter(|m| m.project_code == project_code && kind.is_none_or(|k| k == m.mission_kind))
            .cloned()
            .collect())
    }
    async fn list_by_user(&self, user_code: &str) -> Result<Vec<Mission>, DomainError> {
        Ok(self
            .missions
            .lock()
            .unwrap()
            .iter()
            .filter(|m| m.assignees.iter().any(|a| a.user_code == user_code))
            .cloned()
            .collect())
    }
    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let mut g = self.missions.lock().unwrap();
        let before = g.len();
        g.retain(|m| m.id != id);
        if g.len() == before {
            Err(DomainError::NotFound)
        } else {
            Ok(())
        }
    }
}

#[derive(Default)]
pub struct FakeAssigneeRepo {
    pub next_id: AtomicI32,
    pub assignees: Mutex<Vec<(i64, Assignee)>>, // (mission_id, assignee)
}

#[async_trait]
impl AssigneeRepository for FakeAssigneeRepo {
    async fn add(&self, mission_id: i64, input: AssigneeNew) -> Result<Assignee, DomainError> {
        let now = Utc::now();
        let a = Assignee::new(
            self.next_id.fetch_add(1, Ordering::SeqCst) as i64,
            input.user_code,
            input.role,
            now,
            now,
        )?;
        let mut g = self.assignees.lock().unwrap();
        if g.iter()
            .any(|(mid, x)| *mid == mission_id && x.user_code == a.user_code && x.role == a.role)
        {
            return Err(DomainError::DuplicateAssignee {
                mission_id,
                user_code: a.user_code.clone(),
                role: a.role,
            });
        }
        g.push((mission_id, a.clone()));
        Ok(a)
    }
    async fn remove(&self, mission_id: i64, assignee_id: i64) -> Result<(), DomainError> {
        let mut g = self.assignees.lock().unwrap();
        let before = g.len();
        g.retain(|(mid, a)| !(*mid == mission_id && a.id == assignee_id));
        if g.len() == before {
            Err(DomainError::AssigneeNotFound)
        } else {
            Ok(())
        }
    }
}

pub struct FakeProject {
    pub leader_for: Vec<&'static str>,
}

#[async_trait]
impl ProjectLookup for FakeProject {
    async fn get_by_code(&self, code: &str) -> Result<(), DomainError> {
        if code == "p1" {
            Ok(())
        } else {
            Err(DomainError::ProjectNotFound(code.into()))
        }
    }
    async fn is_leader(&self, project_code: &str, user_code: &str) -> Result<bool, DomainError> {
        Ok(project_code == "p1" && self.leader_for.contains(&user_code))
    }
}

pub struct FakeUser;

#[async_trait]
impl UserLookup for FakeUser {
    async fn get_by_code(&self, code: &str) -> Result<(), DomainError> {
        if code.starts_with('u') {
            Ok(())
        } else {
            Err(DomainError::UserNotFound(code.into()))
        }
    }
}
