# `logging-utils` Workspace Crate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the `lib/crates/logging-utils` workspace crate that unifies the tracing bootstrap currently duplicated in `aegis-server` and `aegis-desktop`.

> **Note on iteration:** An earlier draft of this plan included a Task 2 to absorb `lib/crates/trace-id`'s source into `logging-utils`. That work was started, committed, then reverted on user request ("let it be independence for this time"). The plan below reflects the narrower scope — `trace-id` stays untouched.

**Architecture:** New workspace crate with a single `tracing_init` module exposing `LoggingConfig { log_dir, file_name_prefix }` + `init_tracing(&LoggingConfig) -> Result<LogGuard, LoggingInitError>` that both apps can call once they migrate.

**Tech Stack:** Rust 2024 edition, resolver 3, `tracing` + `tracing-subscriber` + `tracing-appender` + `thiserror` (all workspace deps), `tempfile` (dev dep). No `ulid` / `trace-id` dependency in this PR.

## Global Constraints

- `edition = "2024"`, `resolver = "3"` (workspace root, [Cargo.toml:13](Cargo.toml))
- All shared deps declared in `[workspace.dependencies]` and inherited via `{ workspace = true }`
- One-line `why` comment on each non-obvious workspace dep
- No DDD layered structure (this crate is not a business lib; `lib-crate-development.md` does not apply)
- Workspace member names use kebab-case (`logging-utils`)
- No edits to `aegis-server` / `aegis-desktop` / `trace-id` / any other workspace member in this PR
- Crate-level doc-comment in `src/lib.rs` shows the canonical `use` line
- README at the crate root with: one-line purpose, file tree, `LoggingConfig` snippet, verification command

---

### Task 1: Scaffold the new `logging-utils` crate

**Files:**
- Create: `lib/crates/logging-utils/Cargo.toml`
- Create: `lib/crates/logging-utils/README.md` (stub)
- Create: `lib/crates/logging-utils/src/lib.rs` (empty placeholder)
- Modify: `Cargo.toml` (workspace root) — add `"lib/crates/logging-utils"` to `[workspace].members`

**Interfaces:**
- Produces: an empty `logging-utils` crate that resolves via the workspace and compiles cleanly

- [ ] **Step 1: Add `lib/crates/logging-utils` to root `[workspace].members`**

In `Cargo.toml` at the workspace root, add one line to the `members` array, preserving alphabetical-ish order (place it next to `lib/crates/logging-utils` alphabetically — i.e. between `lib/crates/domain-model` and `lib/crates/mission`, or in whatever ordering the existing list uses; mirror the order that puts `apis` before `auth`):

```toml
[workspace]
members = [
    "apps/desktop/aegis-desktop/src-tauri",
    "apps/server/aegis-server",
    "lib/crates/apis",
    "lib/crates/auth", "lib/crates/crf", "lib/crates/domain-model",
    "lib/crates/logging-utils",
    "lib/crates/mission",
    "lib/crates/project",
    "lib/crates/terminology",
    "lib/crates/trace-id",
    "lib/crates/user",
    "lib/crates/windows-utils",
]
```

(The exact ordering follows what existed; the only required change is adding `"lib/crates/logging-utils"`.)

- [ ] **Step 2: Create `lib/crates/logging-utils/Cargo.toml`**

```toml
[package]
name = "logging-utils"
version = "0.1.0"
edition = "2024"

[dependencies]
tracing            = { workspace = true }
tracing-subscriber = { workspace = true }
tracing-appender   = { workspace = true }
thiserror          = { workspace = true }
# `ulid` generates the time-sortable log ids that compose a trace id
# (moved from the `trace-id` crate, now a backwards-compat shim).
ulid               = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
```

- [ ] **Step 3: Create `lib/crates/logging-utils/src/lib.rs` (stub)**

```rust
//! `logging-utils` workspace crate.
//!
//! Unifies the `tracing` bootstrap used by `aegis-server` and
//! `aegis-desktop`, and owns the `TraceIdGenerator` / `Side` types
//! moved from `lib/crates/trace-id`.

// Modules are added in later tasks.
```

- [ ] **Step 4: Create `lib/crates/logging-utils/README.md` (stub — finalized in Task 4)**

```markdown
# `logging-utils`

Unified tracing bootstrap + trace-id generator for `aegis-server` and
`aegis-desktop`. (Full docs land in Task 4.)
```

- [ ] **Step 5: Verify the empty crate compiles via the workspace**

Run from workspace root:
```bash
cargo check --workspace
```
Expected: PASS. The new `logging-utils` member resolves and compiles as an empty crate. No errors about missing deps or duplicate packages.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml lib/crates/logging-utils/
git commit -m "feat(logging-utils): scaffold empty workspace crate

