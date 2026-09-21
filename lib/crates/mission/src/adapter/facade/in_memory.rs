//! In-memory facade adapters. `MissionServiceImpl` implements
//! `apis::mission::MissionService` on top of
//! `MissionUsecase` + `MissionIssueUsecase`.

pub mod issue_service;
pub mod service;
