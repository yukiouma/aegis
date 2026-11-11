//! `scaffolding` workspace crate.
//!
//! Turns a structure template into the list of filesystem operations
//! that materialises it. The crate performs **no** I/O: it resolves
//! `{{ … }}` templates in a node's `entry_name` against a [`Context`]
//! and produces a [`Scaffold`] — the ordered steps an applier would
//! execute, plus the nodes that could not be planned.
//!
//! The applier lives in the desktop app, which owns the filesystem.
//!
//! ```ignore
//! use scaffolding::{Context, scaffold};
//! # fn demo(ctx: Context, node: scaffolding::StructureNode) {
//! let plan = scaffold::build("D:\\studies", &node, &ctx)?;
//! # Ok::<(), scaffolding::ScaffoldError>(())
//! # }
//! ```

pub mod context;
pub mod error;
pub mod report;
pub mod scaffold;
pub mod template;

pub use context::Context;
pub use error::ScaffoldError;
// pub use report::{SkipReason, SkippedItem};
// pub use scaffold::{Migration, Scaffold, Step, StructureNode, build};
pub use template::{Field, Operation, TemplateError, resolve};
