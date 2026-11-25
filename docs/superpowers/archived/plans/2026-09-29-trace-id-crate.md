# `trace-id` Crate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a new `trace-id` workspace crate that generates trace IDs composed of a side identifier (`C`/`S`), an optional device prefix, and a ULID, joined by `-`.

**Architecture:** Single small utility crate under `lib/crates/trace-id/`. No DDD layering (explicitly exempted by the user — no business logic). One public type `TraceIdGenerator` plus the `Side` enum that the two constructor-named methods return. ULID generation is delegated to the `ulid` crate, added as a workspace dep so any future caller can pin the same version.

**Tech Stack:** Rust 2024 edition; `ulid = "1.2"` (workspace dep).

## Global Constraints

- Crate name: `trace-id` (Cargo package name with hyphen).
- `TraceIdGenerator::new(device_prefix: Option<String>)` stores `device_prefix` privately.
- `TraceIdGenerator::client_side()` returns a trace id starting with `C`.
- `TraceIdGenerator::server_side()` returns a trace id starting with `S`.
- Trace id format: `<side>-<device_prefix?>-<ulid>` where `device_prefix` is omitted (no extra dash) when `None`. A trace id with `device_prefix = None` looks like `C-01H...`; with `device_prefix = Some("desktop")` it looks like `C-desktop-01H...`.
- `ulid` is pinned in `[workspace.dependencies]` once; the crate inherits it via `{ workspace = true }`.
- The crate is not a business lib crate — no `domain` / `usecase` / `adapter` layers, no DTOs, no `FromRow`, no migrations.
- Public surface is just `TraceIdGenerator` (and `Side` if exposed); re-exported at the crate root.
- Tests live under `#[cfg(test)] mod tests` inside `src/lib.rs` — this is a tiny utility, not a layered module. A `tests/public_api.rs` file pins the documented import.

---

## File Structure

```text
Cargo.toml                                # modify: add `trace-id` to workspace.members, add `ulid` to workspace.dependencies
lib/crates/trace-id/
  Cargo.toml                              # new: name + edition + ulid dep
  src/
    lib.rs                                # new: TraceIdGenerator + Side + tests
  tests/
    public_api.rs                         # new: compile-only API smoke test
  README.md                               # new: one-paragraph purpose + usage
```

---

## Task 1: Pin `ulid` in the workspace and register the new crate

**Files:**
- Modify: `Cargo.toml` (workspace root)
- Create: `lib/crates/trace-id/Cargo.toml`

**Interfaces:**
- Consumes: nothing.
- Produces: `ulid` available as `{ workspace = true }`; `trace-id` recognised as a workspace member.

- [ ] **Step 1: Add `ulid` to `[workspace.dependencies]`**

In the root `Cargo.toml`, append to the `[workspace.dependencies]` block (keep alphabetical-ish ordering — after `tower-http`):

```toml
# `ulid` generates the time-sortable log ids that compose a trace
# id. Pinned in the workspace so the `trace-id` crate and any
# downstream consumer resolve the same version.
ulid = "1.2"
```

- [ ] **Step 2: Add `trace-id` to `[workspace].members`**

In the root `Cargo.toml`, edit `members` to add `"lib/crates/trace-id"` after `"lib/crates/terminology"` (keep alphabetical):

```toml
members = [
    "apps/desktop/aegis-desktop/src-tauri",
    "apps/server/aegis-server",
    "lib/crates/apis",
    "lib/crates/auth", "lib/crates/crf", "lib/crates/domain-model", "lib/crates/mission",
    "lib/crates/project",
    "lib/crates/terminology",
    "lib/crates/trace-id",
    "lib/crates/user",
    "lib/crates/windows-utils",
]
```

- [ ] **Step 3: Create `lib/crates/trace-id/Cargo.toml`**

```toml
[package]
name = "trace-id"
version = "0.1.0"
edition = "2024"

[dependencies]
ulid = { workspace = true }
```

- [ ] **Step 4: Verify the workspace resolves**

Run: `cargo check -p trace-id`
Expected: PASS (a missing `src/lib.rs` would error here, so this is also a smoke test for the next task).

---

## Task 2: Implement `TraceIdGenerator` and `Side`

**Files:**
- Create: `lib/crates/trace-id/src/lib.rs`

**Interfaces:**
- `pub enum Side { Client, Server }` — used in test assertions for the side identifier.
- `pub struct TraceIdGenerator { device_prefix: Option<String> }` — `new(Option<String>) -> Self`; `client_side() -> String`; `server_side() -> String`.
- A trace id is `{C|S}-{device_prefix?}-{ulid}` where the middle dash-segment only appears when `device_prefix` is `Some`.

- [ ] **Step 1: Write the failing unit tests**

