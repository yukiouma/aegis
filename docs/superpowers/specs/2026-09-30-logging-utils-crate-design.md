# `logging-utils` Workspace Crate — Design

**Date:** 2026-09-30
**Scope:** Add `lib/crates/logging-utils`, a workspace crate that unifies the tracing bootstrap currently duplicated in `aegis-server` and `aegis-desktop`, **and carries its own copy of the `TraceIdGenerator` / `Side` logic from `lib/crates/trace-id`** so the logging crate is self-contained. The `trace-id` crate stays untouched and independent — it does NOT depend on `logging-utils` and is NOT a shim. Both crates carry the same logic independently.

> **Note on iteration:** An earlier draft proposed rewriting `trace-id` as a backwards-compat shim of `logging-utils`. That was rejected in favour of true independence — both crates carry the logic on their own.

## Motivation

The two runnable surfaces each ship a near-identical `init_tracing`:

- [`apps/server/aegis-server/src/run.rs:130-148`](apps/server/aegis-server/src/run.rs#L130-L148) — reads `AEGIS_LOG_DIR`, daily-rotates `aegis-server.log.YYYY-MM-DD`, JSON, `AEGIS_LOG_LEVEL` filter.
- [`apps/desktop/aegis-desktop/src-tauri/src/tracing_init.rs:43-58`](apps/desktop/aegis-desktop/src-tauri/src/tracing_init.rs#L43-L58) — takes `log_dir`, daily-rotates `aegis-desktop.log.YYYY-MM-DD`, JSON, `AEGIS_LOG_LEVEL` filter.

The only differences are the dir source (env-var vs parameter) and the file-name prefix. Everything else — JSON format, daily rotation, `EnvFilter`, `try_init` semantics, `LogGuard` ownership — is duplicated. Any future change (e.g. structured fields, OpenTelemetry export, async-flush support) has to be applied twice.

This PR adds the crate. It does NOT touch the two apps — they keep their own copies until a follow-up PR migrates each call site. The new crate is built, tested, documented, and ready to be consumed.

`lib/crates/trace-id` is **out of scope** for this PR — it stays exactly as it is today. Future PRs may choose to fold its types (and server-side `TraceIdMakeSpan` / desktop device-prefix helpers) into `logging-utils`; this PR leaves room for them but does not introduce them.

## Approach

- Add `lib/crates/logging-utils` to the workspace.
- Provide `LoggingConfig { log_dir, file_name_prefix }` + `init_tracing(&LoggingConfig) -> Result<LogGuard, LoggingInitError>` so each app's call site is a five-line block.
- Carry a self-contained copy of `TraceIdGenerator` / `Side` logic inside `logging-utils::trace_id`. The `trace-id` crate keeps its own copy unchanged; the two crates do not depend on each other (no shim relationship).
- Keep the crate small: no HTTP, no Tauri, no business types. The `tracing_init` + `trace_id` modules are enough for now.

## Architecture

```
lib/crates/logging-utils/
├── Cargo.toml
├── README.md
└── src/
    ├── lib.rs           # crate-level doc + pub use surface
    ├── tracing_init.rs  # LoggingConfig, LogGuard, init_tracing, build_filter, LoggingInitError
    └── trace_id.rs      # TraceIdGenerator, Side (carried independently — see note)
```

No DDD layers — this crate is observability infrastructure, not a business lib. The `lib-crate-development.md` guideline does not apply.

> **Why two crates carry the same logic?** `lib/crates/trace-id` is depended on directly by `aegis-server` and `aegis-desktop` today. Keeping it as a standalone crate preserves those path-deps and keeps the existing crate list clean. `logging-utils` carries its own copy so it is self-contained — the logging utilities don't have to reach into a sibling crate. If a future PR decides to delete the `trace-id` crate, the two call sites can be flipped to depend on `logging-utils::trace_id` in one mechanical edit.

## Components

### 1. `src/tracing_init.rs`

Owns the tracing bootstrap. Mirrors the existing desktop `init_tracing` and server `init_tracing` in behaviour; differs only in taking a `LoggingConfig` parameter instead of resolving the dir itself.

```rust
use std::path::PathBuf;
use thiserror::Error;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;

/// Failure modes for `init_tracing`. Today only directory creation
/// can fail; the `try_init` path swallows the "already initialized"
/// error from the global subscriber.
#[derive(Debug, Error)]
pub enum LoggingInitError {
    #[error("failed to create log directory {dir}: {source}")]
    CreateDir {
        dir: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// Owns the `WorkerGuard` returned by `tracing_appender::non_blocking`
/// so the caller can stash it in managed state or in a `let _guard = …;`
/// and the guard lives for the process lifetime.
#[derive(Debug)]
pub struct LogGuard(pub WorkerGuard);

/// Knobs for [`init_tracing`]. Both fields are required; the caller
/// resolves the directory (env var, `AppHandle::path().app_data_dir()`,
/// test tempdir) and supplies the file-name prefix (`"aegis-server.log"`
/// / `"aegis-desktop.log"` / etc.).
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    /// Directory the daily-rotated log file lives under. Created with
    /// `create_dir_all` if missing — failure returns
    /// [`LoggingInitError::CreateDir`].
    pub log_dir: PathBuf,
    /// Prefix handed to `tracing_appender::rolling::daily`. The lib
    /// produces `{file_name_prefix}.YYYY-MM-DD`.
    pub file_name_prefix: String,
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

/// Build the global `EnvFilter` from `AEGIS_LOG_LEVEL`. Defaults to
/// `info` when the variable is unset. Exposed for callers that want
/// the same filter semantics without installing the subscriber.
pub fn build_filter() -> EnvFilter {
    let level = std::env::var("AEGIS_LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
    EnvFilter::new(level)
}
```

### 2. `src/lib.rs`

```rust
//! `logging-utils` workspace crate.
//!
//! Unifies the `tracing` bootstrap used by `aegis-server` and
//! `aegis-desktop`. Consumers build a [`LoggingConfig`] and call
//! [`init_tracing`]; the returned [`LogGuard`] must be held for the
//! lifetime of the program.
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

pub mod tracing_init;

pub use tracing_init::{build_filter, init_tracing, LogGuard, LoggingConfig, LoggingInitError};
```

## Workspace wiring

### Root `Cargo.toml`

```toml
[workspace]
members = [
    # … existing entries …
    "lib/crates/logging-utils",
    # … existing entries …
]
```

### `lib/crates/logging-utils/Cargo.toml`

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
# (mirrored from the `trace-id` crate — the two crates carry the
# same logic independently, not via a dep).
ulid               = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
```

### `lib/crates/logging-utils/src/trace_id.rs` (new — independent copy)

A self-contained copy of the `TraceIdGenerator` / `Side` source from `lib/crates/trace-id/src/lib.rs`. The file's top-level doc-comment is updated to make the duplication explicit. Both crates carry the same logic; they do not depend on each other.

```rust
//! Trace ids that identify a single unit of work as it crosses a
//! client ↔ server boundary.
//!
//! This module is an independent copy of the same logic that lives in
//! `lib/crates/trace-id`. The two crates do NOT depend on each other
//! — `logging-utils` keeps its own copy so the logging utilities are
//! self-contained, and `trace-id` continues to exist as a standalone
//! workspace crate for `aegis-server` and `aegis-desktop` to depend on
//! directly. A future PR may consolidate them.

use ulid::Ulid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    Client,
    Server,
}

#[derive(Debug, Clone)]
pub struct TraceIdGenerator { device_prefix: Option<String> }

impl TraceIdGenerator {
    pub fn new(device_prefix: Option<String>) -> Self { /* … */ }
    pub fn client_side(&self) -> String { /* … */ }
    pub fn server_side(&self) -> String { /* … */ }
    fn build(&self, side: Side) -> String { /* … */ }
}
```

### `lib/crates/trace-id` — independent, untouched

The `trace-id` crate is NOT modified by this PR. It stays exactly as it is today (full source + `ulid` dep), and `aegis-server` / `aegis-desktop` continue to depend on it directly. The `logging-utils` crate carries its own copy of the same logic — both crates are independent.

## Public API at a glance

| Symbol | Kind | Notes |
|---|---|---|
| `LoggingConfig` | struct | `{ log_dir: PathBuf, file_name_prefix: String }` |
| `LogGuard` | newtype | `pub LogGuard(pub WorkerGuard)` |
| `LoggingInitError` | enum | `CreateDir { dir, source }` only |
| `init_tracing(&LoggingConfig) -> Result<LogGuard, LoggingInitError>` | fn | Idempotent via `try_init` |
| `build_filter() -> EnvFilter` | fn | `AEGIS_LOG_LEVEL` → `EnvFilter`, default `info` |
| `Side`, `TraceIdGenerator` | types (mirrored from `trace-id`) | Independent copy; `trace-id` carries its own too |

## Testing

Port and adapt the union of the two existing init test suites. Env-var tests use the same `ENV_LOCK` + `EnvGuard` pattern already in both apps' tracing modules.

1. `init_tracing_creates_log_dir_when_missing` — `tmp.path().join("sub")`, assert created.
2. `init_tracing_is_idempotent` — two back-to-back `init_tracing` calls don't panic.
3. `init_tracing_propagates_create_dir_failure` — point at a path whose parent is a file; assert `LoggingInitError::CreateDir { dir, .. }`.
4. `init_tracing_writes_to_configured_file_name_prefix` — call with `file_name_prefix = "aegis-test.log"`, emit a `tracing::info!("hello")`, drop the guard, assert the daily-rotate file exists in the dir with non-zero size.
5. `build_filter_defaults_to_info_when_env_missing` — `AEGIS_LOG_LEVEL` unset → `"info"`.
6. `build_filter_uses_aegis_log_level_when_set` — `AEGIS_LOG_LEVEL=debug` → `"debug"`.
7. `logging_config_clone_preserves_fields` — `#[derive(Clone)]` smoke test (locks in the derive).

The `trace_id` module carries the same six-test suite as `lib/crates/trace-id/src/lib.rs` (`client_side_starts_with_c`, `server_side_starts_with_s`, `no_device_prefix_omits_middle_segment`, `device_prefix_appears_in_middle`, `each_call_yields_a_fresh_ulid`, `side_enum_variants_are_distinct`). Both crates run their own copy of the suite; both must pass.

## Data model & wire shape

No DDL, no DTO, no request/response changes. The crate is library-only; it writes to the local filesystem via `tracing-appender`. The on-disk format (JSON, daily-rotate) matches what both apps produce today.

## Out of scope (explicitly NOT in this PR)

- **Refactoring `aegis-server/src/run.rs::init_tracing`** to call `logging_utils::init_tracing`. Server keeps its own copy.
- **Refactoring `aegis-desktop/src-tauri/src/tracing_init.rs`** to call `logging_utils::init_tracing`. Desktop keeps its own copy.
- **Consolidating the two copies of `TraceIdGenerator` / `Side`** (one in `logging-utils`, one in `trace-id`). Deferred to a future PR; both crates carry the logic independently for this PR.
- **Server-side HTTP trace helpers** (`TraceIdMakeSpan`, `extract_trace_id`, `is_valid_trace_id`, `MAX_TRACE_ID_LEN`, `X_TRACE_ID_HEADER`) — see [`2026-09-29-aegis-server-x-trace-id-design.md`](2026-09-29-aegis-server-x-trace-id-design.md). They are a natural fit for `logging-utils` once the server adopts the crate.
- **Desktop device-prefix file management** (`aegis-desktop/src-tauri/src/trace_id_setup.rs::load_or_create_device_prefix`). Depends on `tauri::AppHandle`; a future PR can move it into a `logging-utils::device_prefix` module once the desktop adopts the crate, at which point the module would gain a `tauri` dependency.

## Verification gate

```bash
cargo fmt --all -- --check
cargo clippy -p logging-utils --all-targets --all-features -- -D warnings
cargo test  -p logging-utils
cargo doc   -p logging-utils --no-deps
cargo check --workspace       # confirms the new member compiles alongside the rest
```

## File changes summary

- New: `lib/crates/logging-utils/Cargo.toml`
- New: `lib/crates/logging-utils/README.md`
- New: `lib/crates/logging-utils/src/lib.rs`
- New: `lib/crates/logging-utils/src/tracing_init.rs` (~60 lines + 7 tests)
- New: `lib/crates/logging-utils/src/trace_id.rs` (~80 lines + 6 tests, independent copy from `lib/crates/trace-id/src/lib.rs`)
- Modified: `Cargo.toml` (workspace root) — one new line in `[workspace].members` for `logging-utils`
- **Unchanged:** `lib/crates/trace-id/` — stays exactly as it is today; carries its own independent copy of the logic
- **Unchanged:** `apps/server/aegis-server/`, `apps/desktop/aegis-desktop/` — keep their own copies
