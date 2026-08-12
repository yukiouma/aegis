//! Outbound HTTP client for the aegis-server.
//!
//! Modules here know nothing about Tauri. The `commands/` layer adapts each
//! function to a `#[tauri::command]` shim.

pub mod config;
pub mod dto;

// Filled in by Tasks 4–10:
// pub mod auth;
// pub mod client;
// pub mod healthz;
// pub mod product;
// pub mod project;
// pub mod user;
// pub mod user_credential;
