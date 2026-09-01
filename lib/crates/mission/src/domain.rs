//! Domain layer.
//!
//! Pure types, value objects, ports (traits), and `DomainError`.
//! No I/O — no `sqlx`, no `tokio`. Validates inputs and enforces
//! invariants.

mod assignee;
mod error;
mod mission;
mod mission_kind;
mod mission_lookup;
mod mission_role;
mod project_lookup;
#[cfg(test)]
mod tests;
mod user_lookup;

pub use assignee::Assignee;
pub use error::DomainError;
pub use mission::Mission;
pub use mission_kind::MissionKind;
pub use mission_lookup::{
    AssigneeNew, AssigneeRepository, MissionNew, MissionRepository,
    assignees_within_mission_are_unique,
};
pub use mission_role::MissionRole;
pub use project_lookup::ProjectLookup;
pub use user_lookup::UserLookup;
