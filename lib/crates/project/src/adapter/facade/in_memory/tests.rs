//! End-to-end tests for the apis `ProjectService` facade, exercised
//! against in-memory repository + user-service fakes.

use std::collections::HashMap;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};

use apis::project::{CreateProductRequest, CreateProjectRequest, ProjectApiError, ProjectService};

use crate::adapter::facade::in_memory::ProjectServiceImpl;
use crate::domain::{
    DomainError, Product, ProductNew, ProductRepository, ProductUpdate, Project, ProjectMember,
    ProjectNew, ProjectRepository, ProjectUpdate, RoleType, TeamType, UserService, UserSummary,
};
use crate::usecase::{ProjectUsecase, ProjectUsecaseConfig};

fn mock_now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 9, 0, 0, 0).unwrap()
}

// ---------- in-memory fakes ----------

#[derive(Default)]
struct InMemProductState {
    products: HashMap<i32, Product>,
    next_id: AtomicI32,
}

#[derive(Clone, Default)]
struct InMemProductRepo {
    state: Arc<Mutex<InMemProductState>>,
}

impl InMemProductRepo {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(InMemProductState {
                products: HashMap::new(),
                next_id: AtomicI32::new(1),
            })),
        }
    }
}

#[async_trait]
impl ProductRepository for InMemProductRepo {
    async fn create(&self, input: ProductNew) -> Result<Product, DomainError> {
        let mut s = self.state.lock().unwrap();
        if s.products.values().any(|p| p.code == input.code) {
            return Err(DomainError::DuplicateCode(
                "(constraint products_code_unique)".into(),
            ));
        }
        let id = s.next_id.fetch_add(1, Ordering::SeqCst);
        let now = mock_now();
        let p = Product::for_repository(
            id,
            input.code,
            input.name,
            input.description,
            true,
            now,
            now,
        );
        s.products.insert(id, p.clone());
        Ok(p)
    }
    async fn find_by_id(&self, id: i32) -> Result<Product, DomainError> {
        self.state
            .lock()
            .unwrap()
            .products
            .get(&id)
            .cloned()
            .ok_or(DomainError::NotFound)
    }
    async fn find_by_code(&self, code: &str) -> Result<Product, DomainError> {
        self.state
            .lock()
            .unwrap()
            .products
            .values()
            .find(|p| p.code == code)
            .cloned()
            .ok_or(DomainError::NotFound)
    }
    async fn list(&self) -> Result<Vec<Product>, DomainError> {
        Ok(self.state.lock().unwrap().products.values().cloned().collect())
    }
    async fn update(&self, input: ProductUpdate) -> Result<Product, DomainError> {
        let mut s = self.state.lock().unwrap();
        if let Some(ref c) = input.code {
            let dup = s
                .products
                .values()
                .any(|o| o.code == *c && o.id != input.id);
            if dup {
                return Err(DomainError::DuplicateCode(
                    "(constraint products_code_unique)".into(),
                ));
            }
        }
        let p = s.products.get_mut(&input.id).ok_or(DomainError::NotFound)?;
        if let Some(ref c) = input.code {
            p.code = c.clone();
        }
        if let Some(ref n) = input.name {
            p.name = n.clone();
        }
        if let Some(ref d) = input.description {
            p.description = d.clone();
        }
        if let Some(a) = input.active {
            p.active = a;
        }
        Ok(p.clone())
    }
}

#[derive(Default)]
struct InMemProjectState {
    projects: HashMap<i32, Project>,
    next_id: AtomicI32,
}

#[derive(Clone, Default)]
struct InMemProjectRepo {
    state: Arc<Mutex<InMemProjectState>>,
}

impl InMemProjectRepo {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(InMemProjectState {
                projects: HashMap::new(),
                next_id: AtomicI32::new(1),
            })),
        }
    }
}

