//! In-memory `UserService` adapter.
//!
//! Hosts `UserServiceImpl<R>`, the implementation of
//! `apis::user::UserService` that adapts `user::UserUsecase` to the
//! API contract. Behaviour is exercised by `tests`, which wires the
//! adapter on top of an in-memory `UserRepository` so no live
//! PostgreSQL connection is required.