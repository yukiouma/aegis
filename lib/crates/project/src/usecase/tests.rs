//! Tests for the usecase layer.
//!
//! Mock repository + a mock `UserService` stand in for the real
//! adapters so the orchestration + view projection can be exercised
//! without infrastructure.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};

use crate::domain::{
    DomainError, Project, ProjectMember, ProjectNew, ProjectRepository, ProjectTag, ProjectUpdate,
    UserService, UserSummary,
};
use crate::usecase::commands::{CreateProject, UpdateProject};
use crate::usecase::error::UsecaseError;
use crate::usecase::project_usecase::{ProjectUsecase, ProjectUsecaseConfig};

fn mock_now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 9, 0, 0, 0).unwrap()
}

// ---------- mock project repo ----------

#[derive(Default)]
struct MockProjectState {
    projects: HashMap<i32, Project>,
    next_id: i32,
}

#[derive(Clone, Default)]
struct MockProjectRepo {
    state: Arc<Mutex<MockProjectState>>,
}

impl MockProjectRepo {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(MockProjectState {
                projects: HashMap::new(),
                next_id: 1,
            })),
        }
    }
}

#[async_trait]
impl ProjectRepository for MockProjectRepo {
    async fn create(&self, input: ProjectNew) -> Result<Project, DomainError> {
        let mut s = self.state.lock().unwrap();
        if s.projects.values().any(|p| p.code == input.code) {
            return Err(DomainError::DuplicateCode(
                "(constraint projects_code_unique)".into(),
            ));
        }
        let id = s.next_id;
        s.next_id += 1;
        let now = mock_now();
        let members = input.members.unwrap_or_default();
        let unblind_members = input.unblind_members.unwrap_or_default();
        let tags = input.tags.unwrap_or_default();
        let project = Project::for_repository(
            id,
            input.code,
            input.description,
            members,
            unblind_members,
            tags,
            true,
            now,
            now,
        );
        s.projects.insert(id, project.clone());
        Ok(project)
    }
    async fn find_by_id(&self, id: i32) -> Result<Project, DomainError> {
        self.state
            .lock()
            .unwrap()
            .projects
            .get(&id)
            .cloned()
            .ok_or(DomainError::NotFound)
    }
    async fn find_by_code(&self, code: &str) -> Result<Project, DomainError> {
        self.state
            .lock()
            .unwrap()
            .projects
            .values()
            .find(|p| p.code == code)
            .cloned()
            .ok_or(DomainError::NotFound)
    }
    async fn list(&self) -> Result<Vec<Project>, DomainError> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .projects
            .values()
            .cloned()
            .collect())
    }
    async fn update(&self, input: ProjectUpdate) -> Result<Project, DomainError> {
        let mut s = self.state.lock().unwrap();
        if let Some(ref code) = input.code {
            let dup = s
                .projects
                .values()
                .any(|other| other.code == *code && other.id != input.id);
            if dup {
                return Err(DomainError::DuplicateCode(
                    "(constraint projects_code_unique)".into(),
                ));
            }
        }
        let p = s.projects.get_mut(&input.id).ok_or(DomainError::NotFound)?;
        if let Some(ref code) = input.code {
            p.code = code.clone();
        }
        if let Some(ref desc) = input.description {
            p.description = desc.clone();
        }
        if let Some(a) = input.active {
            p.active = a;
        }
        // Replace membership wholesale per team.
        if let Some(ref m) = input.members {
            p.members = m.clone();
        }
        if let Some(ref m) = input.unblind_members {
            p.unblind_members = m.clone();
        }
        if let Some(ref tags) = input.tags {
            p.tags = tags.clone();
        }
        Ok(p.clone())
    }
}

// ---------- mock user service ----------

#[derive(Clone, Default)]
struct MockUserService {
    users: Arc<Mutex<Vec<UserSummary>>>,
}

impl MockUserService {
    fn with_users(users: Vec<UserSummary>) -> Self {
        Self {
            users: Arc::new(Mutex::new(users)),
        }
    }
}

#[async_trait]
impl UserService for MockUserService {
    async fn get_by_code(&self, code: &str) -> Result<UserSummary, DomainError> {
        self.users
            .lock()
            .unwrap()
            .iter()
            .find(|u| u.code == code)
            .cloned()
            .ok_or(DomainError::NotFound)
    }
    async fn list(&self) -> Result<Vec<UserSummary>, DomainError> {
        Ok(self.users.lock().unwrap().clone())
    }
}

