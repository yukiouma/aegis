//! The release-update path.
//!
//! The updater talks to the release server, not to `aegis-server`, so it
//! is deliberately *not* a `http/` module: the policy, the release
//! source and the on-disk artifact cache all live here.
//!
//! - [`status`] — the phase the updater is in, and the shape the
//!   frontend reads it in.
//! - [`error`] — the failure modes of the path.
//! - [`version`] — ordering of announced versions against held ones.

pub mod error;
pub mod notifier;
pub mod service;
pub mod source;
pub mod status;
pub mod version;
