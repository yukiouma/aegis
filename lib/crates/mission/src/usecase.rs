//! Usecase layer.
//!
//! `MissionUsecase<M, A, P, U>` orchestrates the four ports
//! (mission, assignee, project lookup, user lookup) and surfaces
//! `UsecaseError`. Every write method calls
//! `project_lookup.is_leader` and projects the domain aggregate
//! into the `MissionView` / `AssigneeView` DTOs the facade maps
//! to the apis port types.

mod commands;
mod error;
mod mission_usecase;
mod views;

#[cfg(test)]
mod tests;

pub use commands::{AssigneeData, CreateMission};
pub use error::UsecaseError;
pub use mission_usecase::{MissionUsecase, MissionUsecaseConfig};
pub use views::{AssigneeView, MissionView};