Adds lib/crates/logging-utils to the workspace members with
workspace-inherited deps for tracing, tracing-subscriber,
tracing-appender, thiserror, ulid, and tempfile (dev). The crate
has no modules yet — those land in follow-up tasks."
```

---

### Task 2: Absorb `trace-id` source into `logging-utils` and convert `trace-id` to a shim

> **Status: REVERTED.** This task was executed (commit `57e252f`) and then reverted (commit `4158bbe`) on user request. The `trace-id` crate stays untouched and independent for this PR. The current `logging-utils` crate has no `trace_id` module and no `ulid` dep; see Tasks 3 and 4 for the final implementation.

The original Task 2 content is preserved below for reference.

---

(Original Task 2 content follows — DO NOT execute; kept for context only.)

**Files:**
- Create: `lib/crates/logging-utils/src/trace_id.rs` — full moved content from `lib/crates/trace-id/src/lib.rs`
- Modify: `lib/crates/logging-utils/src/lib.rs` — declare `pub mod trace_id;` + `pub use trace_id::{Side, TraceIdGenerator};`
- Modify: `lib/crates/trace-id/src/lib.rs` — replace contents with `pub use logging_utils::{Side, TraceIdGenerator};`
- Modify: `lib/crates/trace-id/Cargo.toml` — drop `ulid`, add `logging-utils = { path = "../logging-utils" }`

**Interfaces:**
- Consumes: `lib/crates/trace-id/src/lib.rs` (read for source content to move)
- Produces: `logging_utils::{Side, TraceIdGenerator}` (canonical home); `trace_id::{Side, TraceIdGenerator}` (still resolvable via the shim)

- [ ] **Step 1: Read the current `lib/crates/trace-id/src/lib.rs` source**

Read `lib/crates/trace-id/src/lib.rs`. Copy the `Side` enum, `TraceIdGenerator` struct + impl block, and the `#[cfg(test)] mod tests` block (with all six tests: `client_side_starts_with_c`, `server_side_starts_with_s`, `no_device_prefix_omits_middle_segment`, `device_prefix_appears_in_middle`, `each_call_yields_a_fresh_ulid`, `side_enum_variants_are_distinct`) verbatim into the new file in Task 2 Step 2.

- [ ] **Step 2: Create `lib/crates/logging-utils/src/trace_id.rs`**

This file is a verbatim copy of `lib/crates/trace-id/src/lib.rs` (after the move). Update the top-level module doc-comment to mention it has been moved:

```rust
//! Trace ids that identify a single unit of work as it crosses a
//! client ↔ server boundary. A trace id is composed of three optional
//! segments joined by `-`:
//!
//! 1. **Side identifier** — `C` for client, `S` for server.
//! 2. **Device prefix** — caller-supplied free-form label
//!    (e.g. `"desktop"`). Omitted entirely when not provided.
//! 3. **Log id** — a time-sortable ULID.
//!
//! Examples:
//!
//! ```text
//! C-01H9XQ8Z6VK3FJ4P5N2W7Y0T8CB
//! S-desktop-01H9XQ9A2DF4GJ7M5P1R3V6X9BC
//! ```
//!
//! Moved verbatim from `lib/crates/trace-id/src/lib.rs`; the
//! `trace-id` crate now re-exports these types as a backwards-compat
//! shim so existing consumers keep compiling.

use ulid::Ulid;

/// Which side of the wire a trace id was minted on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    /// Mints ids prefixed with `C`.
    Client,
    /// Mints ids prefixed with `S`.
    Server,
}

/// Mints trace ids composed of a side identifier, optional device
/// prefix, and a fresh ULID on every call.
///
/// Construct once per process (or per request-handling task) and
/// reuse — the generator itself is stateless; only `device_prefix`
/// is captured at construction time.
#[derive(Debug, Clone)]
pub struct TraceIdGenerator {
    device_prefix: Option<String>,
}

impl TraceIdGenerator {
    /// Build a generator. `device_prefix` is embedded between the
    /// side identifier and the ULID; pass `None` to omit it.
    pub fn new(device_prefix: Option<String>) -> Self {
        Self { device_prefix }
    }

    /// Mint a client-side trace id (`C-…`).
    pub fn client_side(&self) -> String {
        self.build(Side::Client)
    }

    /// Mint a server-side trace id (`S-…`).
    pub fn server_side(&self) -> String {
        self.build(Side::Server)
    }

