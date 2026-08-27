//! Adapter layer.
//!
//! Houses three sub-modules:
//!
//! - `persistence` — SQLx/PostgreSQL implementations of the
//!   seven repository ports.
//! - `service` — adapters from the domain ports to the
//!   `apis` crate (today: `ProjectLookupImpl` bridging
//!   `apis::project::ProjectService`).
//! - `facade` — in-memory adapter from `CrfUsecase` to
//!   `apis::crf::CrfService`.

pub mod facade;
pub mod persistence;
pub mod service;
