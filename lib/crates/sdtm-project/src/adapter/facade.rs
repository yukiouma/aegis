//! Outbound-port adapters.
//!
//! Adapters in this sub-module implement API-facing traits defined in
//! other workspace crates (today: `apis::sdtm_project::SdtmProjectService`).

mod in_memory;

pub use in_memory::SdtmProjectServiceImpl;