#[async_trait]
impl ProjectRepository for InMemProjectRepo {
    async fn create(&self, input: ProjectNew) -> Result<Project, DomainError> {
        let mut s = self.state.lock().unwrap();
        if s.projects.values().any(|p| p.code == input.code) {
            return Err(DomainError::DuplicateCode(
                "(constraint projects_code_unique)".into(),
            ));
        }
        let id = s.next_id.fetch_add(1, Ordering::SeqCst);
        let now = mock_now();
        let members = input.members.clone().unwrap_or_default();
        let unblind = input.unblind_members.clone().unwrap_or_default();
        let project = Project::for_repository(
            id,
            input.code,
            input.description,
            input.product_id,
            members,
            unblind,
            true,
            now,
            now,
        );
        s.projects.insert(id, project.clone());
        Ok(project)
    }
    async fn find_by_id(&self, id: i32) -> Result<Project, DomainError> {
        let s = self.state.lock().unwrap();
        let p = s.projects.get(&id).cloned().ok_or(DomainError::NotFound)?;
        Ok(p)
    }
    async fn find_by_code(&self, code: &str) -> Result<Project, DomainError> {
        let s = self.state.lock().unwrap();
        s.projects
            .values()
            .find(|p| p.code == code)
            .cloned()
            .ok_or(DomainError::NotFound)
    }
    async fn list(&self) -> Result<Vec<Project>, DomainError> {
        Ok(self.state.lock().unwrap().projects.values().cloned().collect())
    }
    async fn update(&self, input: ProjectUpdate) -> Result<Project, DomainError> {
        let mut s = self.state.lock().unwrap();
        if let Some(ref c) = input.code {
            let dup = s
                .projects
                .values()
                .any(|o| o.code == *c && o.id != input.id);
            if dup {
                return Err(DomainError::DuplicateCode(
                    "(constraint projects_code_unique)".into(),
                ));
            }
        }
        let p = s.projects.get_mut(&input.id).ok_or(DomainError::NotFound)?;
        if let Some(ref c) = input.code {
            p.code = c.clone();
        }
        if let Some(ref d) = input.description {
            p.description = d.clone();
        }
        if let Some(pid) = input.product_id {
            p.product_id = pid;
        }
        if let Some(a) = input.active {
            p.active = a;
        }
        if let Some(ref m) = input.members {
            p.members = m.clone();
        }
        if let Some(ref m) = input.unblind_members {
            p.unblind_members = m.clone();
        }
        Ok(p.clone())
    }
}

#[derive(Clone, Default)]
struct InMemUserService {
    users: Arc<Mutex<Vec<UserSummary>>>,
}

impl InMemUserService {
    fn with_users(users: Vec<UserSummary>) -> Self {
        Self {
            users: Arc::new(Mutex::new(users)),
        }
    }
}