In `src/lib.rs`, place the tests after the implementation but write them first. Tests (they will fail until the impl is in place):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_side_starts_with_c() {
        let gen = TraceIdGenerator::new(None);
        let id = gen.client_side();
        assert!(id.starts_with('C'), "expected to start with 'C', got {id:?}");
        assert_eq!(id.chars().nth(1), Some('-'), "expected '-' after 'C', got {id:?}");
    }

    #[test]
    fn server_side_starts_with_s() {
        let gen = TraceIdGenerator::new(None);
        let id = gen.server_side();
        assert!(id.starts_with('S'), "expected to start with 'S', got {id:?}");
        assert_eq!(id.chars().nth(1), Some('-'), "expected '-' after 'S', got {id:?}");
    }

    #[test]
    fn no_device_prefix_omits_middle_segment() {
        let gen = TraceIdGenerator::new(None);
        let id = gen.client_side();
        // Format is `C-<ulid>` -> exactly two dash-separated segments.
        let parts: Vec<&str> = id.split('-').collect();
        assert_eq!(parts.len(), 2, "expected 2 segments, got {id:?}");
        assert_eq!(parts[0], "C");
    }

    #[test]
    fn device_prefix_appears_in_middle() {
        let gen = TraceIdGenerator::new(Some("desktop".to_string()));
        let id = gen.server_side();
        let parts: Vec<&str> = id.split('-').collect();
        assert_eq!(parts.len(), 3, "expected 3 segments, got {id:?}");
        assert_eq!(parts[0], "S");
        assert_eq!(parts[1], "desktop");
    }

    #[test]
    fn each_call_yields_a_fresh_ulid() {
        let gen = TraceIdGenerator::new(None);
        let a = gen.client_side();
        let b = gen.client_side();
        assert_ne!(a, b);
    }

    #[test]
    fn side_enum_variants_are_distinct() {
        assert_ne!(Side::Client, Side::Server);
    }
}
```

- [ ] **Step 2: Run the tests to confirm they fail**

Run: `cargo test -p trace-id`
Expected: FAIL — `TraceIdGenerator` and `Side` are not defined yet.

- [ ] **Step 3: Write the implementation**

Replace the body of `src/lib.rs` with:

```rust
//! `trace-id` workspace crate.
//!
//! Generates trace ids that identify a single unit of work as it
//! crosses a client ↔ server boundary. A trace id is composed of
//! three optional segments joined by `-`:
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
//! The same generator instance is reused across the lifetime of a
//! process; callers do not need to recreate it per call.

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
        let ulid = Ulid::new().to_string();
        match &self.device_prefix {
            Some(device) => format!("{prefix}-{device}-{ulid}"),
            None => format!("{prefix}-{ulid}"),
        }
    }
}

#[cfg(test)]
mod tests {
    // (test body from Step 1)
}
```

(Paste the test body from Step 1 into the `mod tests { … }` block.)

- [ ] **Step 4: Run the tests to confirm they pass**

Run: `cargo test -p trace-id`
Expected: PASS — all 6 tests green.

---

## Task 6: Add a `public_api` compile-test and a README

**Files:**
- Create: `lib/crates/trace-id/tests/public_api.rs`
- Create: `lib/crates/trace-id/README.md`

- [ ] **Step 1: Write `tests/public_api.rs`**

```rust
//! Public-API compile test for the `trace-id` crate.
//!
//! Locks the documented import path and the public surface so a
//! regression in `src/lib.rs` is caught at `cargo test -p
//! trace-id` time.

use trace_id::{Side, TraceIdGenerator};

/// Every public type in `trace_id` is nameable from the test.
#[test]
fn public_types_are_nameable() {
    fn assert_gen(_: TraceIdGenerator) {}
    fn assert_side(_: Side) {}

    // `TraceIdGenerator` is constructible with and without a prefix.
    assert_gen(TraceIdGenerator::new(None));
    assert_gen(TraceIdGenerator::new(Some("desktop".to_string())));

    // Touch both `Side` variants.
    let _: Side = Side::Client;
    let _: Side = Side::Server;
    let _ = assert_gen;
    let _ = assert_side;
}

/// Calling both methods on the same instance returns distinct
/// ids prefixed with the expected side identifier.
#[test]
fn both_sides_produce_prefixed_ids() {
    let gen = TraceIdGenerator::new(Some("host".to_string()));
    assert!(gen.client_side().starts_with("C-host-"));
    assert!(gen.server_side().starts_with("S-host-"));
}
```

- [ ] **Step 2: Write `README.md`**

```markdown
# `trace-id`

Generates trace ids that identify a single unit of work as it crosses a
client ↔ server boundary. A trace id has three segments joined by `-`:

1. Side identifier — `C` (client) or `S` (server).
2. Device prefix — caller-supplied label (e.g. `"desktop"`); omitted when
   not provided.
3. Log id — a time-sortable ULID.

## Usage

```rust
use trace_id::TraceIdGenerator;

let gen = TraceIdGenerator::new(Some("desktop".to_string()));
let client_id = gen.client_side();   // e.g. "C-desktop-01H9XQ8Z6VK3FJ4P5N2W7Y0T8CB"
let server_id = gen.server_side();   // e.g. "S-desktop-01H9XQ9A2DF4GJ7M5P1R3V6X9BC"
```

Construct a generator once per process or per request-handling task and
reuse it — the generator is stateless.

## Verification

```bash
cargo test  -p trace-id
cargo clippy -p trace-id --all-targets --all-features -- -D warnings
cargo fmt   --all -- --check
```
```

- [ ] **Step 3: Run the full crate test suite**

Run: `cargo test -p trace-id`
Expected: PASS — 6 unit tests + 2 integration tests green.

---

## Task 7: Final verification gate

- [ ] **Step 1: Run the full workspace check**

Run: `cargo check --workspace`
Expected: PASS.

- [ ] **Step 2: Run clippy on the new crate**

Run: `cargo clippy -p trace-id --all-targets --all-features -- -D warnings`
Expected: PASS, no warnings.

- [ ] **Step 3: Run rustfmt check**

Run: `cargo fmt --all -- --check`
Expected: no diff.

- [ ] **Step 4: Run the new crate's full test suite**

Run: `cargo test -p trace-id`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml lib/crates/trace-id
git commit -m "feat(trace-id): add trace-id crate with TraceIdGenerator"
```

The commit message lists the spec coverage (TraceIdGenerator + client_side
+ server_side + optional device prefix + ULID log id) and the verification
commands at the bottom.