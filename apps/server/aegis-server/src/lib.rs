//! # aegis-server
//!
//! HTTP server binary. Wires the `auth` crate's `AuthServiceImpl`
//! against a Postgres pool + in-memory token-version cache, mounts
//! the auth-flow endpoints under `/api/auth/*` with `axum`, and
//! exposes the OpenAPI document at `/api-docs/openapi.json` plus
//! swagger-ui at `/swagger-ui`.
//!
//! The public surface is small (`run`, `Config`, `AppState`,
//! `transport::router`) so the binary entry point stays a thin
//! `main.rs` that parses env, initialises tracing, and calls
//! `aegis_server::run(config)`.

pub mod config;
pub mod state;
pub mod transport;