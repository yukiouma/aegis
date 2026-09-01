//! `mission` workspace crate.
//!
//! Hosts the `Mission` and `Assignee` aggregates, their ports,
//! the PostgreSQL-backed persistence adapters, the cross-crate
//! `ProjectLookup` / `UserLookup` adapters, the usecase layer
//! that orchestrates them with project-leader authorization, and
//! the in-memory facade that adapts `MissionUsecase` to
//! `apis::mission::MissionService`.
//!
//! Layered architecture:
//!
//! ```text
//! mission crate
//! └── adapter
//!     ├── facade                  (MissionServiceImpl<M, A, P, U>)
//!     ├── persistence             (MissionRepoPg, AssigneeRepoPg)
//!     └── service                 (ProjectLookupImpl, UserLookupImpl)
//! usecase
//!     └── MissionUsecase<M, A, P, U>
//! domain
//!     └── Mission, Assignee,
//!         MissionKind, MissionRole,
//!         MissionRepository, AssigneeRepository,
//!         ProjectLookup, UserLookup,
//!         DomainError
//! ```

pub mod adapter;
pub mod domain;
pub mod usecase;

pub use adapter::facade::in_memory::MissionServiceImpl;
pub use adapter::persistence::postgres::{AssigneeRepo, MissionRepo};
pub use adapter::service::project::ProjectLookupImpl;
pub use adapter::service::user::UserLookupImpl;
pub use domain::{
    Assignee, AssigneeRepository, DomainError, Mission, MissionKind, MissionRepository,
    MissionRole, ProjectLookup, UserLookup,
};
pub use usecase::{MissionUsecase, MissionUsecaseConfig, UsecaseError};
