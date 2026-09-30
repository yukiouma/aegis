//! `trace-id` workspace crate — backwards-compat shim.
//!
//! The `TraceIdGenerator` and `Side` types now live in
//! `logging-utils` (their canonical home). This crate re-exports
//! them so existing consumers (`aegis-server`, `aegis-desktop`) keep
//! compiling untouched. Delete this crate once those consumers
//! migrate to `use logging_utils::…` directly.

pub use logging_utils::{Side, TraceIdGenerator};
