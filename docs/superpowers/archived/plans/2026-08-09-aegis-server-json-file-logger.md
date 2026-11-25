# aegis-server JSON File Logger Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the text-formatted, stdout-only `tracing` setup in `apps/server/aegis-server` with a JSON-formatted, file-based logger that writes daily-rotated logs into the directory named by `AEGIS_LOG_DIR` and uses `AEGIS_LOG_LEVEL` (default `info`) for the global filter.

**Architecture:** The current `init_tracing` in [run.rs:84-91](../../apps/server/aegis-server/src/run.rs#L84-L91) builds a `tracing_subscriber::fmt()` text subscriber and ignores file output. The replacement uses `tracing_subscriber::fmt::SubscriberBuilder::json()` (the `json` feature is already enabled on the workspace dep) for structured output, plus `tracing_appender::rolling::daily` for daily file rotation. The non-blocking writer's `WorkerGuard` is returned from `init_tracing` and held by `run` for the duration of the program so buffered writes are flushed on shutdown.

**Tech Stack:** Rust 2024, `tracing 0.1`, `tracing-subscriber 0.3` (with `env-filter` + `json`), `tracing-appender 0.2` (already in workspace deps).

---

## Global Constraints

- Every dep in `apps/server/aegis-server/Cargo.toml` is either a workspace dep or a path-dep — no direct version pinning. `tracing-appender` is added via `workspace = true`.
- The function signature change (`init_tracing` returns a `WorkerGuard`) is internal — the public surface of the crate (`run`, `Config`, `AppState`, `router`) is unchanged.
- `try_init` swallows the "already initialized" error so the test that calls `init_tracing` twice still passes.
- `RUST_LOG` is intentionally not honored in the new path; the spec calls for `AEGIS_LOG_LEVEL` as the explicit knob.
- `tracing-subscriber`'s `json` feature is already on in [Cargo.toml:32](../../Cargo.toml#L32); no workspace change needed.
- Commit messages follow the project's existing convention (`feat(aegis-server):`, `test(aegis-server):`, `docs(aegis-server):`).

---

## Task 1: Add `tracing-appender` direct dep to aegis-server

**Files:**
- Modify: `apps/server/aegis-server/Cargo.toml:30-31`

- [ ] **Step 1: Add the dependency**

In `apps/server/aegis-server/Cargo.toml`, after the `tracing-subscriber` line (currently line 31), add:

```toml
# tracing-appender provides rolling file appenders (used by
# run::init_tracing to write daily JSON logs into $AEGIS_LOG_DIR).
tracing-appender = { workspace = true }
```

The full block around the new dep should read:

```toml
# tracing + tracing-subscriber log every request via TraceLayer; the
# subscriber is initialized in main.rs.
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
# tracing-appender provides rolling file appenders (used by
# run::init_tracing to write daily JSON logs into $AEGIS_LOG_DIR).
tracing-appender = { workspace = true }
```

- [ ] **Step 2: Verify the workspace resolves**

Run: `cargo check -p aegis-server`
Expected: completes with no errors (warnings are OK). Confirms `tracing-appender` resolves through the workspace.

- [ ] **Step 3: Commit**

```bash
git add apps/server/aegis-server/Cargo.toml
git commit -m "feat(aegis-server): add tracing-appender direct dep for file logging"
```

---

## Task 2: Refactor `init_tracing` to JSON + file output

**Files:**
- Modify: `apps/server/aegis-server/src/run.rs:82-91`

- [ ] **Step 1: Write the failing test for the new env-var behavior**

In `apps/server/aegis-server/src/run.rs`, replace the existing `#[cfg(test)] mod tests { … }` block (currently lines 143-154) with the following test module. Use the same `ENV_LOCK` + `EnvGuard` pattern from `config.rs:118-155` (duplicate them inline — there's no shared test utility in this crate):

```rust
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
    fn init_tracing_is_idempotent() {
        // Calling `init_tracing` twice shouldn't panic — the
        // `try_init` path swallows the "already initialized" error.
        // AEGIS_LOG_DIR is pointed at a temp dir so the file appender
        // does not write into the repo's working tree.
        let tmp = std::env::temp_dir().join("aegis-server-logger-test-idempotent");
        let _ = std::fs::create_dir_all(&tmp);
        let _g = lock_env();
        let _dir = set_env("AEGIS_LOG_DIR", tmp.to_str().unwrap());
        let _lvl = set_env("AEGIS_LOG_LEVEL", "info");
        let _a = init_tracing();
        let _b = init_tracing();
    }

    #[test]
    fn init_tracing_defaults_level_to_info_when_env_missing() {
        let _g = lock_env();
        // SAFETY: serialized via ENV_LOCK.
        unsafe { std::env::remove_var("AEGIS_LOG_LEVEL"); }
        let filter = build_env_filter();
        // The default directive ("info") is present somewhere in the
        // directive list, and the filter is parseable.
        assert_eq!(filter.to_string(), "info");
    }

    #[test]
    fn init_tracing_uses_aegis_log_level_when_set() {
        let _g = lock_env();
        let _lvl = set_env("AEGIS_LOG_LEVEL", "debug");
        let filter = build_env_filter();
        assert_eq!(filter.to_string(), "debug");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail to compile**

Run: `cargo test -p aegis-server --lib run::tests::`
Expected: compile error — `build_env_filter` is not defined and `init_tracing` does not return a `WorkerGuard`. This is the failing-test state we want.

- [ ] **Step 3: Replace `init_tracing` with the new JSON + file implementation**

In `apps/server/aegis-server/src/run.rs`, replace lines 82-91 (the `init_tracing` function and its doc comment) with:

```rust
/// Build the global `EnvFilter` from `AEGIS_LOG_LEVEL`. Defaults to
/// `info` when the variable is unset. The previous `RUST_LOG` escape
/// hatch is intentionally dropped — `AEGIS_LOG_LEVEL` is the only
/// knob documented in `.env`.
fn build_env_filter() -> EnvFilter {
    let level = std::env::var("AEGIS_LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
    EnvFilter::new(level)
}

/// Initialise tracing. Writes JSON-formatted events to a daily
/// rotating file under `$AEGIS_LOG_DIR` (defaults to `./logs` if
/// unset, which only happens in tests). Returns the
/// `WorkerGuard` for the non-blocking writer — it MUST be held for
/// the lifetime of the program or the buffered writes are lost on
/// shutdown. The returned guard is `let _guard = …;` in [`run`].
///
/// `try_init` swallows the "already initialized" error, so calling
/// `init_tracing` more than once is a no-op (not a panic).
fn init_tracing() -> tracing_appender::non_blocking::WorkerGuard {
    let dir = std::env::var("AEGIS_LOG_DIR").unwrap_or_else(|_| "./logs".to_string());
    let filter = build_env_filter();

    // Daily rotation produces files named
    // `{prefix}.YYYY-MM-DD` (e.g. `aegis-server.log.2026-08-09`).
    let file_appender = tracing_appender::rolling::daily(&dir, "aegis-server.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .json()
        .with_current_span(true)
        .with_span_list(false)
        .with_writer(non_blocking)
        .try_init();

    guard
}
```

- [ ] **Step 4: Bind the guard in `run`**

In `apps/server/aegis-server/src/run.rs`, change line 35 from:

```rust
    init_tracing();
```

to:

```rust
    let _log_guard = init_tracing();
```

The `_` prefix documents that we deliberately hold the guard for its
`Drop` side effect; the value itself is unused.

- [ ] **Step 5: Run the new tests**

Run: `cargo test -p aegis-server --lib run::tests::`
Expected: all three tests pass.

- [ ] **Step 6: Run the full test suite for the crate**

Run: `cargo test -p aegis-server --lib`
Expected: every existing test in `config`, `run`, `transport`, etc. continues to pass. The integration test in `tests/integration_auth.rs` is `#[ignore]`-d and not run by default — do NOT pass `-- --ignored` here.

- [ ] **Step 7: Commit**

```bash
git add apps/server/aegis-server/src/run.rs
git commit -m "feat(aegis-server): emit JSON logs to daily files in AEGIS_LOG_DIR"
```

---

## Task 3: Sanity-check the runtime behavior end-to-end

**Files:**
- Read-only: `apps/server/aegis-server/Cargo.toml`, `apps/server/aegis-server/src/run.rs`, `.env`

- [ ] **Step 1: Build the release binary**

Run: `cargo build -p aegis-server`
Expected: builds with no errors. Warnings are acceptable (the `_` on the guard binding is intentional).

- [ ] **Step 2: Run the binary briefly and confirm a log file appears**

Run from the repo root:

```bash
mkdir -p .data/logs/aegis-server
AEGIS_DATABASE_URL=postgres://localhost/aegis \
AEGIS_AUTH_SIGNING_KEY=06e5f77baee959b0f5c25d3ee7be811846441841e3d07f39166211bca2331296 \
AEGIS_LOG_DIR=./.data/logs/aegis-server \
AEGIS_LOG_LEVEL=info \
timeout 2 cargo run -p aegis-server 2>/dev/null; true
```

Then:

```bash
ls -1 .data/logs/aegis-server/
```

Expected: a file named `aegis-server.log.YYYY-MM-DD` exists, where `YYYY-MM-DD` is today's date (2026-08-09, so `aegis-server.log.2026-08-09`).

- [ ] **Step 3: Confirm the file contains valid JSON**

Run:

```bash
head -n 1 .data/logs/aegis-server/aegis-server.log.2026-08-09 | python3 -c "import json,sys; json.loads(sys.stdin.read()); print('ok')"
```

Expected: prints `ok`. (The very first line may be the "aegis-server listening" event after the Postgres pool connects; if Postgres is unreachable, the binary errors before writing — that's also fine, the test in Task 2 still passed.)

- [ ] **Step 4: Clean up the test log directory**

Run:

```bash
rm -rf .data/logs/aegis-server/aegis-server.log.2026-08-09
```

(`rm` here only deletes the single test file; the parent directory is left intact so the next run lands in the same place.)

- [ ] **Step 5: No commit**

This task is verification-only — there is no source change to commit.

---

## Self-Review

1. **Spec coverage:**
   - JSON format → Task 2 Step 3 (`.json()`).
   - Save into `AEGIS_LOG_DIR` → Task 2 Step 3 (`rolling::daily(&dir, …)`).
   - Default `info`, configurable via `AEGIS_LOG_LEVEL` → Task 2 Step 3 + the new `build_env_filter` test pair in Task 2 Step 1.
2. **Placeholder scan:** No "TBD"/"fill in later"/etc. — every code block is complete.
3. **Type consistency:** `init_tracing` returns `WorkerGuard` (Task 2 Step 3) and `run` binds it as `_log_guard` (Task 2 Step 4). Tests call `init_tracing()` and discard the guard with `let _a = …;` (Task 2 Step 1).
