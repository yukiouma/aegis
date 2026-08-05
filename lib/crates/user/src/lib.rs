//! # user crate
//!
//! Workspace library providing a SQLx/PostgreSQL-backed DDD user
//! repository and an async `UserUsecase`.
//!
//! The crate exposes the three DDD layers as sub-modules for power
//! users (`domain`, `usecase`, `adapter`) and re-exports the
//! public surface at the crate root so consumers can simply write
//!
//! ```no_run
//! use user::{UserRepo, UserUsecase, CreateUser, UpdateUser, Role, UserView};
//! ```

pub mod adapter;
pub mod domain;
pub mod usecase;

// Re-exports for the documented public surface.
//
// `UserRepo` is the SQLx-backed repository implementation that
// consumers wire into a `UserUsecase`. `User` and `UserRepository` are
// re-exported alongside it so consumers who only depend on the port
// can name the trait at the crate root. The error types and input
// DTOs (`UsecaseError`, `DomainError`, `UserNew`, `UserUpdate`) are
// re-exported so consumers can `match` on them and construct
// repository inputs without reaching into the internal modules.
pub use domain::{DomainError, Role, User, UserNew, UserRepository, UserUpdate};
pub use adapter::UserRepo;
pub use usecase::{CreateUser, UpdateUser, UsecaseError, UserUsecase, UserView};