#[async_trait]
impl UserService for InMemUserService {
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

fn make_service() -> ProjectServiceImpl<InMemProductRepo, InMemProjectRepo, InMemUserService> {
    let products = InMemProductRepo::new();
    let projects = InMemProjectRepo::new();
    let users = InMemUserService::with_users(vec![
        UserSummary { code: "u1".into(), name: "Alice".into() },
        UserSummary { code: "u2".into(), name: "Bob".into() },
        UserSummary { code: "u3".into(), name: "Carol".into() },
    ]);
    let usecase = ProjectUsecase::new(ProjectUsecaseConfig {
        product_repo: products,
        project_repo: projects,
        users,
    });
    ProjectServiceImpl::new(usecase)
}

#[tokio::test]
async fn create_then_get_product_round_trip() {
    let service = make_service();
    let created = service
        .create_product(CreateProductRequest {
            code: "p1".into(),
            name: "Widget".into(),
            description: "desc".into(),
        })
        .await
        .expect("create");
    assert_eq!(created.code, "p1");
    let by_id = service.get_product_by_id(created.id).await.expect("by id");
    assert_eq!(by_id.id, created.id);
    let by_code = service.get_product_by_code("p1").await.expect("by code");
    assert_eq!(by_code.id, created.id);
    let list = service.list_products().await.expect("list");
    assert_eq!(list.len(), 1);
}

#[tokio::test]
async fn update_product_flips_active() {
    let service = make_service();
    let created = service
        .create_product(CreateProductRequest {
            code: "p1".into(),
            name: "Widget".into(),
            description: "".into(),
        })
        .await
        .expect("create");
    let updated = service
        .update_product(apis::project::UpdateProductRequest {
            id: created.id,
            active: Some(false),
            ..Default::default()
        })
        .await
        .expect("update");
    assert!(!updated.active);
}

#[tokio::test]
async fn create_project_with_none_membership_returns_empty_views() {
    let service = make_service();
    let _ = service
        .create_product(CreateProductRequest {
            code: "p1".into(),
            name: "Widget".into(),
            description: "".into(),
        })
        .await
        .expect("seed");
    let product_id = service.list_products().await.unwrap()[0].id;

    let view = service
        .create_project(CreateProjectRequest {
            code: "proj1".into(),
            description: "".into(),
            product_id,
            members: None,
            unblind_members: None,
        })
        .await
        .expect("create");
    assert!(view.members.leaders.is_empty());
    assert!(view.members.workers.is_empty());
    assert!(view.unblind_members.leaders.is_empty());
    assert!(view.unblind_members.workers.is_empty());
}

#[tokio::test]
async fn create_project_with_some_empty_membership_equivalent_to_none() {
    let service = make_service();
    let _ = service
        .create_product(CreateProductRequest {
            code: "p1".into(),
            name: "Widget".into(),
            description: "".into(),
        })
        .await
        .expect("seed");
    let product_id = service.list_products().await.unwrap()[0].id;

    let view = service
        .create_project(CreateProjectRequest {
            code: "proj1".into(),
            description: "".into(),
            product_id,
            members: Some(Default::default()),
            unblind_members: Some(Default::default()),
        })
        .await
        .expect("create");
    assert!(view.members.leaders.is_empty());
    assert!(view.unblind_members.leaders.is_empty());
}

#[tokio::test]
async fn create_project_hydrates_full_membership() {
    let service = make_service();
    let _ = service
        .create_product(CreateProductRequest {
            code: "p1".into(),
            name: "Widget".into(),
            description: "".into(),
        })
        .await
        .expect("seed");
    let product_id = service.list_products().await.unwrap()[0].id;

    let view = service
        .create_project(CreateProjectRequest {
            code: "proj1".into(),
            description: "".into(),
            product_id,
            members: Some(apis::project::ProjectMemberData {
                leaders: vec!["u1".into()],
                workers: vec!["u2".into()],
            }),
            unblind_members: Some(apis::project::ProjectMemberData {
                leaders: vec!["u3".into()],
                workers: vec![],
            }),
        })
        .await
        .expect("create");
    assert_eq!(view.members.leaders[0].code, "u1");
    assert_eq!(view.members.workers[0].code, "u2");
    assert_eq!(view.unblind_members.leaders[0].code, "u3");
}

#[tokio::test]
async fn create_project_with_unknown_member_returns_user_not_found() {
    let service = make_service();
    let _ = service
        .create_product(CreateProductRequest {
            code: "p1".into(),
            name: "Widget".into(),
            description: "".into(),
        })
        .await
        .expect("seed");
    let product_id = service.list_products().await.unwrap()[0].id;

    let err = service
        .create_project(CreateProjectRequest {
            code: "proj1".into(),
            description: "".into(),
            product_id,
            members: Some(apis::project::ProjectMemberData {
                leaders: vec!["ghost".into()],
                workers: vec![],
            }),
            unblind_members: None,
        })
        .await
        .expect_err("unknown member");
    assert!(matches!(err, ProjectApiError::UserNotFound(ref c) if c == "ghost"));
}

#[tokio::test]
async fn create_project_with_missing_product_returns_product_not_found() {
    let service = make_service();
    let err = service
        .create_project(CreateProjectRequest {
            code: "proj1".into(),
            description: "".into(),
            product_id: 999,
            members: None,
            unblind_members: None,
        })
        .await
        .expect_err("missing product");
    assert!(matches!(err, ProjectApiError::ProductNotFound(ref s) if s == "999"));
}

#[tokio::test]
async fn update_project_replaces_membership_whole_list() {
    let service = make_service();
    let _ = service
        .create_product(CreateProductRequest {
            code: "p1".into(),
            name: "Widget".into(),
            description: "".into(),
        })
        .await
        .expect("seed");
    let product_id = service.list_products().await.unwrap()[0].id;

    let created = service
        .create_project(CreateProjectRequest {
            code: "proj1".into(),
            description: "".into(),
            product_id,
            members: Some(apis::project::ProjectMemberData {
                leaders: vec!["u1".into()],
                workers: vec![],
            }),
            unblind_members: None,
        })
        .await
        .expect("create");
    let updated = service
        .update_project(apis::project::UpdateProjectRequest {
            id: created.id,
            members: Some(apis::project::ProjectMemberData {
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
async fn project_service_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<ProjectServiceImpl<InMemProductRepo, InMemProjectRepo, InMemUserService>>();
}

#[tokio::test]
async fn project_service_box_dyn_compiles() {
    let service = make_service();
    let _boxed: Box<dyn ProjectService> = Box::new(service);
}

// silence unused import warnings for enums the tests exercise via
// the in-memory fakes
#[allow(dead_code)]
fn _force_use_team_role() -> (TeamType, RoleType) {
    (TeamType::Members, RoleType::Leader)
}

// silence unused-import for the connector use
#[allow(dead_code)]
fn _force_use_project_member() -> ProjectMember {
    ProjectMember::default()
}
