//! In-memory facade: the only facade implementation today.
//!
//! Holds a `ProjectUsecase<P, R, U>` and projects its results into
//! the apis `ProjectView` / `ProductView` types. We keep the module
//! name for now (the user crate uses the same layout) so future
//! storage-specific facades can sit alongside it.

mod service;
#[cfg(test)]
mod tests;

pub use service::ProjectServiceImpl;
