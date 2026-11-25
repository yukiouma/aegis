# aegis-desktop Logging & Trace-Id Wiring Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `tracing` to the Tauri backend of `aegis-desktop`, mint a per-install device prefix, and thread the trace id through every command shim into the outbound HTTP request as `X-Trace-ID`.

**Architecture:** Two new modules in `src-tauri/src/` (`tracing_init` and `trace_id_setup`) wire the trace-id crate + nanoid + tracing at startup. Every `#[tauri::command]` shim gets a uniform logging wrap (mint id → enter span → scope http call → log enter/success/failed). The `http/*` modules stay untouched; `http/client.rs` grows a `tokio::task_local!` read + `X-Trace-ID` header attach on its `send` and `refresh_with_lock` paths.

**Tech Stack:** Rust 2024 edition; `nanoid`, `tracing`, `tracing-subscriber`, `tracing-appender` (all pinned in workspace). The `trace-id` workspace crate (already at `lib/crates/trace-id/`, version 0.1.0).

## Global Constraints

- Spec source of truth: [`docs/superpowers/specs/2026-09-29-aegis-desktop-logging-design.md`](docs/superpowers/specs/2026-09-29-aegis-desktop-logging-design.md) — every decision below traces to a section in it.
- Log destination: JSON, daily-rotating file `aegis-desktop.log.YYYY-MM-DD`. Default dir is `<app_data_dir>/logs`; env var `AEGIS_LOG_DIR` overrides.
- Log filter: `EnvFilter` from `AEGIS_LOG_LEVEL` (default `info`).
- device_prefix: persisted per-install `nanoid!()` (21 chars, default URL-safe alphabet) at `<app_data_dir>/aegis-desktop-device-prefix`. Lazy-create on startup; I/O errors fall back to `None` and emit a warning.
- Trace ID plumbing: `tokio::task_local! { pub static TRACE_ID: String; }` declared in `http/client.rs`. Command shims scope the existing http call once; `HttpClient::send` and `refresh_with_lock` read it and attach `X-Trace-ID`.
- Header name: literal `X-Trace-ID` (matches server-side convention).
- Verification commands (run from workspace root before any commit claims completeness):
  ```bash
  cargo fmt --all -- --check
  cargo clippy -p aegis-desktop --all-targets --all-features -- -D warnings
  cargo test -p aegis-desktop
  ```

---

## File Structure

```text
Cargo.toml                                          # modify: add `nanoid`, `tempfile` to workspace.dependencies
apps/desktop/aegis-desktop/src-tauri/Cargo.toml     # modify: pull nanoid, tracing, tracing-subscriber,
                                                     #   tracing-appender via { workspace = true };
                                                     #   add tempfile to dev-dependencies
apps/desktop/aegis-desktop/src-tauri/src/
  tracing_init.rs                                   # new: init_tracing(&Path) + LogGuard + build_filter + tests
  trace_id_setup.rs                                 # new: load_or_create(&Path) + DEVICE_PREFIX_FILE_NAME
  lib.rs                                             # modify: add mod declarations; .setup calls
  http/client.rs                                    # modify: task_local! TRACE_ID; X-Trace-ID on send();
                                                     #   instrument on send; thread header into refresh_with_lock;
                                                     #   3 new wiremock tests
  commands/auth.rs                                  # modify: every command shim wraps with logging pattern;
                                                     #   1 new captured-log smoke test
  commands/healthz.rs                               # modify: same pattern
  commands/identity.rs                              # modify: same pattern
  commands/mission.rs                               # modify: same pattern
  commands/project.rs                               # modify: same pattern
  commands/user.rs                                  # modify: same pattern
  commands/user_credential.rs                       # modify: same pattern
  commands/crf/{version,form,item,option,unit,annotation,domain_annotation}.rs
                                                    # modify: same pattern (one shim per file)
  commands/domain_model/{domain,variable,version}.rs
                                                    # modify: same pattern
  commands/terminology/{code_item,code_list,import,version}.rs
                                                    # modify: same pattern
```

---

## Task 1: Add workspace + crate dependencies

**Files:**
- Modify: `Cargo.toml` (workspace root)
- Modify: `apps/desktop/aegis-desktop/src-tauri/Cargo.toml`

**Interfaces:**
- Consumes: nothing.
- Produces: `nanoid` and `tempfile` available as `{ workspace = true }`; `aegis-desktop` resolves all four logging deps.

- [ ] **Step 1: Add `nanoid` and `tempfile` to `[workspace.dependencies]`**