    fn build(&self, side: Side) -> String {
        let prefix = match side {
            Side::Client => "C",
            Side::Server => "S",
        };
        let ulid = Ulid::generate().to_string();
        match &self.device_prefix {
            Some(device) => format!("{prefix}-{device}-{ulid}"),
            None => format!("{prefix}-{ulid}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_side_starts_with_c() {
        let generator = TraceIdGenerator::new(None);
        let id = generator.client_side();
        assert!(
            id.starts_with('C'),
            "expected to start with 'C', got {id:?}"
        );
        assert_eq!(
            id.chars().nth(1),
            Some('-'),
            "expected '-' after 'C', got {id:?}"
        );
    }

    #[test]
    fn server_side_starts_with_s() {
        let generator = TraceIdGenerator::new(None);
        let id = generator.server_side();
        assert!(
            id.starts_with('S'),
            "expected to start with 'S', got {id:?}"
        );
        assert_eq!(
            id.chars().nth(1),
            Some('-'),
            "expected '-' after 'S', got {id:?}"
        );
    }

    #[test]
    fn no_device_prefix_omits_middle_segment() {
        let generator = TraceIdGenerator::new(None);
        let id = generator.client_side();
        let parts: Vec<&str> = id.split('-').collect();
        assert_eq!(parts.len(), 2, "expected 2 segments, got {id:?}");
        assert_eq!(parts[0], "C");
    }

    #[test]
    fn device_prefix_appears_in_middle() {
        let generator = TraceIdGenerator::new(Some("desktop".to_string()));
        let id = generator.server_side();
        let parts: Vec<&str> = id.split('-').collect();
        assert_eq!(parts.len(), 3, "expected 3 segments, got {id:?}");
        assert_eq!(parts[0], "S");
        assert_eq!(parts[1], "desktop");
    }

    #[test]
    fn each_call_yields_a_fresh_ulid() {
        let generator = TraceIdGenerator::new(None);
        let a = generator.client_side();
        let b = generator.client_side();
        assert_ne!(a, b);
    }

    #[test]
    fn side_enum_variants_are_distinct() {
        assert_ne!(Side::Client, Side::Server);
    }
}
```

- [ ] **Step 3: Update `lib/crates/logging-utils/src/lib.rs` to expose `trace_id`**

Replace the placeholder `lib/crates/logging-utils/src/lib.rs` with:

```rust
//! `logging-utils` workspace crate.
//!
//! Unifies the `tracing` bootstrap used by `aegis-server` and
//! `aegis-desktop`, and owns the `TraceIdGenerator` / `Side` types
//! moved from `lib/crates/trace-id`.

pub mod trace_id;
pub mod tracing_init;

pub use trace_id::{Side, TraceIdGenerator};
```

(`tracing_init` is declared but not yet defined — that's fine, the empty `pub mod tracing_init;` will fail to compile until Task 3 creates the file. If Task 3 is in the same commit / before the verification step, leave the line out for now and add it in Task 3.)

- [ ] **Step 4: Replace `lib/crates/trace-id/src/lib.rs` with the shim**

Overwrite `lib/crates/trace-id/src/lib.rs` with:

```rust
//! `trace-id` workspace crate — backwards-compat shim.
//!
//! The `TraceIdGenerator` and `Side` types now live in
//! `logging-utils` (their canonical home). This crate re-exports
//! them so existing consumers (`aegis-server`, `aegis-desktop`) keep
//! compiling untouched. Delete this crate once those consumers
//! migrate to `use logging_utils::…` directly.

pub use logging_utils::{Side, TraceIdGenerator};
```

- [ ] **Step 5: Update `lib/crates/trace-id/Cargo.toml`**

Replace the existing `lib/crates/trace-id/Cargo.toml`:

```toml
[package]
name = "trace-id"
version = "0.1.0"
edition = "2024"

[dependencies]
# `logging-utils` is the canonical home for `TraceIdGenerator` /
# `Side`. This crate is a backwards-compat shim that re-exports
# them so existing consumers keep compiling.
logging-utils = { path = "../logging-utils" }
```

(If there were any other entries — there shouldn't be — keep only what's shown.)

- [ ] **Step 6: Verify both crates build and test cleanly**

Run from workspace root:
```bash
cargo check --workspace
cargo test  -p logging-utils
cargo test  -p trace-id
```

Expected: PASS for all three. `logging-utils` has its six trace_id tests passing. `trace-id` shim compiles and re-exports resolve. The rest of the workspace (`aegis-server`, `aegis-desktop`, etc.) still compiles because nothing has changed for them.

- [ ] **Step 7: Commit**

```bash
git add lib/crates/logging-utils/src/trace_id.rs \
        lib/crates/logging-utils/src/lib.rs \
        lib/crates/trace-id/src/lib.rs \
        lib/crates/trace-id/Cargo.toml
git commit -m "feat(logging-utils): absorb trace-id source, shim trace-id crate

Moves the TraceIdGenerator and Side source (and its six-test suite)
from lib/crates/trace-id/src/lib.rs into
lib/crates/logging-utils/src/trace_id.rs. logging-utils now owns
the canonical types.

lib/crates/trace-id is rewritten as a one-line re-export shim so
aegis-server and aegis-desktop keep compiling untouched — their
path-deps and use statements do not change. The shim is deleted in
a follow-up PR once both apps migrate to use logging_utils::*
directly."
```

**Files:**
- Create: `lib/crates/logging-utils/src/trace_id.rs` — full moved content from `lib/crates/trace-id/src/lib.rs`
- Modify: `lib/crates/logging-utils/src/lib.rs` — declare `pub mod trace_id;` + `pub use trace_id::{Side, TraceIdGenerator};`
- Modify: `lib/crates/trace-id/src/lib.rs` — replace contents with `pub use logging_utils::{Side, TraceIdGenerator};`
- Modify: `lib/crates/trace-id/Cargo.toml` — drop `ulid`, add `logging-utils = { path = "../logging-utils" }`

**Interfaces:**
- Consumes: `lib/crates/trace-id/src/lib.rs` (read for source content to move)
- Produces: `logging_utils::{Side, TraceIdGenerator}` (canonical home); `trace_id::{Side, TraceIdGenerator}` (still resolvable via the shim)

- [ ] **Step 1: Read the current `lib/crates/trace-id/src/lib.rs` source**

Read `lib/crates/trace-id/src/lib.rs`. Copy the `Side` enum, `TraceIdGenerator` struct + impl block, and the `#[cfg(test)] mod tests` block (with all six tests: `client_side_starts_with_c`, `server_side_starts_with_s`, `no_device_prefix_omits_middle_segment`, `device_prefix_appears_in_middle`, `each_call_yields_a_fresh_ulid`, `side_enum_variants_are_distinct`) verbatim into the new file in Task 2 Step 2.

- [ ] **Step 2: Create `lib/crates/logging-utils/src/trace_id.rs`**

This file is a verbatim copy of `lib/crates/trace-id/src/lib.rs` (after the move). Update the top-level module doc-comment to mention it has been moved:

```rust
//! Trace ids that identify a single unit of work as it crosses a
//! client ↔ server boundary. A trace id is composed of three optional
//! segments joined by `-`:
//!
//! 1. **Side identifier** — `C` for client, `S` for server.
//! 2. **Device prefix** — caller-supplied free-form label
//!    (e.g. `"desktop"`). Omitted entirely when not provided.
//! 3. **Log id** — a time-sortable ULID.
//!
//! Examples:
//!
//! ```text
//! C-01H9XQ8Z6VK3FJ4P5N2W7Y0T8CB
//! S-desktop-01H9XQ9A2DF4GJ7M5P1R3V6X9BC
//! ```
//!
//! Moved verbatim from `lib/crates/trace-id/src/lib.rs`; the
//! `trace-id` crate now re-exports these types as a backwards-compat
//! shim so existing consumers keep compiling.

use ulid::Ulid;

/// Which side of the wire a trace id was minted on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    /// Mints ids prefixed with `C`.
    Client,
    /// Mints ids prefixed with `S`.
    Server,
}

/// Mints trace ids composed of a side identifier, optional device
/// prefix, and a fresh ULID on every call.
///
/// Construct once per process (or per request-handling task) and
/// reuse — the generator itself is stateless; only `device_prefix`
/// is captured at construction time.
#[derive(Debug, Clone)]
pub struct TraceIdGenerator {
    device_prefix: Option<String>,
}

impl TraceIdGenerator {
    /// Build a generator. `device_prefix` is embedded between the
    /// side identifier and the ULID; pass `None` to omit it.
    pub fn new(device_prefix: Option<String>) -> Self {
        Self { device_prefix }
    }

