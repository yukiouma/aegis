//! Tests for the usecase layer.
//!
//! Mock repositories + a mock `UserService` stand in for the real
//! adapters so the orchestration + view projection can be exercised
//! without infrastructure.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};

use crate::domain::{
    DomainError, Product, ProductNew, ProductRepository, ProductUpdate, Project, ProjectMember,
    ProjectNew, ProjectRepository, ProjectUpdate, UserService, UserSummary,
};
use crate::usecase::commands::{CreateProduct, CreateProject, UpdateProduct, UpdateProject};
use crate::usecase::error::UsecaseError;
use crate::usecase::project_usecase::{ProjectUsecase, ProjectUsecaseConfig};

fn mock_now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 9, 0, 0, 0).unwrap()
}

// ---------- mock product repo ----------

#[derive(Default)]
struct MockProductState {
    products: HashMap<i32, Product>,
    next_id: i32,
}

#[derive(Clone, Default)]
struct MockProductRepo {
    state: Arc<Mutex<MockProductState>>,
}

impl MockProductRepo {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(MockProductState {
                products: HashMap::new(),
                next_id: 1,
            })),
        }
    }
    fn with_products(products: Vec<Product>) -> Self {
        let max_id = products.iter().map(|p| p.id).max().unwrap_or(0);
        let mut map = HashMap::new();
        for p in products {
            map.insert(p.id, p);
        }
        Self {
            state: Arc::new(Mutex::new(MockProductState {
                products: map,
                next_id: max_id + 1,
            })),
        }
    }
}

