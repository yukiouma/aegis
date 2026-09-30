# `logging-utils` Workspace Crate — Design

**Date:** 2026-09-30
**Scope:** Add `lib/crates/logging-utils`, a workspace crate that unifies the tracing bootstrap currently duplicated in `aegis-server` and `aegis-desktop`, and absorbs the `TraceIdGenerator` / `Side` source from `lib/crates/trace-id`. The `trace-id` crate becomes a one-line backwards-compat shim so `aegis-server` and `aegis-desktop` keep compiling untouched.

## Motivation

The two runnable surfaces each ship a near-identical `init_tracing`:

- [`apps/server/aegis-server/src/run.rs:130-148`](apps/server/aegis-server/src/run.rs#L130-L148) — reads `AEGIS_LOG_DIR`, daily-rotates `aegis-server.log.YYYY-MM-DD`, JSON, `AEGIS_LOG_LEVEL` filter.
- [`apps/desktop/aegis-desktop/src-tauri/src/tracing_init.rs:43-58`](apps/desktop/aegis-desktop/src-tauri/src/tracing_init.rs#L43-L58) — takes `log_dir`, daily-rotates `aegis-desktop.log.YYYY-MM-DD`, JSON, `AEGIS_LOG_LEVEL` filter.

The only differences are the dir source (env-var vs parameter) and the file-name prefix. Everything else — JSON format, daily rotation, `EnvFilter`, `try_init` semantics, `LogGuard` ownership — is duplicated. Any future change (e.g. structured fields, OpenTelemetry export, async-flush support) has to be applied twice.

`lib/crates/trace-id` exists today (PR #86) but is only consumed by future HTTP-trace plumbing on the server side and the desktop's `trace_id_setup.rs`. Its types (`TraceIdGenerator`, `Side`) are observability primitives and their source is absorbed into `logging-utils` in this PR; the `trace-id` crate itself becomes a one-line shim so existing consumers keep compiling untouched.

This PR adds the crate. It does NOT touch the two apps — they keep their own copies until a follow-up PR migrates each call site. The new crate is built, tested, documented, and ready to be consumed.

## Approach

- Add `lib/crates/logging-utils` to the workspace.
- Provide `LoggingConfig { log_dir, file_name_prefix }` + `init_tracing(&LoggingConfig) -> Result<LogGuard, LoggingInitError>` so each app's call site is a five-line block.
- Move the `TraceIdGenerator` and `Side` source from `lib/crates/trace-id/src/lib.rs` into `lib/crates/logging-utils/src/trace_id.rs` (the canonical home). `lib/crates/trace-id/src/lib.rs` becomes a one-line `pub use logging_utils::*;` shim so `aegis-server` and `aegis-desktop` keep compiling untouched.
- Keep the crate small: no HTTP, no Tauri, no business types. Future PRs may fold in server-side `TraceIdMakeSpan` / `extract_trace_id` and desktop-side device-prefix management; the crate leaves room for them but does not introduce dependencies they would force.

## Architecture

```
lib/crates/logging-utils/
├── Cargo.toml
├── README.md
└── src/
    ├── lib.rs           # crate-level doc + pub use surface
    ├── tracing_init.rs  # LoggingConfig, LogGuard, init_tracing, build_filter, LoggingInitError
    └── trace_id.rs      # TraceIdGenerator, Side (moved from lib/crates/trace-id)
```

No DDD layers — this crate is observability infrastructure, not a business lib. The `lib-crate-development.md` guideline does not apply.

## Components

### 1. `src/tracing_init.rs`

Owns the tracing bootstrap. Mirrors the existing desktop `init_tracing` and server `init_tracing` in behaviour; differs only in taking a `LoggingConfig` parameter instead of resolving the dir itself.

```rust
use std::path::{Path, PathBuf};
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

### 2. `src/trace_id.rs` (moved from `lib/crates/trace-id/src/lib.rs`)

The `TraceIdGenerator` and `Side` types **live here now** — their source code is moved verbatim from `lib/crates/trace-id/src/lib.rs` into this module. The dependency on the workspace `ulid` crate (used to mint the time-sortable log id segment) is inherited via `{ workspace = true }`. The existing `trace-id` test suite moves with the code; no tests are dropped, none are added.

```rust
//! Trace ids that identify a single unit of work as it crosses a
//! client ↔ server boundary.
//!
//! (Moved verbatim from `lib/crates/trace-id/src/lib.rs`. See the
//! commit history of that file for the original author / context.)

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
#[derive(Debug, Clone)]
pub struct TraceIdGenerator {
    device_prefix: Option<String>,
}

impl TraceIdGenerator {
    pub fn new(device_prefix: Option<String>) -> Self {
        Self { device_prefix }
    }
    pub fn client_side(&self) -> String { /* …moved… */ }
    pub fn server_side(&self) -> String { /* …moved… */ }
}
```

### 3. `src/lib.rs`

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

pub mod trace_id;
pub mod tracing_init;

pub use trace_id::{Side, TraceIdGenerator};
pub use tracing_init::{
    build_filter, init_tracing, LogGuard, LoggingConfig, LoggingInitError,
};
```

## Workspace wiring

### Root `Cargo.toml`

```toml
[workspace]
members = [
    # … existing entries …
    "lib/crates/logging-utils",
    "lib/crates/trace-id",
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
# (moved from the `trace-id` crate, now a backwards-compat shim).
ulid               = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
```

### `lib/crates/trace-id` becomes a backwards-compat shim

The `lib/crates/trace-id` workspace crate is **not deleted** — it is rewritten as a one-line re-export so consumers that still depend on it directly (`aegis-server`, `aegis-desktop`) keep compiling without any change to their source:

```rust
// lib/crates/trace-id/src/lib.rs (full file after the rewrite)
pub use logging_utils::{Side, TraceIdGenerator};
```

`lib/crates/trace-id/Cargo.toml` gains `logging-utils = { path = "../logging-utils" }` as its sole `[dependencies]` entry and drops the now-unused `ulid` entry. The crate name, version, and edition stay the same so the existing path-deps from `aegis-server` and `aegis-desktop` (`trace-id = { path = "../../../../lib/crates/trace-id" }`) keep resolving without edits.

The shim is deleted in a follow-up PR once `aegis-server` and `aegis-desktop` both migrate to `use logging_utils::…` directly.

## Public API at a glance

| Symbol | Kind | Notes |
|---|---|---|
| `LoggingConfig` | struct | `{ log_dir: PathBuf, file_name_prefix: String }` |
| `LogGuard` | newtype | `pub LogGuard(pub WorkerGuard)` |
| `LoggingInitError` | enum | `CreateDir { dir, source }` only |
| `init_tracing(&LoggingConfig) -> Result<LogGuard, LoggingInitError>` | fn | Idempotent via `try_init` |
| `build_filter() -> EnvFilter` | fn | `AEGIS_LOG_LEVEL` → `EnvFilter`, default `info` |
| `Side`, `TraceIdGenerator` | types (moved from `trace-id`) | Owns the source; `trace-id` re-exports them |

## Testing

Port and adapt the union of the two existing init test suites. Env-var tests use the same `ENV_LOCK` + `EnvGuard` pattern already in both apps' tracing modules.

1. `init_tracing_creates_log_dir_when_missing` — `tmp.path().join("sub")`, assert created.
2. `init_tracing_is_idempotent` — two back-to-back `init_tracing` calls don't panic.
3. `init_tracing_propagates_create_dir_failure` — point at a path whose parent is a file; assert `LoggingInitError::CreateDir { dir, .. }`.
4. `init_tracing_writes_to_configured_file_name_prefix` — call with `file_name_prefix = "aegis-test.log"`, emit a `tracing::info!("hello")`, drop the guard, assert the daily-rotate file exists in the dir with non-zero size.
5. `build_filter_defaults_to_info_when_env_missing` — `AEGIS_LOG_LEVEL` unset → `"info"`.
6. `build_filter_uses_aegis_log_level_when_set` — `AEGIS_LOG_LEVEL=debug` → `"debug"`.
7. `logging_config_clone_preserves_fields` — `#[derive(Clone)]` smoke test (locks in the derive).

The `trace-id` crate's existing test suite moves with the code into `logging-utils::trace_id`'s `#[cfg(test)] mod tests`.

## Data model & wire shape

No DDL, no DTO, no request/response changes. The crate is library-only; it writes to the local filesystem via `tracing-appender`. The on-disk format (JSON, daily-rotate) matches what both apps produce today.

## Out of scope (explicitly NOT in this PR)

- **Refactoring `aegis-server/src/run.rs::init_tracing`** to call `logging_utils::init_tracing`. Server keeps its own copy.
- **Refactoring `aegis-desktop/src-tauri/src/tracing_init.rs`** to call `logging_utils::init_tracing`. Desktop keeps its own copy.
- **Server-side HTTP trace helpers** (`TraceIdMakeSpan`, `extract_trace_id`, `is_valid_trace_id`, `MAX_TRACE_ID_LEN`, `X_TRACE_ID_HEADER`) — see [`2026-09-29-aegis-server-x-trace-id-design.md`](2026-09-29-aegis-server-x-trace-id-design.md). They are a natural fit for `logging-utils` once the server adopts the crate.
- **Desktop device-prefix file management** (`aegis-desktop/src-tauri/src/trace_id_setup.rs::load_or_create_device_prefix`). Depends on `tauri::AppHandle`; a future PR can move it into a `logging-utils::device_prefix` module once the desktop adopts the crate, at which point the module would gain a `tauri` dependency.
- **Deleting the `lib/crates/trace-id` crate.** It is rewritten as a backwards-compat shim in this PR (one-line re-export); deletion happens in the consumer-migration PR that flips `aegis-server` / `aegis-desktop` to import from `logging-utils` directly.

## Verification gate

```bash
cargo fmt --all -- --check
cargo clippy -p logging-utils --all-targets --all-features -- -D warnings
cargo clippy -p trace-id      --all-targets --all-features -- -D warnings
cargo test  -p logging-utils
cargo test  -p trace-id       # confirms the shim still resolves and re-exports compile
cargo doc   -p logging-utils --no-deps
cargo check --workspace       # confirms the new member + the shim compile alongside the rest
```

## File changes summary

- New: `lib/crates/logging-utils/Cargo.toml`
- New: `lib/crates/logging-utils/README.md`
- New: `lib/crates/logging-utils/src/lib.rs`
- New: `lib/crates/logging-utils/src/tracing_init.rs` (~60 lines + tests)
- New: `lib/crates/logging-utils/src/trace_id.rs` (~60 lines + tests, moved from `lib/crates/trace-id/src/lib.rs`)
- Modified: `lib/crates/trace-id/src/lib.rs` — replaced with one-line `pub use logging_utils::*;` shim
- Modified: `lib/crates/trace-id/Cargo.toml` — replace `ulid` dep with `logging-utils = { path = "../logging-utils" }`
- Modified: `Cargo.toml` (workspace root) — one new line in `[workspace].members` for `logging-utils`
