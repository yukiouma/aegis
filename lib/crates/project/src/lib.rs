//! # project crate
//!
//! Workspace library providing a SQLx/PostgreSQL-backed DDD repository
//! for the `Project` aggregate (with `ProjectTag` JSONB tags) and an
//! async `ProjectUsecase` that orchestrates them and adapts to the
//! `apis::project::ProjectService` port.

pub mod adapter;
pub mod domain;
pub mod usecase;

pub use adapter::facade::in_memory::ProjectServiceImpl;
pub use adapter::persistence::postgres::ProjectRepo;
pub use adapter::service::user::UserServiceImpl;
pub use domain::{
    DomainError, Project, ProjectMember, ProjectNew, ProjectRepository, ProjectTag, ProjectUpdate,
    RoleType, TeamType, UserService, UserSummary,
};
pub use usecase::{
    CreateProject, ProjectMemberView, ProjectUsecase, ProjectUsecaseConfig, ProjectView, TagView,
    UpdateProject, UsecaseError, UserSummaryView,
};