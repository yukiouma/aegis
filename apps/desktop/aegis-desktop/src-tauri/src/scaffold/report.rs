//! The report the initialize-project-folder command returns.
//!
//! These types have no server counterpart — they are the desktop's
//! own answer about what happened on disk — so they live here rather
//! than in `http/`, which mirrors the server's wire shapes.
//!
//! The skipped-item type is `scaffolding::SkippedItem`: the plan-time
//! skips the planner produced and the apply-time skips the applier
//! produces are the same vocabulary, so they share one type. Its
//! `camelCase` serde casing is what the TypeScript union mirrors.

use scaffolding::SkippedItem;
use serde::{Deserialize, Serialize};

/// What the command did.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitReportResponse {
    /// The base the tree was written under.
    pub root: String,
    /// The paths that were created.
    pub created: Vec<String>,
    /// The nodes that were not created, with the reason.
    pub skipped: Vec<SkippedItem>,
}
