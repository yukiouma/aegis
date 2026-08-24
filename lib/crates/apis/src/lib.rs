//! `apis` workspace crate.
//!
//! Hosts outbound port traits that adapters (HTTP/gRPC handlers,
//! other backends) consume. Each trait is a self-contained
//! contract: this crate does not depend on any other workspace
//! crate, so any backend can implement the traits by adapting its
//! own types to the ones defined here.

pub mod auth;
pub mod domain_model;
pub mod project;
pub mod terminology;
pub mod user;
