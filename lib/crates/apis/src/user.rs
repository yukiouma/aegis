//! Outbound port for user lifecycle operations.
//!
//! See [`UserService`] for the trait surface. All supporting
//! types (`Role`, `UserApiError`, `UserView`, `CreateUserRequest`,
//! `UpdateUserRequest`) are defined alongside the trait so a
//! single `use apis::user::*;` brings the whole contract into
//! scope.