//! Outbound-port adapters that sit on top of an in-memory
//! `UserRepository`.
//!
//! Today this module hosts a single backend — the
//! [`UserServiceImpl`](service::UserServiceImpl) that adapts
//! `user::UserUsecase` to `apis::user::UserService`. Additional
//! backends (e.g. a future Postgres-backed implementation) can be
//! added as siblings under this module without disturbing the
//! current re-export point.

mod service;

pub use service::UserServiceImpl;