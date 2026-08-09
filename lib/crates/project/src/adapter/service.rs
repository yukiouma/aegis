//! Outbound port adapters.
//!
//! Adapters from the domain ports to the apis crates live here. Today
//! this only houses the `UserService` adapter that bridges the apis
//! `user::UserService` to the narrow domain `UserService`.

pub mod user;
