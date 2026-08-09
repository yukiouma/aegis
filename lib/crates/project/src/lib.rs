//! # project crate
//!
//! Workspace library providing a SQLx/PostgreSQL-backed DDD repository
//! for `Product` and `Project` aggregates and an async
//! `ProjectUsecase` that orchestrates them and adapts to the
//! `apis::project::ProjectService` port.

pub mod adapter;
pub mod domain;
pub mod usecase;

pub use adapter::facade::in_memory::ProjectServiceImpl;
pub use adapter::persistence::postgres::{ProductRepo, ProjectRepo};
pub use adapter::service::user::UserServiceImpl;
pub use domain::{
    DomainError, Product, ProductNew, ProductRepository, ProductUpdate, Project, ProjectMember,
    ProjectNew, ProjectRepository, ProjectUpdate, RoleType, TeamType, UserService, UserSummary,
};
pub use usecase::{
    CreateProduct, CreateProject, ProductView, ProjectMemberView, ProjectUsecase,
    ProjectUsecaseConfig, ProjectView, UpdateProduct, UpdateProject, UsecaseError, UserSummaryView,
};