// ---------- fixtures ----------

fn make_usecase() -> (
    MockProjectRepo,
    MockUserService,
    ProjectUsecase<MockProjectRepo, MockUserService>,
) {
    let projects = MockProjectRepo::new();
    let users = MockUserService::with_users(vec![
        UserSummary {
            code: "u1".into(),
            name: "Alice".into(),
        },
        UserSummary {
            code: "u2".into(),
            name: "Bob".into(),
        },
        UserSummary {
            code: "u3".into(),
            name: "Carol".into(),
        },
    ]);
    let usecase = ProjectUsecase::new(ProjectUsecaseConfig {
        project_repo: projects.clone(),
        users: users.clone(),
    });
    (projects, users, usecase)
}

// ---------- tests ----------

#[tokio::test]
async fn create_project_without_membership_succeeds() {
    let (_projects, _users, usecase) = make_usecase();
    let view = usecase
        .create_project(CreateProject {
            code: "proj1".into(),
            description: "".into(),
            members: None,
            unblind_members: None,
            tags: None,
        })
        .await
        .expect("create");
    assert_eq!(view.code, "proj1");
    assert!(view.members.leaders.is_empty());
    assert!(view.members.workers.is_empty());
    assert!(view.unblind_members.leaders.is_empty());
    assert!(view.unblind_members.workers.is_empty());
    assert!(view.tags.is_empty());
}

#[tokio::test]
async fn create_project_hydrates_membership() {
    let (_projects, _users, usecase) = make_usecase();
    let view = usecase
        .create_project(CreateProject {
            code: "proj1".into(),
            description: "".into(),
            members: Some(ProjectMember {
                leaders: vec!["u1".into()],
                workers: vec!["u2".into()],
            }),
            unblind_members: Some(ProjectMember::default()),
            tags: None,
        })
        .await
        .expect("create");
    assert_eq!(view.members.leaders.len(), 1);
    assert_eq!(view.members.leaders[0].code, "u1");
    assert_eq!(view.members.workers[0].code, "u2");
}

#[tokio::test]
async fn create_project_with_unknown_member_returns_user_not_found() {
    let (_projects, _users, usecase) = make_usecase();
    let err = usecase
        .create_project(CreateProject {
            code: "proj1".into(),
            description: "".into(),
            members: Some(ProjectMember {
                leaders: vec!["ghost".into()],
                workers: vec![],
            }),
            unblind_members: None,
            tags: None,
        })
        .await
        .expect_err("unknown member rejected");
    assert!(
        matches!(err, UsecaseError::Repository(DomainError::UserNotFound(ref c)) if c == "ghost"),
        "got {err:?}"
    );
}

#[tokio::test]
async fn create_project_with_tags_succeeds() {
    let (_projects, _users, usecase) = make_usecase();
    let view = usecase
        .create_project(CreateProject {
            code: "proj1".into(),
            description: "".into(),
            members: None,
            unblind_members: None,
            tags: Some(vec![
                ProjectTag::for_repository("Product".into(), "DEMO-001".into()),
                ProjectTag::for_repository("Region".into(), "EU".into()),
            ]),
        })
        .await
        .expect("create");
    assert_eq!(view.tags.len(), 2);
    assert_eq!(view.tags[0].key, "Product");
    assert_eq!(view.tags[0].value, "DEMO-001");
    assert_eq!(view.tags[1].key, "Region");
    assert_eq!(view.tags[1].value, "EU");
}

#[tokio::test]
async fn create_project_with_duplicate_tag_keys_succeeds() {
    let (_projects, _users, usecase) = make_usecase();
    let view = usecase
        .create_project(CreateProject {
            code: "proj1".into(),
            description: "".into(),
            members: None,
            unblind_members: None,
            tags: Some(vec![
                ProjectTag::for_repository("Product".into(), "DEMO-001".into()),
                ProjectTag::for_repository("Product".into(), "DEMO-002".into()),
            ]),
        })
        .await
        .expect("create");
    assert_eq!(view.tags.len(), 2);
    assert_eq!(view.tags[0].value, "DEMO-001");
    assert_eq!(view.tags[1].value, "DEMO-002");
}

