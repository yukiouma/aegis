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
//!
//! ## Public API
//!
//! The crate exposes the items downstream consumers need to embed
//! the server elsewhere — or to wire the router into a test
//! harness. The doctest below asserts that the whole public path
//! resolves from a hypothetical consumer crate: `Config` from
//! `config`, `AppState` from `state`, `router` from
//! `transport::http`, and the re-exported `run` function. It does
//! not require a database or runtime — the items are merely
//! named so the compiler is the only thing that runs.
//!
//! ```no_run
//! use aegis_server::config::Config;
//! use aegis_server::state::AppState;
//! use aegis_server::transport::http::router;
//!
//! // Type-only references: as long as these names resolve, the
//! // public surface is intact.
//! let _ = std::any::type_name::<Config>();
//! let _ = std::any::type_name::<AppState>();
//! let _ = std::any::type_name_of_val(&router);
//! let _ = std::any::type_name_of_val(&aegis_server::run);
//! ```

pub mod config;
pub mod run;
pub mod state;
pub mod transport;

pub use run::run;
