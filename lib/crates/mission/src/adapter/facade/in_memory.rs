//! In-memory facade.
//!
//! Holds a `MissionUsecase<M, A, P, U>` and projects its results
//! into the apis `MissionView` / `AssigneeView` types. The only
//! facade today.

mod service;
#[cfg(test)]
mod tests;

pub use service::MissionServiceImpl;
