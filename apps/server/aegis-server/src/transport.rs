//! Transport layer.
//!
//! Houses every supported network transport as a sub-module. Today
//! the only transport is `http`; future transports (gRPC, tarpc,
//! CLI) land as siblings under this module.

pub mod http;

pub use http::router;
