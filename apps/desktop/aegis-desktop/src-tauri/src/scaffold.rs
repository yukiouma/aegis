//! Materialises a structure template onto the desktop machine's disk.
//!
//! The *rules* — context derivation, template resolution, and the
//! tree-to-plan transformation — live in the `scaffolding` crate. This
//! module holds only the filesystem half.
pub mod apply;
pub mod report;
