//! Adapter layer.
//!
//! Hosts three sub-modules:
//! - `persistence` — SQLx/PostgreSQL implementations of the repository
//!   ports.
//! - `service` — adapters that bridge another workspace crate's port
//!   into a domain port.
//! - `facade` — implementations of the API trait defined in
//!   `apis::sdtm_project::SdtmProjectService`.

pub mod facade;
pub mod persistence;
pub mod service;
