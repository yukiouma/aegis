//! Outbound-port adapters.
//!
//! Adapters in this sub-module implement API-facing traits defined
//! in other workspace crates (today:
//! `apis::domain_model::DomainModelService`). Each backend lives
//! under its own child module so a second port (e.g. a future
//! gRPC facade) is purely additive.

mod in_memory;

pub use in_memory::DomainModelServiceImpl;