#[tokio::test]
async fn create_project_with_empty_tag_key_returns_validation_error() {
    let (_projects, _users, usecase) = make_usecase();
    let err = usecase
        .create_project(CreateProject {
            code: "proj1".into(),
            description: "".into(),
            members: None,
            unblind_members: None,
            tags: Some(vec![ProjectTag::for_repository("".into(), "v".into())]),
        })
        .await
        .expect_err("empty key rejected");
    assert!(matches!(
        err,
        UsecaseError::Validation(DomainError::EmptyTagKey)
    ));
}

#[tokio::test]
async fn create_project_with_empty_tag_value_returns_validation_error() {
    let (_projects, _users, usecase) = make_usecase();
    let err = usecase
        .create_project(CreateProject {
            code: "proj1".into(),
            description: "".into(),
            members: None,
            unblind_members: None,
            tags: Some(vec![ProjectTag::for_repository("k".into(), "   ".into())]),
        })
        .await
        .expect_err("empty value rejected");
    assert!(matches!(
        err,
        UsecaseError::Validation(DomainError::EmptyTagValue)
    ));
}

#[tokio::test]
async fn update_project_replaces_membership_whole_list() {
    let (_projects, _users, usecase) = make_usecase();
    let created = usecase
        .create_project(CreateProject {
            code: "proj1".into(),
            description: "".into(),
            members: Some(ProjectMember {
                leaders: vec!["u1".into()],
                workers: vec![],
            }),
            unblind_members: None,
            tags: None,
        })
        .await
        .expect("create");
    let updated = usecase
        .update_project(UpdateProject {
            id: created.id,
            members: Some(ProjectMember {
                leaders: vec![],
                workers: vec!["u2".into(), "u3".into()],
            }),
            unblind_members: None,
            ..Default::default()
        })
        .await
        .expect("update");
    assert!(updated.members.leaders.is_empty());
    assert_eq!(updated.members.workers.len(), 2);
}

#[tokio::test]
async fn update_project_replaces_tags_whole_list() {
    let (_projects, _users, usecase) = make_usecase();
    let created = usecase
        .create_project(CreateProject {
            code: "proj1".into(),
            description: "".into(),
            members: None,
            unblind_members: None,
            tags: Some(vec![ProjectTag::for_repository("k1".into(), "v1".into())]),
        })
        .await
        .expect("create");
    assert_eq!(created.tags.len(), 1);

    let updated = usecase
        .update_project(UpdateProject {
            id: created.id,
            tags: Some(vec![
                ProjectTag::for_repository("k2".into(), "v2".into()),
                ProjectTag::for_repository("k3".into(), "v3".into()),
            ]),
            ..Default::default()
        })
        .await
        .expect("update");
    assert_eq!(updated.tags.len(), 2);
    assert_eq!(updated.tags[0].key, "k2");
    assert_eq!(updated.tags[1].key, "k3");
}

#[tokio::test]
async fn update_project_leaves_tags_unchanged_when_none() {
    let (_projects, _users, usecase) = make_usecase();
    let created = usecase
        .create_project(CreateProject {
            code: "proj1".into(),
            description: "".into(),
            members: None,
            unblind_members: None,
            tags: Some(vec![ProjectTag::for_repository("k1".into(), "v1".into())]),
        })
        .await
        .expect("create");
    let updated = usecase
        .update_project(UpdateProject {
            id: created.id,
            description: Some("new".into()),
            ..Default::default()
        })
        .await
        .expect("update");
    assert_eq!(updated.tags.len(), 1);
    assert_eq!(updated.tags[0].key, "k1");
}

#[tokio::test]
async fn list_projects_returns_all_views() {
    let (_projects, _users, usecase) = make_usecase();
    let _ = usecase
        .create_project(CreateProject {
            code: "p1".into(),
            description: "".into(),
            members: None,
            unblind_members: None,
            tags: None,
        })
        .await
        .unwrap();
    let _ = usecase
        .create_project(CreateProject {
            code: "p2".into(),
            description: "".into(),
            members: None,
            unblind_members: None,
            tags: None,
        })
        .await
        .unwrap();
    let list = usecase.list_projects().await.expect("list");
    assert_eq!(list.len(), 2);
}

#[tokio::test]
async fn project_usecase_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<ProjectUsecase<MockProjectRepo, MockUserService>>();
}