    /// Mint a client-side trace id (`C-…`).
    pub fn client_side(&self) -> String {
        self.build(Side::Client)
    }

    /// Mint a server-side trace id (`S-…`).
    pub fn server_side(&self) -> String {
        self.build(Side::Server)
    }

    fn build(&self, side: Side) -> String {
        let prefix = match side {
            Side::Client => "C",
            Side::Server => "S",
        };
        let ulid = Ulid::generate().to_string();
        match &self.device_prefix {
            Some(device) => format!("{prefix}-{device}-{ulid}"),
            None => format!("{prefix}-{ulid}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_side_starts_with_c() {
        let generator = TraceIdGenerator::new(None);
        let id = generator.client_side();
        assert!(
            id.starts_with('C'),
            "expected to start with 'C', got {id:?}"
        );
        assert_eq!(
            id.chars().nth(1),
            Some('-'),
            "expected '-' after 'C', got {id:?}"
        );
    }

    #[test]
    fn server_side_starts_with_s() {
        let generator = TraceIdGenerator::new(None);
        let id = generator.server_side();
        assert!(
            id.starts_with('S'),
            "expected to start with 'S', got {id:?}"
        );
        assert_eq!(
            id.chars().nth(1),
            Some('-'),
            "expected '-' after 'S', got {id:?}"
        );
    }

    #[test]
    fn no_device_prefix_omits_middle_segment() {
        let generator = TraceIdGenerator::new(None);
        let id = generator.client_side();
        let parts: Vec<&str> = id.split('-').collect();
        assert_eq!(parts.len(), 2, "expected 2 segments, got {id:?}");
        assert_eq!(parts[0], "C");
    }

    #[test]
    fn device_prefix_appears_in_middle() {
        let generator = TraceIdGenerator::new(Some("desktop".to_string()));
        let id = generator.server_side();
        let parts: Vec<&str> = id.split('-').collect();
        assert_eq!(parts.len(), 3, "expected 3 segments, got {id:?}");
        assert_eq!(parts[0], "S");
        assert_eq!(parts[1], "desktop");
    }

    #[test]
    fn each_call_yields_a_fresh_ulid() {
        let generator = TraceIdGenerator::new(None);
        let a = generator.client_side();
        let b = generator.client_side();
        assert_ne!(a, b);
    }

    #[test]
    fn side_enum_variants_are_distinct() {
        assert_ne!(Side::Client, Side::Server);
    }
}
```

- [ ] **Step 3: Update `lib/crates/logging-utils/src/lib.rs` to expose `trace_id`**

Replace the placeholder `lib/crates/logging-utils/src/lib.rs` with:

```rust
//! `logging-utils` workspace crate.
//!
//! Unifies the `tracing` bootstrap used by `aegis-server` and
//! `aegis-desktop`, and owns the `TraceIdGenerator` / `Side` types
//! moved from `lib/crates/trace-id`.

pub mod trace_id;
pub mod tracing_init;

pub use trace_id::{Side, TraceIdGenerator};
```

(`tracing_init` is declared but not yet defined — that's fine, the empty `pub mod tracing_init;` will fail to compile until Task 3 creates the file. If Task 3 is in the same commit / before the verification step, leave the line out for now and add it in Task 3.)

- [ ] **Step 4: Replace `lib/crates/trace-id/src/lib.rs` with the shim**

Overwrite `lib/crates/trace-id/src/lib.rs` with:

```rust
//! `trace-id` workspace crate — backwards-compat shim.
//!
//! The `TraceIdGenerator` and `Side` types now live in
//! `logging-utils` (their canonical home). This crate re-exports
//! them so existing consumers (`aegis-server`, `aegis-desktop`) keep
//! compiling untouched. Delete this crate once those consumers
//! migrate to `use logging_utils::…` directly.

pub use logging_utils::{Side, TraceIdGenerator};
```

- [ ] **Step 5: Update `lib/crates/trace-id/Cargo.toml`**

Replace the existing `lib/crates/trace-id/Cargo.toml`:

```toml
[package]
name = "trace-id"
version = "0.1.0"
edition = "2024"

[dependencies]
# `logging-utils` is the canonical home for `TraceIdGenerator` /
# `Side`. This crate is a backwards-compat shim that re-exports
# them so existing consumers keep compiling.
logging-utils = { path = "../logging-utils" }
```

(If there were any other entries — there shouldn't be — keep only what's shown.)

- [ ] **Step 6: Verify both crates build and test cleanly**

Run from workspace root:
```bash
cargo check --workspace
cargo test  -p logging-utils
cargo test  -p trace-id
```

Expected: PASS for all three. `logging-utils` has its six trace_id tests passing. `trace-id` shim compiles and re-exports resolve. The rest of the workspace (`aegis-server`, `aegis-desktop`, etc.) still compiles because nothing has changed for them.

- [ ] **Step 7: Commit**

```bash
git add lib/crates/logging-utils/src/trace_id.rs \
        lib/crates/logging-utils/src/lib.rs \
        lib/crates/trace-id/src/lib.rs \
        lib/crates/trace-id/Cargo.toml
git commit -m "feat(logging-utils): absorb trace-id source, shim trace-id crate

Moves the TraceIdGenerator and Side source (and its six-test suite)
from lib/crates/trace-id/src/lib.rs into
lib/crates/logging-utils/src/trace_id.rs. logging-utils now owns
the canonical types.

lib/crates/trace-id is rewritten as a one-line re-export shim so
aegis-server and aegis-desktop keep compiling untouched — their
path-deps and use statements do not change. The shim is deleted in
a follow-up PR once both apps migrate to use logging_utils::*
directly."
```

---

### Task 3: Implement `tracing_init.rs` (TDD)

**Files:**
- Create: `lib/crates/logging-utils/src/tracing_init.rs` — LoggingConfig, LogGuard, LoggingInitError, build_filter, init_tracing + tests

**Interfaces:**
- Produces:
  - `pub struct LoggingConfig { pub log_dir: PathBuf, pub file_name_prefix: String }`
  - `pub struct LogGuard(pub WorkerGuard)`
  - `pub enum LoggingInitError { CreateDir { dir: PathBuf, #[source] source: io::Error } }`
  - `pub fn init_tracing(config: &LoggingConfig) -> Result<LogGuard, LoggingInitError>`
  - `pub fn build_filter() -> EnvFilter`
- Consumes: `tracing_appender::rolling::daily`, `tracing_appender::non_blocking`, `tracing_subscriber::fmt`, `std::env::var("AEGIS_LOG_LEVEL")`

Follow TDD order: write the failing tests, run them (expect failure), implement, run again (expect pass). The tests live in `#[cfg(test)] mod tests` at the bottom of `tracing_init.rs`. The ENV_LOCK / EnvGuard pattern is taken from `apps/desktop/aegis-desktop/src-tauri/src/tracing_init.rs` and `apps/server/aegis-server/src/run.rs` — copy the structure but adapt for the new module.

- [ ] **Step 1: Write the test module first (failing tests)**

Create `lib/crates/logging-utils/src/tracing_init.rs` with the test module and empty stub impls so it compiles but every test fails:

```rust
//! Tracing bootstrap. See module-level docs at the crate root.

use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Error)]
pub enum LoggingInitError {
    #[error("failed to create log directory {dir}: {source}")]
    CreateDir {
        dir: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Debug)]
pub struct LogGuard(pub WorkerGuard);

#[derive(Debug, Clone)]
pub struct LoggingConfig {
    pub log_dir: PathBuf,
    pub file_name_prefix: String,
}

// ---- Stubs that satisfy the type checker but are not yet implemented. ----

pub fn init_tracing(_config: &LoggingConfig) -> Result<LogGuard, LoggingInitError> {
    unimplemented!("init_tracing not yet implemented")
}

pub fn build_filter() -> EnvFilter {
    unimplemented!("build_filter not yet implemented")
}

// ---- Tests ----

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, MutexGuard};

    static ENV_LOCK: Mutex<()> = Mutex::new(());
    fn lock_env() -> MutexGuard<'static, ()> {
        ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }
    struct EnvGuard {
        key: &'static str,
        prev: Option<String>,
    }
    impl Drop for EnvGuard {
        fn drop(&mut self) {
            // SAFETY: env vars are process-global; ENV_LOCK serializes.
            unsafe {
                match &self.prev {
                    Some(v) => std::env::set_var(self.key, v),
                    None => std::env::remove_var(self.key),
                }
            }
        }
    }
    fn set_env(key: &'static str, value: &str) -> EnvGuard {
        let prev = std::env::var(key).ok();
        // SAFETY: serialized via ENV_LOCK.
        unsafe {
            std::env::set_var(key, value);
        }
        EnvGuard { key, prev }
    }

    fn cfg(dir: &Path, prefix: &str) -> LoggingConfig {
        LoggingConfig {
            log_dir: dir.to_path_buf(),
            file_name_prefix: prefix.to_string(),
        }
    }

    #[test]
    fn init_tracing_creates_log_dir_when_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let log_dir = tmp.path().join("aegis-subdir");
        assert!(!log_dir.exists());
        let _g = lock_env();
        let _lvl = set_env("AEGIS_LOG_LEVEL", "info");
        let guard = init_tracing(&cfg(&log_dir, "aegis-test.log"))
            .expect("init_tracing succeeds");
        assert!(log_dir.is_dir(), "log dir should be created");
        let _ = guard;
    }

    #[test]
    fn init_tracing_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let _g = lock_env();
        let _lvl = set_env("AEGIS_LOG_LEVEL", "info");
        let _a = init_tracing(&cfg(tmp.path(), "aegis-test.log"))
            .expect("first init");
        let _b = init_tracing(&cfg(tmp.path(), "aegis-test.log"))
            .expect("second init");
    }

    #[test]
    fn init_tracing_propagates_create_dir_failure() {
        let tmp = tempfile::tempdir().unwrap();
        let blocker = tmp.path().join("blocker");
        std::fs::write(&blocker, b"not a dir").unwrap();
        let bad = blocker.join("inside");
        let err = init_tracing(&cfg(&bad, "aegis-test.log")).unwrap_err();
        match err {
            LoggingInitError::CreateDir { dir, .. } => assert_eq!(dir, bad),
        }
    }

    #[test]
    fn init_tracing_writes_to_configured_file_name_prefix() {
        let tmp = tempfile::tempdir().unwrap();
        let _g = lock_env();
        let _lvl = set_env("AEGIS_LOG_LEVEL", "info");
        let guard = init_tracing(&cfg(tmp.path(), "aegis-write-test.log"))
            .expect("init_tracing succeeds");
        tracing::info!("hello from test");
        drop(guard);
        let entries: Vec<_> = std::fs::read_dir(tmp.path()).unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with("aegis-write-test.log")
            })
            .collect();
        assert!(
            !entries.is_empty(),
            "expected at least one daily-rotate file named with the configured prefix"
        );
    }

    #[test]
    fn build_filter_defaults_to_info_when_env_missing() {
        let _g = lock_env();
        unsafe {
            std::env::remove_var("AEGIS_LOG_LEVEL");
        }
        let filter = build_filter();
        assert_eq!(filter.to_string(), "info");
    }

    #[test]
    fn build_filter_uses_aegis_log_level_when_set() {
        let _g = lock_env();
        let _lvl = set_env("AEGIS_LOG_LEVEL", "debug");
        let filter = build_filter();
        assert_eq!(filter.to_string(), "debug");
    }

    #[test]
    fn logging_config_clone_preserves_fields() {
        let original = LoggingConfig {
            log_dir: PathBuf::from("/tmp/aegis"),
            file_name_prefix: "aegis-clone-test.log".to_string(),
        };
        let cloned = original.clone();
        assert_eq!(original.log_dir, cloned.log_dir);
        assert_eq!(original.file_name_prefix, cloned.file_name_prefix);
    }
}
```

- [ ] **Step 2: Run the tests and verify they fail**

Run from workspace root:
```bash
cargo test -p logging-utils --lib tracing_init
```

Expected: FAIL on every `init_tracing` / `build_filter` test with "not yet implemented" panics. The `logging_config_clone_preserves_fields` test should PASS (it's a derive smoke test that doesn't touch the stubs).

- [ ] **Step 3: Implement `build_filter` and `init_tracing`**

In `lib/crates/logging-utils/src/tracing_init.rs`, replace the two `unimplemented!` stubs with the real implementations:

```rust
/// Build the global `EnvFilter` from `AEGIS_LOG_LEVEL`. Defaults to
/// `info` when the variable is unset.
pub fn build_filter() -> EnvFilter {
    let level = std::env::var("AEGIS_LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
    EnvFilter::new(level)
}

/// Install the global JSON `tracing` subscriber writing to
/// `{log_dir}/{file_name_prefix}.YYYY-MM-DD` (one file per day,
/// rotation at local midnight). The `EnvFilter` is built from
/// `AEGIS_LOG_LEVEL` (default `info`).
///
/// The returned [`LogGuard`] MUST be held for the lifetime of the
/// program; dropping it flushes the buffered writer. `try_init`
/// semantics make a re-entry from tests a no-op.
pub fn init_tracing(config: &LoggingConfig) -> Result<LogGuard, LoggingInitError> {
    std::fs::create_dir_all(&config.log_dir).map_err(|source| LoggingInitError::CreateDir {
        dir: config.log_dir.clone(),
        source,
    })?;
    let file_appender =
        tracing_appender::rolling::daily(&config.log_dir, &config.file_name_prefix);
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    let _ = tracing_subscriber::fmt()
        .with_env_filter(build_filter())
        .json()
        .with_current_span(true)
        .with_span_list(false)
        .with_writer(non_blocking)
        .try_init();
    Ok(LogGuard(guard))
}
```

- [ ] **Step 4: Re-run the tests and verify they pass**

Run from workspace root:
```bash
cargo test -p logging-utils
```

Expected: PASS for every test in the new crate (the six `trace_id` tests + the seven new `tracing_init` tests = 13 total).

- [ ] **Step 5: Add the `tracing_init` re-export to `lib.rs`**

In `lib/crates/logging-utils/src/lib.rs`, the `pub mod tracing_init;` line is already declared from Task 2. Add the flat re-exports alongside the existing `pub use trace_id::{Side, TraceIdGenerator};` line:

```rust
//! `logging-utils` workspace crate.
//!
//! Unifies the `tracing` bootstrap used by `aegis-server` and
//! `aegis-desktop`, and owns the `TraceIdGenerator` / `Side` types
//! moved from `lib/crates/trace-id`.
//!
//! ```ignore
//! use logging_utils::{LoggingConfig, init_tracing};
//!
//! let _guard = init_tracing(&LoggingConfig {
//!     log_dir: "./logs".into(),
//!     file_name_prefix: "aegis-server.log".into(),
//! })?;
//! # Ok::<(), logging_utils::LoggingInitError>(())
//! ```

pub mod trace_id;
pub mod tracing_init;

pub use trace_id::{Side, TraceIdGenerator};
pub use tracing_init::{
    build_filter, init_tracing, LogGuard, LoggingConfig, LoggingInitError,
};
```

- [ ] **Step 6: Verify the full workspace still compiles**

```bash
cargo check --workspace
```

Expected: PASS. No edits to `aegis-server` or `aegis-desktop`; the `trace-id` shim still resolves; `logging-utils` builds.

- [ ] **Step 7: Commit**

```bash
git add lib/crates/logging-utils/src/tracing_init.rs \
        lib/crates/logging-utils/src/lib.rs
git commit -m "feat(logging-utils): add tracing_init module

Implements the unified tracing bootstrap that aegis-server and
aegis-desktop will call once they migrate. LoggingConfig is a
two-field struct (log_dir, file_name_prefix) so each app's call
site is a five-line block; init_tracing wires tracing-appender's
daily-rotate, JSON format, EnvFilter from AEGIS_LOG_LEVEL, and
the try_init idempotency both apps already rely on.

Includes seven unit tests covering log-dir creation, idempotent
re-init, create-dir failure propagation, the configured file
name prefix landing on disk, EnvFilter default + override, and
the LoggingConfig Clone derive."
```

---

### Task 4: Finalize README and run the full verification gate

**Files:**
- Modify: `lib/crates/logging-utils/README.md` — replace the Task 1 stub with the real README
- (No code changes — verification only.)

- [ ] **Step 1: Replace the README stub**

Overwrite `lib/crates/logging-utils/README.md` with:

````markdown
# `logging-utils`

Unified tracing bootstrap + trace-id generator for `aegis-server` and
`aegis-desktop`. Centralises the JSON daily-rotating file appender, the
`AEGIS_LOG_LEVEL` env filter, and the `C-` / `S-` trace id format so
neither app has to re-implement it.

## Modules

- `logging_utils::tracing_init` — `LoggingConfig`, `init_tracing`,
  `LogGuard`, `LoggingInitError`, `build_filter`.
- `logging_utils::trace_id` — `Side`, `TraceIdGenerator`
  (moved from `lib/crates/trace-id`, which is now a backwards-compat
  re-export shim).

## Usage

```rust
use logging_utils::{LoggingConfig, init_tracing};

// Server:
let dir = std::env::var("AEGIS_LOG_DIR").unwrap_or_else(|_| "./logs".into());
let _guard = init_tracing(&LoggingConfig {
    log_dir: dir.into(),
    file_name_prefix: "aegis-server.log".into(),
})?;

// Desktop:
let _guard = init_tracing(&LoggingConfig {
    log_dir: log_dir.into(),
    file_name_prefix: "aegis-desktop.log".into(),
})?;
```

`LogGuard` MUST be held for the lifetime of the program (drop it and
buffered writes are lost). `init_tracing` is idempotent — a second
call is a no-op.

## Trace ids

```rust
use logging_utils::{Side, TraceIdGenerator};

let generator = TraceIdGenerator::new(Some("desktop".to_string()));
let client_id = generator.client_side();   // e.g. "C-desktop-01H…"
let server_id = generator.server_side();   // e.g. "S-desktop-01H…"
```

Construct the generator once per process and reuse it; the generator
is stateless apart from `device_prefix`.

## Verification

```bash
cargo test  -p logging-utils
cargo test  -p trace-id       # the backwards-compat shim
cargo clippy -p logging-utils --all-targets --all-features -- -D warnings
cargo clippy -p trace-id      --all-targets --all-features -- -D warnings
cargo doc   -p logging-utils --no-deps
cargo check --workspace
```
````

- [ ] **Step 2: Run the full verification gate**

Run from workspace root:

```bash
cargo fmt --all -- --check
cargo clippy -p logging-utils --all-targets --all-features -- -D warnings
cargo clippy -p trace-id      --all-targets --all-features -- -D warnings
cargo test  -p logging-utils
cargo test  -p trace-id
cargo doc   -p logging-utils --no-deps
cargo check --workspace
```

Expected: PASS for every command. If `cargo fmt --check` reports diffs, run `cargo fmt --all` once and re-run.

- [ ] **Step 3: Final commit**

```bash
git add lib/crates/logging-utils/README.md
git commit -m "docs(logging-utils): full README

Documents the two-module layout, the LoggingConfig call-site shape
both apps will use once they migrate, the trace-id usage snippet,
and the verification commands.

Also closes out the crate build by running the full verification
gate (cargo fmt / clippy / test / doc / workspace check)."
```

---

## Self-Review Notes

- **Spec coverage:**
  - `LoggingConfig` + `LogGuard` + `LoggingInitError` + `init_tracing` + `build_filter` → Task 3
  - `Side` + `TraceIdGenerator` moved into the new crate → Task 2
  - `trace-id` shim → Task 2
  - Workspace member wiring → Task 1
  - `Cargo.toml` workspace dep usage + non-obvious-dep `why` comments → Tasks 1, 2, 3
  - Tests (7 for tracing_init, 6 for trace_id carried over) → Tasks 2, 3
  - README at the crate root → Task 4
  - Verification gate → Task 4
  - No edits to `aegis-server` or `aegis-desktop` → confirmed across all tasks
  - Out-of-scope items (HTTP trace helpers, desktop device-prefix) → not introduced
- **Type consistency:** `LoggingConfig { log_dir: PathBuf, file_name_prefix: String }` is consistent across Task 1 (Cargo.toml dep), Task 3 (struct def + tests), Task 4 (README snippet). `LoggingInitError::CreateDir { dir, source }` is consistent.
- **Placeholder scan:** No "TBD", "TODO", "implement later", or vague "add appropriate handling" steps. Every code step includes the actual code.
- **One ambiguity caught during review:** Task 2 Step 3 leaves `pub mod tracing_init;` declared before the module exists. The bracketed note flags this so the engineer either skips the line at Step 3 and adds it in Task 3 Step 5, or holds off on running `cargo check` between Tasks 2 and 3.
