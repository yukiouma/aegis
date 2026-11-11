use serde::{Deserialize, Serialize};

/// Why a node was not created.
///
/// Shared between the planner (which skips a node it cannot resolve)
/// and the applier (which skips a node whose target already exists),
/// so the two never drift apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SkipReason {
    /// The target already exists on disk.
    AlreadyExists,
    /// The `entry_name` referenced a field the context did not have.
    UnresolvedField,
    /// The `entry_name` did not parse as a template.
    MalformedTemplate,
    /// The resolved name was empty or contained a path separator.
    InvalidName,
    /// A file node with no `reference` — nothing to copy.
    NoReference,
    /// The `reference` file could not be read or copied.
    ReferenceFailed,
    /// The `migration.directory` could not be read.
    MigrationFailed,
}

/// One node that was not created, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkippedItem {
    /// The path the node would have had. For a node whose name could
    /// not be resolved this is the parent's path joined with the raw,
    /// unresolved `entry_name` — it is not a real directory.
    pub path: String,
    pub reason: SkipReason,
    /// The missing field name, the raw template text, or the
    /// underlying I/O error message.
    pub detail: Option<String>,
}