In the root `Cargo.toml`, append to `[workspace.dependencies]` (keep the prior entries' ordering):

```toml
# `nanoid` mints the URL-safe device prefix persisted to
# `<app_data_dir>/aegis-desktop-device-prefix`. The default 21-char
# length + alphanumeric + `-_` alphabet gives ~10^36 ids, so collisions
# across installs are vanishingly unlikely without coordination.
nanoid = "0.7"
# `tempfile` provides scoped temp dirs for tests that exercise the
# filesystem (tracing_init writes a log file; trace_id_setup reads /
# writes the device-prefix file). Used only as a dev-dep; production
# builds don't link it.
tempfile = "3"
```

- [ ] **Step 2: Pull the four logging deps + `nanoid` into the desktop crate**

In `apps/desktop/aegis-desktop/src-tauri/Cargo.toml`, under `[dependencies]` (insert after `tauri-plugin-dialog = "2"`, keep alphabetical):

```toml
# `nanoid` mints the per-install device prefix that becomes the
# middle segment of every trace id. Workspace-pinned so the same
# version is used by any future crate that wants to mint ids.
nanoid = { workspace = true }
# `tracing` is the logging facade the desktop backend emits through.
# Every command shim logs `enter` / `success` / `failed` events on a
# span that carries `trace_id`; HttpClient emits request lines.
tracing = { workspace = true }
# `tracing-subscriber` initialises the global subscriber in
# `tracing_init::init_tracing`. `env-filter` + `json` features let us
# filter by level and emit machine-readable events.
tracing-subscriber = { workspace = true }
# `tracing-appender` provides the rolling file writer (one file per
# day) used by `tracing_init`. The `non_blocking` wrapper keeps the
# request path off the log-write critical section.
tracing-appender = { workspace = true }
```

- [ ] **Step 3: Add `tempfile` to the desktop crate's `[dev-dependencies]`**

In the same file, under `[dev-dependencies]` (after `wiremock = "0.6"`, keep alphabetical):

```toml
# `tempfile::tempdir()` scopes a per-test temp directory so
# `tracing_init` and `trace_id_setup` tests don't touch the repo
# working tree.
tempfile = { workspace = true }
```

- [ ] **Step 4: Verify the dep wiring builds**

Run: `cargo check -p aegis-desktop`
Expected: exit 0; the only output is "Checking aegis-desktop" / "Finished". No code uses these deps yet, so build is still empty.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml apps/desktop/aegis-desktop/src-tauri/Cargo.toml
git commit -m "chore(deps): add nanoid, tempfile, tracing to aegis-desktop

- nanoid mints the per-install device prefix in
  trace_id_setup::load_or_create
- tempfile scopes per-test directories for tracing_init and
  trace_id_setup tests
- tracing / tracing-subscriber / tracing-appender are pulled in
  for the JSON daily-rotating log sink

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

## Task 2: `tracing_init` module — init, guard, filter

**Files:**
- Create: `apps/desktop/aegis-desktop/src-tauri/src/tracing_init.rs`

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `pub struct LogGuard(pub tracing_appender::non_blocking::WorkerGuard)`
  - `pub fn init_tracing(log_dir: &Path) -> Result<LogGuard, TracingInitError>`
  - `pub fn build_filter() -> EnvFilter`

- [ ] **Step 1: Write the failing tests**

In a fresh `apps/desktop/aegis-desktop/src-tauri/src/tracing_init.rs`:

```rust
//! Tracing bootstrap for the aegis-desktop Tauri backend.
//!
//! Mirrors `apps/server/aegis-server/src/run.rs::init_tracing` in
//! shape: a JSON daily-rotating file appender, an `EnvFilter` driven
//! by `AEGIS_LOG_LEVEL`, and a `WorkerGuard` that the caller MUST
//! hold for the process lifetime or buffered writes are lost on
//! shutdown.
//!
//! `init_tracing` is split from the Tauri-resolving glue so the
//! pure path-taking form is testable without a Tauri `AppHandle`.

use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Error)]
pub enum TracingInitError {
    #[error("failed to create log directory {dir}: {source}")]
    CreateDir {
        dir: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// Owns the `WorkerGuard` returned by `tracing_appender::non_blocking`
/// so the caller can stash it in managed state and the guard lives
/// for the process lifetime.
pub struct LogGuard(pub WorkerGuard);

/// Initialise the global tracing subscriber. Writes JSON events to
/// `log_dir/aegis-desktop.log.YYYY-MM-DD` (one file per day, rotation
/// at local midnight).
///
/// The returned [`LogGuard`] MUST be held for the lifetime of the
/// program; dropping it flushes the buffered writer and any pending
/// events are lost.
///
/// `try_init` swallows the "already initialized" error so re-entry
/// from tests is a no-op.
pub fn init_tracing(log_dir: &Path) -> Result<LogGuard, TracingInitError> {
    std::fs::create_dir_all(log_dir).map_err(|source| TracingInitError::CreateDir {
        dir: log_dir.to_path_buf(),
        source,
    })?;
    let file_appender = tracing_appender::rolling::daily(log_dir, "aegis-desktop.log");
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
/// `info` when the variable is unset.
pub fn build_filter() -> EnvFilter {
    let level = std::env::var("AEGIS_LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
    EnvFilter::new(level)
}

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
        unsafe { std::env::set_var(key, value); }
        EnvGuard { key, prev }
    }

    #[test]
    fn init_tracing_creates_log_dir_when_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let log_dir = tmp.path().join("aegis-subdir");
        assert!(!log_dir.exists());
        let _g = lock_env();
        let _lvl = set_env("AEGIS_LOG_LEVEL", "info");
        let guard = init_tracing(&log_dir).expect("init_tracing succeeds");
        assert!(log_dir.is_dir(), "log dir should be created");
        // Hold the guard so the writer isn't flushed mid-test.
        let _ = guard;
    }

    #[test]
    fn init_tracing_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let _g = lock_env();
        let _lvl = set_env("AEGIS_LOG_LEVEL", "info");
        // Two calls in a row must not panic — `try_init` swallows the
        // "already initialized" error.
        let _a = init_tracing(tmp.path()).expect("first init");
        let _b = init_tracing(tmp.path()).expect("second init");
    }

    #[test]
    fn init_tracing_propagates_create_dir_failure() {
        // Pointing the log dir at a path whose parent is a regular
        // file forces `create_dir_all` to fail with NotADirectory.
        let tmp = tempfile::tempdir().unwrap();
        let blocker = tmp.path().join("blocker");
        std::fs::write(&blocker, b"not a dir").unwrap();
        let bad = blocker.join("inside");
        let err = init_tracing(&bad).unwrap_err();
        match err {
            TracingInitError::CreateDir { dir, .. } => assert_eq!(dir, bad),
        }
    }

    #[test]
    fn build_filter_defaults_to_info_when_env_missing() {
        let _g = lock_env();
        // SAFETY: serialized via ENV_LOCK.
        unsafe { std::env::remove_var("AEGIS_LOG_LEVEL"); }
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
}
```

- [ ] **Step 2: Run tests; verify they fail for the right reason**

Run: `cargo test -p aegis-desktop --lib tracing_init::`
Expected: compile error (`init_tracing` and `LogGuard` not defined) → 5 unresolved items, not a passing test.

- [ ] **Step 3: Implement the module**

The file above is the implementation. No further code needed.

- [ ] **Step 4: Run tests; verify they pass**

Run: `cargo test -p aegis-desktop --lib tracing_init::`
Expected: 5 passed.

- [ ] **Step 5: Run clippy on the new file**

Run: `cargo clippy -p aegis-desktop --lib --all-features -- -D warnings 2>&1 | grep -A2 tracing_init`
Expected: no diagnostics.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/tracing_init.rs
git commit -m "feat(desktop): tracing_init module with JSON daily rotation

- init_tracing(&Path) builds a JSON daily-rotating subscriber
  writing to <dir>/aegis-desktop.log.YYYY-MM-DD
- LogGuard owns the WorkerGuard so the caller can stash it in
  Tauri managed state and the buffered writer lives for the
  process lifetime
- build_filter() reads AEGIS_LOG_LEVEL (defaults to info)
- 5 tests pin idempotency, default level, env-var level, dir
  creation, and the create-dir-failure path

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

## Task 3: `trace_id_setup` module — per-install device prefix

**Files:**
- Create: `apps/desktop/aegis-desktop/src-tauri/src/trace_id_setup.rs`

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `pub const DEVICE_PREFIX_FILE_NAME: &str = "aegis-desktop-device-prefix";`
  - `pub fn load_or_create(app_data_dir: &Path) -> TraceIdGenerator`

- [ ] **Step 1: Write the failing tests**

In a fresh `apps/desktop/aegis-desktop/src-tauri/src/trace_id_setup.rs`:

```rust
//! Persists a per-install device prefix in the Tauri app-data
//! directory and constructs a `TraceIdGenerator` from it.
//!
//! The first launch mints and writes; subsequent launches re-use the
//! persisted value so a workstation keeps a stable middle segment on
//! every trace id it emits. The I/O is best-effort: a read or write
//! failure falls back to a `None` device prefix and logs a warning,
//! so a corrupted app-data directory does not prevent the app from
//! launching. Trace ids are observability, not a hard requirement.

use std::path::Path;

use trace_id::TraceIdGenerator;

pub const DEVICE_PREFIX_FILE_NAME: &str = "aegis-desktop-device-prefix";

/// Load the device prefix from `<app_data_dir>/aegis-desktop-device-prefix`,
/// or mint a fresh `nanoid!()` and write it if the file is missing.
///
/// I/O failures (no app data dir, permission denied, write failure)
/// fall back to `TraceIdGenerator::new(None)` — the caller logs the
/// condition separately so a corrupted app-data directory does not
/// stop the app from booting.
pub fn load_or_create(app_data_dir: &Path) -> TraceIdGenerator {
    let path = app_data_dir.join(DEVICE_PREFIX_FILE_NAME);
    let prefix = match std::fs::read_to_string(&path) {
        Ok(s) => {
            let trimmed = s.trim().to_string();
            if trimmed.is_empty() {
                mint_and_write(&path).unwrap_or_else(|| fallback())
            } else {
                trimmed
            }
        }
        Err(_) => mint_and_write(&path).unwrap_or_else(fallback),
    };
    TraceIdGenerator::new(Some(prefix))
}

fn mint_and_write(path: &Path) -> Option<String> {
    let id = nanoid::nanoid!();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(path, &id).ok()?;
    Some(id)
}

fn fallback() -> String {
    // nanoid!() cannot fail, but a `None` device_prefix would require
    // re-plumbing the API. Fall back to a freshly-minted id (no
    // persistence) so callers always get a usable prefix.
    nanoid::nanoid!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_or_create_writes_new_nanoid_when_file_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        let prefix_path = dir.join(DEVICE_PREFIX_FILE_NAME);
        assert!(!prefix_path.exists());

        let generator = load_or_create(dir);
        let id = generator.client_side();

        assert!(prefix_path.is_file(), "prefix file should be created");
        let persisted = std::fs::read_to_string(&prefix_path).unwrap();
        assert!(!persisted.is_empty());
        let parts: Vec<&str> = id.split('-').collect();
        assert_eq!(parts.len(), 3, "id should have 3 segments, got {id:?}");
        assert_eq!(parts[1], persisted.trim());
        assert_eq!(parts[0], "C");
    }

    #[test]
    fn load_or_create_reuses_existing_prefix_file() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        let prefix_path = dir.join(DEVICE_PREFIX_FILE_NAME);
        let existing = "fixed-prefix-abc123";
        std::fs::write(&prefix_path, existing).unwrap();

        let generator = load_or_create(dir);
        let id = generator.client_side();

        let parts: Vec<&str> = id.split('-').collect();
        assert_eq!(parts.len(), 3, "id should have 3 segments, got {id:?}");
        assert_eq!(parts[1], existing);
        // The file should not be overwritten.
        let after = std::fs::read_to_string(&prefix_path).unwrap();
        assert_eq!(after, existing);
    }

    #[test]
    fn load_or_create_trims_whitespace_around_existing_prefix() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        let prefix_path = dir.join(DEVICE_PREFIX_FILE_NAME);
        std::fs::write(&prefix_path, "  trimmed-xyz\n").unwrap();

        let generator = load_or_create(dir);
        let id = generator.client_side();
        let parts: Vec<&str> = id.split('-').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[1], "trimmed-xyz");
    }

    #[test]
    fn load_or_create_replaces_empty_existing_file() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        let prefix_path = dir.join(DEVICE_PREFIX_FILE_NAME);
        std::fs::write(&prefix_path, "").unwrap();

        let generator = load_or_create(dir);
        let id = generator.client_side();
        let parts: Vec<&str> = id.split('-').collect();
        assert_eq!(parts.len(), 3, "expected 3 segments, got {id:?}");
        let persisted = std::fs::read_to_string(&prefix_path).unwrap();
        assert!(!persisted.is_empty());
        assert_eq!(parts[1], persisted);
    }

    #[test]
    fn load_or_create_returns_usable_prefix_when_write_fails() {
        // Point the dir at a path whose parent is a regular file so
        // create_dir_all on the prefix file's parent fails. The
        // function should still return a TraceIdGenerator (with a
        // freshly-minted, non-persisted prefix) instead of panicking.
        let tmp = tempfile::tempdir().unwrap();
        let blocker = tmp.path().join("blocker");
        std::fs::write(&blocker, b"not a dir").unwrap();
        let bad = blocker.join("nested");

        let generator = load_or_create(&bad);
        let id = generator.client_side();
        // 2 or 3 segments is acceptable here — both reflect a
        // working generator. We only assert it does not panic and
        // produces a well-formed id.
        assert!(id.starts_with("C-"), "got {id:?}");
        assert!(id.len() > "C-".len() + 10, "expected a ULID tail, got {id:?}");
    }
}
```

- [ ] **Step 2: Run tests; verify they fail for the right reason**

Run: `cargo test -p aegis-desktop --lib trace_id_setup::`
Expected: compile error (`load_or_create` not defined) → unresolved import.

- [ ] **Step 3: Implement the module**

The file above is the implementation.

- [ ] **Step 4: Add `trace-id` as a runtime dep in the Tauri crate**

The `trace_id_setup` module imports `trace_id::TraceIdGenerator`, so the Tauri crate needs the dep. In `apps/desktop/aegis-desktop/src-tauri/Cargo.toml`, under `[dependencies]`:

```toml
# `trace-id` mints the per-trace-id strings. Used by
# `trace_id_setup::load_or_create` at startup; commands pull the
# constructed `TraceIdGenerator` from managed state and call
# `client_side()` per invocation. Local path dep so any change to
# the crate is picked up without a registry bump.
trace-id = { path = "../../../../lib/crates/trace-id" }
```

- [ ] **Step 5: Run tests; verify they pass**

Run: `cargo test -p aegis-desktop --lib trace_id_setup::`
Expected: 5 passed.

- [ ] **Step 6: Run clippy on the new file**

Run: `cargo clippy -p aegis-desktop --lib --all-features -- -D warnings 2>&1 | grep -A2 trace_id_setup`
Expected: no diagnostics.

- [ ] **Step 7: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/trace_id_setup.rs apps/desktop/aegis-desktop/src-tauri/Cargo.toml
git commit -m "feat(desktop): per-install nanoid device prefix loader

- trace_id_setup::load_or_create(&Path) reads
  aegis-desktop-device-prefix from the app-data dir; if missing
  or empty, mints a fresh nanoid!() and persists it
- I/O failures fall back to a freshly-minted (non-persisted)
  prefix so the app still boots — trace ids are observability,
  not a hard requirement
- 5 tests pin create / reuse / trim / replace-empty / write-fail

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

## Task 4: `http/client.rs` — task_local + `X-Trace-ID` header on `send`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/http/client.rs`

**Interfaces:**
- Consumes: existing `HttpClient::send` signature.
- Produces:
  - `pub static TRACE_ID: tokio::task_local!{String}` declared at module top.
  - `send` attaches `X-Trace-ID` when `TRACE_ID.try_with(...).ok()` returns `Some(id)`.

- [ ] **Step 1: Write the failing tests**

Append to the existing `#[cfg(test)] mod tests` block at the bottom of `http/client.rs`:

```rust
    use crate::http::client::TRACE_ID;

    #[tokio::test]
    async fn trace_id_header_attached_when_task_local_set() {
        let server = MockServer::start().await;
        let store = Arc::new(MemoryStore::default());
        let trace_id = "C-desktop-01H9XQ8Z6VK3FJ4P5N2W7Y0T8CB";
        server
            .register(
                Mock::given(method("GET"))
                    .and(path("/api/user"))
                    .and(header("X-Trace-ID", trace_id))
                    .respond_with(ResponseTemplate::new(200).set_body_json(
                        serde_json::json!({"users": []})
                    )),
            )
            .await;
        let c = client_for(&server, store);
        let _: serde_json::Value = TRACE_ID
            .scope(trace_id.to_string(), async {
                c.request::<(), serde_json::Value>(reqwest::Method::GET, "/api/user", None)
                    .await
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn trace_id_header_absent_when_no_task_local() {
        let server = MockServer::start().await;
        let store = Arc::new(MemoryStore::default());
        store.set_access_token("AT_AAA").await.unwrap();
        // NoHeader on X-Trace-ID + a positive match on auth so the
        // request still succeeds.
        server
            .register(
                Mock::given(method("GET"))
                    .and(path("/api/user"))
                    .and(header("authorization", "Bearer AT_AAA"))
                    .and(NoHeader("X-Trace-ID"))
                    .respond_with(ResponseTemplate::new(200).set_body_json(
                        serde_json::json!({"users": []})
                    )),
            )
            .await;
        let c = client_for(&server, store);
        let _: serde_json::Value = c
            .request::<(), serde_json::Value>(reqwest::Method::GET, "/api/user", None)
            .await
            .unwrap();
    }
```

- [ ] **Step 2: Run the two new tests; verify they fail for the right reason**

Run: `cargo test -p aegis-desktop --lib http::client::tests::trace_id`
Expected: compile error — `TRACE_ID` not in scope. The positive test then fails with "header X-Trace-ID not matched"; the negative test fails because the NoHeader matcher is not yet defined for X-Trace-ID.

- [ ] **Step 3: Add the `task_local!` declaration at module top**

In `apps/desktop/aegis-desktop/src-tauri/src/http/client.rs`, after the existing `use std::sync::Arc;` line, add:

```rust
/// Per-task trace id minted by the command shim and consumed by
/// `HttpClient::send` to attach `X-Trace-ID` to every outbound
/// request. Set via `TRACE_ID.scope(id, async { … })` at the top of
/// every `#[tauri::command]`. Read with `TRACE_ID.try_with(|id|
/// id.clone()).ok()` inside `send` — `None` means no trace id is
/// available and the header is omitted.
tokio::task_local! {
    pub static TRACE_ID: String;
}
```

- [ ] **Step 4: Update `HttpClient::send` to attach the header**

Inside the existing `async fn send<TReq>(...)` method (find the line `let mut rb = self.http.request(method, &url);`), change the body so the header is conditionally attached:

```rust
        let url = self.url(path);
        let mut rb = self.http.request(method, &url);
        if let Some(t) = token {
            rb = rb.bearer_auth(t);
        }
        if let Some(b) = body {
            rb = rb.json(b);
        }
        // The command shim scopes a fresh trace id around the http
        // fn call; if one is present, attach it as X-Trace-ID so the
        // server can correlate the request with our log line. We
        // intentionally do NOT mint a fallback id here — every
        // trace id in logs must have been minted explicitly by the
        // shim.
        if let Ok(id) = TRACE_ID.try_with(|id| id.clone()) {
            rb = rb.header("X-Trace-ID", id);
        }
        let resp = rb.send().await?;
        let status = resp.status();
        let bytes = resp.bytes().await?.to_vec();
        Ok((status, bytes))
```

- [ ] **Step 5: Run the two new tests; verify they pass**

Run: `cargo test -p aegis-desktop --lib http::client::tests::trace_id`
Expected: 2 passed.

- [ ] **Step 6: Run the full client test module; verify no regression**

Run: `cargo test -p aegis-desktop --lib http::client::`
Expected: all existing tests + 2 new ones pass.

- [ ] **Step 7: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/http/client.rs
git commit -m "feat(desktop): thread X-Trace-ID header from task-local into HttpClient::send

- Declares tokio::task_local!{TRACE_ID: String} at module top
- send() reads it via try_with and attaches as X-Trace-ID when
  present; absent when no command shim is in scope (e.g. tests
  that exercise the client directly)
- 2 wiremock tests pin the present and absent cases

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

## Task 5: `http/client.rs` — instrument `send` + thread header into `refresh_with_lock`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/http/client.rs`

**Interfaces:**
- Consumes: existing `send` and `refresh_with_lock` signatures.
- Produces: `send` carries a `#[tracing::instrument]` span; `refresh_with_lock`'s outbound POST carries `X-Trace-ID` from the same `TRACE_ID` task-local.

- [ ] **Step 1: Write the failing test**

Append to the existing test module:

```rust
    #[tokio::test]
    async fn refresh_with_lock_attaches_trace_id_header() {
        let server = MockServer::start().await;
        let store = Arc::new(MemoryStore::default());
        store.set_access_token("AT_STALE").await.unwrap();
        store.set_refresh_token("RT").await.unwrap();
        let trace_id = "C-desktop-refreshtoken";

        server
            .register(
                Mock::given(method("POST"))
                    .and(path("/api/auth/refresh"))
                    .and(header("X-Trace-ID", trace_id))
                    .respond_with(ResponseTemplate::new(200).set_body_json(
                        serde_json::json!({"accessToken": "AT_NEW"})
                    )),
            )
            .await;
        server
            .register(
                Mock::given(method("GET"))
                    .and(path("/api/user"))
                    .and(header("authorization", "Bearer AT_STALE"))
                    .respond_with(ResponseTemplate::new(401).set_body_json(
                        serde_json::json!({"code": "token_verification_failed", "message": "expired"})
                    )),
            )
            .await;
        server
            .register(
                Mock::given(method("GET"))
                    .and(path("/api/user"))
                    .and(header("authorization", "Bearer AT_NEW"))
                    .respond_with(ResponseTemplate::new(200).set_body_json(
                        serde_json::json!({"users": []})
                    )),
            )
            .await;

        let c = client_for(&server, store);
        let _: serde_json::Value = TRACE_ID
            .scope(trace_id.to_string(), async {
                c.request::<(), serde_json::Value>(reqwest::Method::GET, "/api/user", None)
                    .await
            })
            .await
            .unwrap();
    }
```

- [ ] **Step 2: Run the new test; verify it fails**

Run: `cargo test -p aegis-desktop --lib http::client::tests::refresh_with_lock_attaches_trace_id`
Expected: fail — the refresh mock's `X-Trace-ID` matcher is not satisfied because `refresh_with_lock` doesn't attach the header yet.

- [ ] **Step 3: Add `#[tracing::instrument]` on `send`**

Add the attribute to the `send` method signature. The whole signature becomes:

```rust
    #[tracing::instrument(level = "debug", skip_all, fields(method = %method, path = %path))]
    async fn send<TReq>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&TReq>,
        token: Option<String>,
    ) -> Result<(reqwest::StatusCode, Vec<u8>), ApiError>
    where
        TReq: Serialize + ?Sized,
    {
```

- [ ] **Step 4: Attach `X-Trace-ID` inside `refresh_with_lock`'s outbound POST**

Find the line `let resp = self.http.post(&url).json(&Req { ... }).send().await?;` inside `refresh_with_lock`. Replace the chain with one that also attaches `X-Trace-ID`:

```rust
        let mut rb = self.http.post(&url).json(&Req {
            refresh_token: &refresh_token,
        });
        if let Ok(id) = TRACE_ID.try_with(|id| id.clone()) {
            rb = rb.header("X-Trace-ID", id);
        }
        let resp = rb.send().await?;
```

- [ ] **Step 5: Run the new test; verify it passes**

Run: `cargo test -p aegis-desktop --lib http::client::tests::refresh_with_lock_attaches_trace_id`
Expected: 1 passed.

- [ ] **Step 6: Run the full client test module; verify no regression**

Run: `cargo test -p aegis-desktop --lib http::client::`
Expected: all tests pass (the instrument attribute does not change observable test behaviour).

- [ ] **Step 7: Run clippy**

Run: `cargo clippy -p aegis-desktop --lib --all-features -- -D warnings 2>&1 | head -40`
Expected: no new diagnostics.

- [ ] **Step 8: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/http/client.rs
git commit -m "feat(desktop): instrument HttpClient::send and thread header into refresh path

- #[tracing::instrument] on send emits a per-request span carrying
  method + path (debug level)
- refresh_with_hand_attach_x_trace_id now also reads TRACE_ID and
  attaches X-Trace-ID, so the retry path is observable too
- 1 wiremock test pins the refresh path's header attachment

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

## Task 6: `lib.rs` — wire `tracing_init` and `trace_id_setup` into the Tauri setup

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: existing `run()` signature.
- Produces:
  - `mod tracing_init;` and `mod trace_id_setup;` declared.
  - `.setup` opens the log dir under `<app_data_dir>/logs`, initialises tracing, stashes the `LogGuard`.
  - `.setup` constructs a `TraceIdGenerator` from the per-install device prefix, stashes it.

- [ ] **Step 1: Add `mod` declarations**

In `apps/desktop/aegis-desktop/src-tauri/src/lib.rs`, near the top, after the existing `mod system;`:

```rust
mod tracing_init;
mod trace_id_setup;
```

- [ ] **Step 2: Update `.setup` to initialise tracing and load the device prefix**

The current `.setup` body is:

```rust
        .setup(|app| {
            let store = app
                .store("auth.bin")
                .map_err(|e| format!("failed to open auth.bin store: {e}"))?;
            let tokens = Arc::new(http::client::TauriStore::new(store));
            let client = http::client::HttpClient::new(http::config::BASE_URL.to_string(), tokens);
            app.manage(client);
            Ok(())
        })
```

Replace it with:

```rust
        .setup(|app| {
            // Tracing init: prefer $AEGIS_LOG_DIR; fall back to
            // <app_data_dir>/logs. LogGuard is stashed in managed
            // state so the buffered writer lives for the process.
            let log_dir = std::env::var("AEGIS_LOG_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| {
                    app.path()
                        .app_data_dir()
                        .expect("app_data_dir resolves")
                        .join("logs")
                });
            let log_guard = tracing_init::init_tracing(&log_dir)
                .map_err(|e| format!("init_tracing: {e}"))?;
            tracing::info!(
                log_dir = %log_dir.display(),
                "aegis-desktop tracing initialised"
            );
            app.manage(log_guard);

            // Per-install device prefix: persisted in app-data dir
            // so a workstation keeps a stable middle segment on
            // every trace id it emits. load_or_create is best-effort
            // and falls back to None device prefix on I/O errors.
            let app_data_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("app_data_dir: {e}"))?;
            let generator = trace_id_setup::load_or_create(&app_data_dir);
            tracing::info!(
                device_prefix_file = %app_data_dir
                    .join(trace_id_setup::DEVICE_PREFIX_FILE_NAME)
                    .display(),
                "trace id generator ready"
            );
            app.manage(generator);

            let store = app
                .store("auth.bin")
                .map_err(|e| format!("failed to open auth.bin store: {e}"))?;
            let tokens = Arc::new(http::client::TauriStore::new(store));
            let client = http::client::HttpClient::new(http::config::BASE_URL.to_string(), tokens);
            app.manage(client);
            Ok(())
        })
```

Add the missing import at the top of the file alongside the existing `use tauri::Manager;`:

```rust
use std::path::PathBuf;
```

- [ ] **Step 3: Verify the build is green**

Run: `cargo check -p aegis-desktop`
Expected: exit 0, no diagnostics.

- [ ] **Step 4: Run the full library test suite; verify no regression**

Run: `cargo test -p aegis-desktop --lib`
Expected: all tests pass (the new modules and the header attach are additive; existing tests don't touch them).

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/lib.rs
git commit -m "feat(desktop): wire tracing init and trace-id generator into Tauri setup

- mod tracing_init and mod trace_id_setup declared
- .setup resolves <app_data_dir>/logs (or AEGIS_LOG_DIR override),
  initialises tracing, stashes LogGuard in managed state
- .setup constructs a TraceIdGenerator from the per-install device
  prefix and stashes it in managed state

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

## Task 7: Wrap every command shim with the logging pattern

**Files:**
- Modify: every `src-tauri/src/commands/*.rs` file (one mechanical edit per `#[tauri::command]` function).
- Add: nothing (no per-shim captured-log test — see Step 3).

**Interfaces:**
- Consumes: each existing command's signature.
- Produces: every command now mints a trace id, opens a span, logs enter / success / failed, and scopes the existing http call into `TRACE_ID`.

- [ ] **Step 1: Define the shim wrap pattern**

The uniform wrap is:

```rust
#[tauri::command]
pub async fn <name>(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    // … existing args, unchanged …
) -> Result<<ret>, ApiError> {
    let trace_id = generator.client_side();
    let span = tracing::info_span!("command", trace_id = %trace_id, command = "<name>");
    let _enter = span.enter();
    tracing::info!("enter");

    let result = crate::http::client::TRACE_ID
        .scope(trace_id, async {
            // the existing http call, unchanged
            crate::http::<module>::<fn>(&client, /* … existing args … */).await
        })
        .await;

    match &result {
        Ok(_) => tracing::info!("success"),
        Err(e) => tracing::error!(error = %e, "failed"),
    }
    result
}
```

Three rules for every wrap:
1. The `generator: State<'_, TraceIdGenerator>` parameter is added alongside `client: State<'_, HttpClient>` (insert `generator` first or second — pick second so existing tests that build State for `HttpClient` only need one extra `app.manage(TraceIdGenerator)` in test setup if they exercise the shim; the http-level tests don't touch the shim at all).
2. `let _enter = span.enter();` runs **before** the `.await` so the span is entered on the right task; the guard is dropped at end-of-function, after all logging.
3. The `TRACE_ID.scope(...)` block is the only `.await` that crosses a task boundary in the body — the `match &result` block uses `&result` so we log without moving.

If a command returns immediately (e.g. `is_logged_in` which doesn't go through http), drop the `TRACE_ID.scope` block and call the http-less body directly inside the span:

```rust
#[tauri::command]
pub async fn is_logged_in(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
) -> Result<bool, ApiError> {
    let trace_id = generator.client_side();
    let span = tracing::info_span!("command", trace_id = %trace_id, command = "is_logged_in");
    let _enter = span.enter();
    tracing::info!("enter");

    let result = client.tokens().access_token().await.map(|opt| opt.is_some());

    match &result {
        Ok(_) => tracing::info!("success"),
        Err(e) => tracing::error!(error = %e, "failed"),
    }
    result
}
```

- [ ] **Step 2: Apply the wrap to `commands/auth.rs`**

The full wrapped file (one shim per command, follow the same shape as the templates above). The five commands are:

- `login(code, password)` → wraps `auth::login(&client, LoginRequest { code, password })`.
- `login_domain()` → wraps `auth::login_domain(&client)`.
- `is_logged_in()` → http-less variant (template above).
- `refresh()` → wraps `auth::refresh(&client)`.
- `logout()` → wraps `auth::logout(&client)`.

- [ ] **Step 3: Verify the wrap pattern works for `commands/auth.rs`**

Run: `cargo check -p aegis-desktop --lib commands::auth`
Expected: exit 0.

Run: `cargo test -p aegis-desktop --lib commands::auth`
Expected: existing tests still pass (the `login_persists_tokens`, `login_propagates_401`, `logout_clears_tokens`, `assert_login_domain_takes_only_the_client`, `login_domain_propagates_the_identity_error` tests do not touch the shim, so they remain green).

> **Why no per-shim captured-log test:** `tauri::State<'_, T>` exposes no public constructor in Tauri 2 (`State(&'r T)` with a private field). Constructing a `State` directly in a unit test requires either `tauri::test::mock_app()` (a full app + plugin chain — disproportionate) or splitting every shim into a thin Tauri wrapper around an inner pure fn that takes raw `&HttpClient` + `&TraceIdGenerator`. The wrap pattern is verified end-to-end by the manual smoke test in Task 8, and the wrap correctness is verified by `cargo check -p aegis-desktop --all-targets` (every shim follows the template).

- [ ] **Step 4: Apply the wrap to every other shim file**

For each of the following files, apply the same wrap to every `#[tauri::command]` function. The mechanical edit per command is exactly the template from Step 1.

| File | Commands |
|---|---|
| `commands/healthz.rs` | `healthz(client)` |
| `commands/identity.rs` | `get_domain_user_info` |
| `commands/mission.rs` | `list_missions_by_project`, `add_assignee`, `remove_assignee`, `create_mission`, `list_issues_by_mission`, `create_issue`, `patch_issue_state`, `update_issue_description`, `append_comment` |
| `commands/project.rs` | `create_project`, `list_projects`, `get_project_by_code`, `update_project` |
| `commands/user.rs` | `create_user`, `list_users`, `get_user_by_code`, `current_user`, `update_user` |
| `commands/user_credential.rs` | `register_user`, `update_user_credential` |
| `commands/crf/version.rs` | `list_crf_versions`, `import_als` |
| `commands/crf/form.rs` | `list_crf_forms_by_version`, `create_crf_form`, `update_crf_form`, `delete_crf_form`, `get_crf_form_by_id`, `get_crf_form_details`, `search_crf_forms_by_version`, `set_crf_form_approved` |
| `commands/crf/item.rs` | `list_crf_items_by_form`, `get_crf_item_by_id`, `update_crf_item`, `search_crf_items_by_version` |
| `commands/crf/option.rs` | `update_crf_option`, `get_crf_option_by_id`, `search_crf_options_by_version` |
| `commands/crf/unit.rs` | `update_crf_unit`, `get_crf_unit_by_id`, `search_crf_units_by_version` |
| `commands/crf/annotation.rs` | `create_crf_annotation`, `update_crf_annotation`, `delete_crf_annotation`, `search_crf_annotations_by_version` |
| `commands/crf/domain_annotation.rs` | `create_crf_domain_annotation`, `list_crf_domain_annotations_by_form`, `update_crf_domain_annotation`, `delete_crf_domain_annotation`, `search_crf_domain_annotations_by_version` |
| `commands/domain_model/version.rs` | `create_sdtm_version`, `list_sdtm_versions`, `get_sdtm_version_by_id`, `update_sdtm_version`, `delete_sdtm_version` |
| `commands/domain_model/domain.rs` | `create_sdtm_domain`, `list_sdtm_domains_by_version`, `get_sdtm_domain_by_id`, `update_sdtm_domain`, `delete_sdtm_domain` |
| `commands/domain_model/variable.rs` | `create_sdtm_variable`, `list_sdtm_variables_by_domain`, `get_sdtm_variable_by_id`, `update_sdtm_variable`, `delete_sdtm_variable` |
| `commands/terminology/version.rs` | `create_terminology_version`, `list_terminology_versions`, `get_terminology_version_by_id`, `update_terminology_version`, `delete_terminology_version` |
| `commands/terminology/code_list.rs` | `create_code_list`, `list_code_lists`, `get_code_list_by_id`, `update_code_list`, `delete_code_list` |
| `commands/terminology/code_item.rs` | `create_code_item`, `list_code_items`, `update_code_item`, `delete_code_item`, `list_code_items_by_version_and_code` |
| `commands/terminology/import.rs` | `import_terminology` |

The barrel files (`commands/crf.rs`, `commands/domain_model.rs`, `commands/terminology.rs`) need no edit.

- [ ] **Step 5: Verify everything compiles**

Run: `cargo check -p aegis-desktop --all-targets`
Expected: exit 0, no diagnostics.

- [ ] **Step 6: Verify the existing tests still pass**

Run: `cargo test -p aegis-desktop --lib`
Expected: all tests pass.

- [ ] **Step 7: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/commands/
git commit -m "feat(desktop): emit tracing enter/success/failed logs from every command

Every #[tauri::command] shim in commands/ now:
- pulls State<'_, TraceIdGenerator> alongside State<'_, HttpClient>
- mints a fresh trace id, opens an info_span! carrying trace_id
  + command name, logs enter on entry
- scopes the existing http fn call into TRACE_ID so HttpClient
  picks it up as X-Trace-ID
- logs success (info) or failed (error) before returning

http/* modules stay byte-identical; the only http edit was in
Task 4 + Task 5 (task_local + header + instrument).

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

## Task 8: Final verification gate

**Files:** none (verification only).

- [ ] **Step 1: Format check**

Run: `cargo fmt --all -- --check`
Expected: exit 0.

If a fix exists: `cargo fmt --all` then re-run the check.

- [ ] **Step 2: Clippy**

Run: `cargo clippy -p aegis-desktop --all-targets --all-features -- -D warnings`
Expected: exit 0, no diagnostics.

- [ ] **Step 3: Full test suite**

Run: `cargo test -p aegis-desktop`
Expected: all tests pass (unit + integration; no `--ignored` live-DB tests on this crate).

- [ ] **Step 4: Manual smoke test**

1. Build the bundled desktop app:
   ```bash
   pnpm --filter aegis-desktop tauri build
   ```
   (This compiles both the Vite frontend and the Tauri shell.)

2. Launch the built `.exe` (the path is printed at the end of the build; on Windows it's typically `apps/desktop/aegis-desktop/src-tauri/target/release/aegis-desktop.exe`).

3. Confirm the log file appears under the app-data directory (on Windows: `C:\Users\<user>\AppData\Roaming\com.yukichen.aegis-desktop\logs\aegis-desktop.log.<YYYY-MM-DD>`). Confirm `aegis-desktop-device-prefix` is a 21-char alphanumeric string.

4. Trigger one login attempt (intentionally wrong password) and a successful login, then a couple of read commands (e.g. list projects). Open the log file and verify:
   - One `enter` info event per command, with `command` = the shim name (e.g. `login`) and `trace_id` = `C-<device>-<ulid>`
   - One `success` or `failed` event after the http call completes
   - The `trace_id` matches the one in the outbound HTTP request's `X-Trace-ID` header (server-side, observable via the server's log file when both run together)

5. Confirm `<app_data_dir>/aegis-desktop-device-prefix` matches the middle segment of every `trace_id` in the log file.

- [ ] **Step 5: No final commit needed**

This task is verification-only. If any step surfaced a fix, commit that fix as a separate `fix(desktop): …` commit before reporting completion.