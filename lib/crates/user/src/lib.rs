//! # user crate
//!
//! Workspace library providing a SQLx/PostgreSQL-backed DDD user
//! repository and an async `UserUsecase`.
//!
//! The crate exposes the three DDD layers as sub-modules for power
//! users (`domain`, `usecase`, `infrastructure`) and re-exports the
//! public surface at the crate root so consumers can simply write
//!
//! ```no_run
//! use user::{UserRepo, UserUsecase, CreateUser, UpdateUser, Role, UserView};
//! ```

pub mod domain;
pub mod infrastructure;
pub mod usecase;

// Re-exports for the documented public surface.
//
// `UserRepo` is the SQLx-backed repository implementation that
// consumers wire into a `UserUsecase`. `User` and `UserRepository` are
// re-exported alongside it so consumers who only depend on the port
// can name the trait at the crate root.
pub use domain::{Role, User, UserRepository};
pub use infrastructure::user_repo::UserRepo;
pub use usecase::{CreateUser, UpdateUser, UserUsecase, UserView};
