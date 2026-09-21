//! Usecase layer.
//!
//! `MissionUsecase<M, A, P, U>` orchestrates the four ports
//! (mission, assignee, project lookup, user lookup) and surfaces
//! `UsecaseError`. `MissionIssueUsecase<M, A, P, I>` orchestrates
//! the four ports (mission, assignee, project lookup, issue repo)
//! and shares the same `UsecaseError`. Every write method calls
//! `project_lookup.is_leader` and projects the domain aggregate
//! into the `MissionView` / `AssigneeView` / `IssueView` DTOs the
//! facade maps to the apis port types.

mod commands;
mod error;
mod issue_usecase;
mod mission_usecase;
mod views;

#[cfg(test)]
mod tests;

pub use commands::{AssigneeData, CreateIssue, CreateMission};
pub use error::UsecaseError;
pub use issue_usecase::{MissionIssueUsecase, MissionIssueUsecaseConfig};
pub use mission_usecase::{MissionUsecase, MissionUsecaseConfig};
pub use views::{AssigneeView, IssueCommentView, IssueView, MissionView};