#[async_trait]
impl ProductRepository for MockProductRepo {
    async fn create(&self, input: ProductNew) -> Result<Product, DomainError> {
        let mut s = self.state.lock().unwrap();
        if s.products.values().any(|p| p.code == input.code) {
            return Err(DomainError::DuplicateCode(
                "(constraint products_code_unique)".into(),
            ));
        }
        let id = s.next_id;
        s.next_id += 1;
        let now = mock_now();
        let product = Product::for_repository(
            id,
            input.code,
            input.name,
            input.description,
            true,
            now,
            now,
        );
        s.products.insert(id, product.clone());
        Ok(product)
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
        // Duplicate-code check happens first against the immutable map.
        if let Some(ref code) = input.code {
            let dup = s
                .products
                .values()
                .any(|other| other.code == *code && other.id != input.id);
            if dup {
                return Err(DomainError::DuplicateCode(
                    "(constraint products_code_unique)".into(),
                ));
            }
        }
        let p = s.products.get_mut(&input.id).ok_or(DomainError::NotFound)?;
        if let Some(ref code) = input.code {
            p.code = code.clone();
        }
        if let Some(ref name) = input.name {
            p.name = name.clone();
        }
        if let Some(ref desc) = input.description {
            p.description = desc.clone();
        }
        if let Some(active) = input.active {
            p.active = active;
        }
        Ok(p.clone())
    }
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
        let project = Project::for_repository(
            id,
            input.code,
            input.description,
            input.product_id,
            members,
            unblind_members,
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
        Ok(self.state.lock().unwrap().projects.values().cloned().collect())
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
        if let Some(pid) = input.product_id {
            p.product_id = pid;
        }
        if let Some(active) = input.active {
            p.active = active;
        }
        // Replace membership wholesale per team.
        if let Some(ref m) = input.members {
            p.members = m.clone();
        }
        if let Some(ref m) = input.unblind_members {
            p.unblind_members = m.clone();
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
    MockProductRepo,
    MockProjectRepo,
    MockUserService,
    ProjectUsecase<MockProductRepo, MockProjectRepo, MockUserService>,
) {
    let products = MockProductRepo::new();
    let projects = MockProjectRepo::new();
    let users = MockUserService::with_users(vec![
        UserSummary { code: "u1".into(), name: "Alice".into() },
        UserSummary { code: "u2".into(), name: "Bob".into() },
        UserSummary { code: "u3".into(), name: "Carol".into() },
    ]);
    let usecase = ProjectUsecase::new(ProjectUsecaseConfig {
        product_repo: products.clone(),
        project_repo: projects.clone(),
        users: users.clone(),
    });
    (products, projects, users, usecase)
}

fn seed_product(id: i32, code: &str) -> Product {
    let now = mock_now();
    Product::for_repository(id, code.into(), "P".into(), "".into(), true, now, now)
}

// ---------- tests ----------

#[tokio::test]
async fn create_product_returns_view() {
    let (_products, _projects, _users, usecase) = make_usecase();
    let view = usecase
        .create_product(CreateProduct {
            code: "p1".into(),
            name: "Widget".into(),
            description: "desc".into(),
        })
        .await
        .expect("create succeeds");
    assert_eq!(view.id, 1);
    assert_eq!(view.code, "p1");
    assert_eq!(view.name, "Widget");
    assert!(view.active);
}

#[tokio::test]
async fn create_product_rejects_empty_code() {
    let (_p, _r, _u, usecase) = make_usecase();
    let err = usecase
        .create_product(CreateProduct {
            code: "  ".into(),
            name: "Widget".into(),
            description: "".into(),
        })
        .await
        .expect_err("blank code rejected");
    assert!(matches!(err, UsecaseError::Validation(DomainError::EmptyCode)));
}

#[tokio::test]
async fn get_product_by_code_returns_view() {
    let product = seed_product(5, "p5");
    let products = MockProductRepo::with_products(vec![product.clone()]);
    let usecase = ProjectUsecase::new(ProjectUsecaseConfig {
        product_repo: products,
        project_repo: MockProjectRepo::new(),
        users: MockUserService::default(),
    });
    let view = usecase.get_product_by_code("p5").await.expect("found");
    assert_eq!(view.id, 5);
}

#[tokio::test]
async fn list_products_returns_all_views() {
    let products = MockProductRepo::with_products(vec![
        seed_product(1, "p1"),
        seed_product(2, "p2"),
    ]);
    let usecase = ProjectUsecase::new(ProjectUsecaseConfig {
        product_repo: products,
        project_repo: MockProjectRepo::new(),
        users: MockUserService::default(),
    });
    let views = usecase.list_products().await.expect("list");
    assert_eq!(views.len(), 2);
}

#[tokio::test]
async fn update_product_flips_active_flag() {
    let product = seed_product(1, "p1");
    let products = MockProductRepo::with_products(vec![product]);
    let usecase = ProjectUsecase::new(ProjectUsecaseConfig {
        product_repo: products,
        project_repo: MockProjectRepo::new(),
        users: MockUserService::default(),
    });
    let view = usecase
        .update_product(UpdateProduct {
            id: 1,
            active: Some(false),
            ..Default::default()
        })
        .await
        .expect("update");
    assert!(!view.active);
}

#[tokio::test]
async fn create_project_without_membership_succeeds() {
    let products = MockProductRepo::with_products(vec![seed_product(1, "p1")]);
    let usecase = ProjectUsecase::new(ProjectUsecaseConfig {
        product_repo: products,
        project_repo: MockProjectRepo::new(),
        users: MockUserService::default(),
    });
    let view = usecase
        .create_project(CreateProject {
            code: "proj1".into(),
            description: "".into(),
            product_id: 1,
            members: None,
            unblind_members: None,
        })
        .await
        .expect("create");
    assert_eq!(view.code, "proj1");
    assert!(view.members.leaders.is_empty());
    assert!(view.members.workers.is_empty());
    assert!(view.unblind_members.leaders.is_empty());
    assert!(view.unblind_members.workers.is_empty());
}

#[tokio::test]
async fn create_project_hydrates_membership() {
    let products = MockProductRepo::with_products(vec![seed_product(1, "p1")]);
    let usecase = ProjectUsecase::new(ProjectUsecaseConfig {
        product_repo: products,
        project_repo: MockProjectRepo::new(),
        users: MockUserService::with_users(vec![
            UserSummary { code: "u1".into(), name: "Alice".into() },
            UserSummary { code: "u2".into(), name: "Bob".into() },
        ]),
    });
    let view = usecase
        .create_project(CreateProject {
            code: "proj1".into(),
            description: "".into(),
            product_id: 1,
            members: Some(ProjectMember {
                leaders: vec!["u1".into()],
                workers: vec!["u2".into()],
            }),
            unblind_members: Some(ProjectMember::default()),
        })
        .await
        .expect("create");
    assert_eq!(view.members.leaders.len(), 1);
    assert_eq!(view.members.leaders[0].code, "u1");
    assert_eq!(view.members.workers[0].code, "u2");
}

#[tokio::test]
async fn create_project_with_unknown_member_returns_user_not_found() {
    let products = MockProductRepo::with_products(vec![seed_product(1, "p1")]);
    let usecase = ProjectUsecase::new(ProjectUsecaseConfig {
        product_repo: products,
        project_repo: MockProjectRepo::new(),
        users: MockUserService::with_users(vec![UserSummary {
            code: "u1".into(),
            name: "Alice".into(),
        }]),
    });
    let err = usecase
        .create_project(CreateProject {
            code: "proj1".into(),
            description: "".into(),
            product_id: 1,
            members: Some(ProjectMember {
                leaders: vec!["ghost".into()],
                workers: vec![],
            }),
            unblind_members: None,
        })
        .await
        .expect_err("unknown member rejected");
    assert!(
        matches!(err, UsecaseError::Repository(DomainError::UserNotFound(ref c)) if c == "ghost"),
        "got {err:?}"
    );
}

#[tokio::test]
async fn create_project_with_missing_product_returns_product_not_found() {
    let usecase = ProjectUsecase::new(ProjectUsecaseConfig {
        product_repo: MockProductRepo::new(),
        project_repo: MockProjectRepo::new(),
        users: MockUserService::default(),
    });
    let err = usecase
        .create_project(CreateProject {
            code: "proj1".into(),
            description: "".into(),
            product_id: 999,
            members: None,
            unblind_members: None,
        })
        .await
        .expect_err("missing product");
    assert!(
        matches!(err, UsecaseError::Repository(DomainError::ProductNotFound(ref s)) if s == "999"),
        "got {err:?}"
    );
}

#[tokio::test]
async fn update_project_replaces_membership_whole_list() {
    let products = MockProductRepo::with_products(vec![seed_product(1, "p1")]);
    let usecase = ProjectUsecase::new(ProjectUsecaseConfig {
        product_repo: products,
        project_repo: MockProjectRepo::new(),
        users: MockUserService::with_users(vec![
            UserSummary { code: "u1".into(), name: "Alice".into() },
            UserSummary { code: "u2".into(), name: "Bob".into() },
            UserSummary { code: "u3".into(), name: "Carol".into() },
        ]),
    });
    let created = usecase
        .create_project(CreateProject {
            code: "proj1".into(),
            description: "".into(),
            product_id: 1,
            members: Some(ProjectMember {
                leaders: vec!["u1".into()],
                workers: vec![],
            }),
            unblind_members: None,
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
async fn list_projects_returns_all_views() {
    let products = MockProductRepo::with_products(vec![seed_product(1, "p1")]);
    let projects = MockProjectRepo::new();
    let usecase = ProjectUsecase::new(ProjectUsecaseConfig {
        product_repo: products,
        project_repo: projects.clone(),
        users: MockUserService::with_users(vec![]),
    });
    let _ = usecase
        .create_project(CreateProject {
            code: "p1".into(),
            description: "".into(),
            product_id: 1,
            members: None,
            unblind_members: None,
        })
        .await
        .unwrap();
    let _ = usecase
        .create_project(CreateProject {
            code: "p2".into(),
            description: "".into(),
            product_id: 1,
            members: None,
            unblind_members: None,
        })
        .await
        .unwrap();
    let list = usecase.list_projects().await.expect("list");
    assert_eq!(list.len(), 2);
}

#[tokio::test]
async fn project_usecase_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<ProjectUsecase<MockProductRepo, MockProjectRepo, MockUserService>>();
}
