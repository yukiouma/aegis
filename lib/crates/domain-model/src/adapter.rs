//! Adapter layer.
//!
//! Hosts two sub-modules:
//! - `persistence` — SQLx/PostgreSQL implementations of the
//!   repository ports.
//! - `facade` — implementations of the API trait defined in
//!   `apis::domain_model::DomainModelService`.

pub mod facade;
pub mod persistence;
