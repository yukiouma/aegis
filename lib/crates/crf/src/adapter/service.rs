//! Outbound-port adapters. Adapters from the domain ports to
//! the `apis` crate live here. Today this only houses the
//! `ProjectLookup` adapter that bridges
//! `apis::project::ProjectService` to the domain
//! `ProjectLookup`.

pub mod project;
