//! Adapter layer.
//!
//! Houses the persistence adapters that implement the
//! `ProductRepository` and `ProjectRepository` ports, plus outbound
//! port adapters (the `UserService` facade adapting
//! `apis::user::UserService`, and the in-memory `ProjectServiceImpl`
//! facade adapting `ProjectUsecase` to `apis::project::ProjectService`).
//!
//! Storage-specific implementations live under `persistence/<backend>/`.

pub mod facade;
pub mod persistence;
pub mod service;
