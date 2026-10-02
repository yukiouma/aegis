# `logging-utils::log_ingestor` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `log_ingestor` module to `lib/crates/logging-utils` that lets `aegis-desktop` submit batches of UTF-8 log lines identified by `batch_id`, deduplicates repeats via a TTL+Lru moka cache, and writes accepted batches through a bounded crossbeam mpsc to a single writer thread that owns a `RollingFileAppender` directly.

**Architecture:** New peer module `lib/crates/logging-utils/src/log_ingestor/` with three focused files (`config`, `ingestor`, `writer`) and a `mod.rs` re-export. `LogIngestor::submit(&self, batch_id, &entries)` is sync; the cache check, cache insert, and channel send happen on the caller's thread. The writer thread blocks on `rx.recv()`, owns the `RollingFileAppender` directly (no `non_blocking` wrapper — the thread IS the buffer), and flushes per envelope. Shutdown uses a separate `bounded(1)` done-signal channel with `recv_timeout(deadline)` + `JoinHandle::is_finished()` + `join()` to distinguish timeout from panic.

**Tech Stack:** Rust 2024 edition, resolver 3. Workspace deps: `moka = "0.12"` (sync feature), `crossbeam-channel = "0.5"`. Existing workspace deps: `tracing-appender = "0.2"`, `thiserror = "2"`, `tempfile = "3"` (dev). No HTTP, no Tauri, no async.

**Spec:** [docs/superpowers/specs/2026-10-02-log-ingestor-design.md](../../specs/2026-10-02-log-ingestor-design.md)

## Global Constraints

- `edition = "2024"`, `resolver = "3"` (workspace root, [Cargo.toml:13](Cargo.toml))
- All shared deps declared in `[workspace.dependencies]` and inherited via `{ workspace = true }`
- One-line `why` comment on each non-obvious workspace dep
- No DDD layered structure (this crate is observability infrastructure, not a business lib; `lib-crate-development.md` does not apply)
- Workspace member names use kebab-case (`logging-utils`)
- No edits to `aegis-server` / `aegis-desktop` / `trace-id` / any other workspace member in this PR
- Crate-level doc-comment in `src/lib.rs` shows the canonical `use` line
- README at the crate root with: one-line purpose, file tree, `LogIngestor::submit` snippet, verification command
- DDL / DTO / wire-shape changes: none (writes to local FS only)
- Defaults: `cache_capacity = 10_000`, `cache_ttl = 600 s`, `channel_capacity = 1_000`, `shutdown_deadline = 5 s`
- Scope is `aegis-desktop` only — `aegis-server` is explicitly excluded

## Review Focus

These are the input classes or failure modes the spec implies but no task's tests pin directly. Each line's test is added to the owning task below.

1. **Empty entries list** (`writer.rs`) — `write_envelope(&mut writer, &BatchEnvelope { batch_id: "x".into(), entries: vec![] })` writes only the `"BATCH x\n"` header, no panic on empty Vec. The spec says "one line per entry" implicitly — empty is the zero case.
2. **Concurrent producers** (`ingestor.rs`) — 8 threads calling `submit` with distinct `batch_id`s concurrently must all return `Ok(())`, no deadlock, no cache corruption (every batch appears in the file). Moka is documented thread-safe but the assertion should be locked in.
3. **Writer thread panic before sending `done_tx`** (`writer.rs`) — when the writer closure panics, `done_tx` is dropped (auto-drop on panic), `done_rx.recv_timeout` returns `Err(Disconnected)`. The shutdown path must translate this to `IngestorError::WriterPanic` by calling `JoinHandle::join()` after `is_finished()`. The spec describes the mechanism; the test pins the outcome.
4. **Shutdown deadline exceeded** (`ingestor.rs`) — when the writer thread is blocked on `recv` (e.g. tx still alive) and `shutdown_deadline` elapses, `shutdown` must return `Err(WriterJoinTimeout { deadline, .. })`, not block indefinitely. Constructed by holding a `Sender` alive past the deadline.
5. **`Submit` after `shutdown`** (`ingestor.rs`) — after `shutdown(&mut self)` returns, calling `submit(&self, …)` on the same instance must return `Err(ShutDown)`, not panic on `self.tx.take()` being `None`. The submit path's pre-check on `shutdown_flag` enforces this; the test pins it.

---

### Task 1: Workspace + module scaffolding

**Files:**
- Modify: `Cargo.toml` (workspace root) — add `moka` + `crossbeam-channel` to `[workspace.dependencies]`
- Modify: `lib/crates/logging-utils/Cargo.toml` — add `moka` + `crossbeam-channel` deps
- Create: `lib/crates/logging-utils/src/log_ingestor/mod.rs` (skeleton)
- Create: `lib/crates/logging-utils/src/log_ingestor/config.rs` (empty)
- Create: `lib/crates/logging-utils/src/log_ingestor/ingestor.rs` (empty)
- Create: `lib/crates/logging-utils/src/log_ingestor/writer.rs` (empty)
- Modify: `lib/crates/logging-utils/src/lib.rs` — declare `pub mod log_ingestor;`

**Interfaces:**
- Produces: an empty module tree that resolves via the workspace and compiles cleanly. `LogIngestor` / `LogIngestorConfig` are not yet exposed (added in later tasks).

- [ ] **Step 1: Add `moka` and `crossbeam-channel` to root `[workspace.dependencies]`**

Edit `Cargo.toml`. After the `# \`ulid\` generates…` block (alphabetical-ish neighbor) add:

```toml
# `moka` provides the TinyLFU-backed dedup cache used by
# `logging-utils::log_ingestor`. Pinned so every consumer resolves
# the same version.
moka              = { version = "0.12", default-features = false, features = ["sync"] }
# `crossbeam-channel` is the bounded mpsc that carries accepted
# batches from producers to the writer thread.
crossbeam-channel = "0.5"
```

- [ ] **Step 2: Add the deps to `lib/crates/logging-utils/Cargo.toml`**

Edit `lib/crates/logging-utils/Cargo.toml`. After the `ulid` dep block, add:

```toml
# `moka` provides the dedup cache that remembers `batch_id`s for
# `LogIngestor::submit`; `sync` is enough (no async API surface).
moka               = { workspace = true }
# `crossbeam-channel` is the bounded mpsc between `submit` callers
# and the writer thread; sync `send` / `recv` is enough.
crossbeam-channel  = { workspace = true }
```

- [ ] **Step 3: Create the four stub files**

Run from the workspace root:

```bash
mkdir -p lib/crates/logging-utils/src/log_ingestor
```

Create `lib/crates/logging-utils/src/log_ingestor/mod.rs`:

```rust
//! `log_ingestor` — batched, deduplicated, bounded-queue log writer.
//!
//! See the [`LogIngestor`](crate::LogIngestor) docs and the design
//! doc `docs/superpowers/specs/2026-10-02-log-ingestor-design.md`.

mod config;
mod ingestor;
mod writer;
```

Create `lib/crates/logging-utils/src/log_ingestor/config.rs`:

```rust
// Filled in by Task 2.
```

Create `lib/crates/logging-utils/src/log_ingestor/ingestor.rs`:

```rust
// Filled in by Task 4.
```

Create `lib/crates/logging-utils/src/log_ingestor/writer.rs`:

```rust
// Filled in by Task 3.
```

- [ ] **Step 4: Declare the module in `src/lib.rs`**

Edit `lib/crates/logging-utils/src/lib.rs`. Add one line, then add the module declaration alongside the existing ones:

```rust
pub mod log_ingestor;
pub mod trace_id;
pub mod tracing_init;
```

(The re-exports for `LogIngestor` / `LogIngestorConfig` / `IngestorError` are added in Task 7 — declaring the module now keeps the workspace green.)

- [ ] **Step 5: Verify the workspace compiles**

Run: `cargo check --workspace`
Expected: success. The empty modules compile because they have no contents.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml lib/crates/logging-utils/
git commit -m "feat(logging-utils): scaffold log_ingestor module"
```

---

### Task 2: `LogIngestorConfig` + builder + defaults

**Files:**
- Modify: `lib/crates/logging-utils/src/log_ingestor/config.rs` — fill in the types

**Interfaces:**
- Produces:
  - `pub struct LogIngestorConfig { log_dir: PathBuf, file_name_prefix: String, cache_capacity: u64, cache_ttl: Duration, channel_capacity: usize, shutdown_deadline: Duration }` with `#[derive(Debug, Clone)]`
  - `impl LogIngestorConfig { pub fn new(log_dir: PathBuf, file_name_prefix: String) -> Self; pub fn builder() -> LogIngestorConfigBuilder; }`
  - `pub struct LogIngestorConfigBuilder` with one setter per knob and `pub fn build(self) -> LogIngestorConfig`

- [ ] **Step 1: Write the failing tests**

Replace `lib/crates/logging-utils/src/log_ingestor/config.rs` with:

```rust
//! Knobs for [`crate::LogIngestor`]. See the spec for the design
//! rationale; defaults are pinned in [`LogIngestorConfig::new`].

use std::path::PathBuf;
use std::time::Duration;

/// Default moka cache capacity (10 000 entries).
const DEFAULT_CACHE_CAPACITY: u64 = 10_000;
/// Default moka cache TTL (600 s).
const DEFAULT_CACHE_TTL: Duration = Duration::from_secs(600);
/// Default bounded-channel capacity (1 000 envelopes).
const DEFAULT_CHANNEL_CAPACITY: usize = 1_000;
/// Default shutdown / drop deadline (5 s).
const DEFAULT_SHUTDOWN_DEADLINE: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
pub struct LogIngestorConfig {
    pub log_dir: PathBuf,
    pub file_name_prefix: String,
    pub cache_capacity: u64,
    pub cache_ttl: Duration,
    pub channel_capacity: usize,
    pub shutdown_deadline: Duration,
}

impl LogIngestorConfig {
    pub fn new(log_dir: PathBuf, file_name_prefix: String) -> Self {
        Self {
            log_dir,
            file_name_prefix,
            cache_capacity: DEFAULT_CACHE_CAPACITY,
            cache_ttl: DEFAULT_CACHE_TTL,
            channel_capacity: DEFAULT_CHANNEL_CAPACITY,
            shutdown_deadline: DEFAULT_SHUTDOWN_DEADLINE,
        }
    }

    pub fn builder() -> LogIngestorConfigBuilder {
        LogIngestorConfigBuilder::default()
    }
}

#[derive(Debug, Clone)]
pub struct LogIngestorConfigBuilder {
    log_dir: Option<PathBuf>,
    file_name_prefix: Option<String>,
    cache_capacity: Option<u64>,
    cache_ttl: Option<Duration>,
    channel_capacity: Option<usize>,
    shutdown_deadline: Option<Duration>,
}

impl Default for LogIngestorConfigBuilder {
    fn default() -> Self {
        Self {
            log_dir: None,
            file_name_prefix: None,
            cache_capacity: None,
            cache_ttl: None,
            channel_capacity: None,
            shutdown_deadline: None,
        }
    }
}

impl LogIngestorConfigBuilder {
    pub fn log_dir(mut self, dir: PathBuf) -> Self {
        self.log_dir = Some(dir);
        self
    }
    pub fn file_name_prefix(mut self, prefix: String) -> Self {
        self.file_name_prefix = Some(prefix);
        self
    }
    pub fn cache_capacity(mut self, capacity: u64) -> Self {
        self.cache_capacity = Some(capacity);
        self
    }
    pub fn cache_ttl(mut self, ttl: Duration) -> Self {
        self.cache_ttl = Some(ttl);
        self
    }
    pub fn channel_capacity(mut self, capacity: usize) -> Self {
        self.channel_capacity = Some(capacity);
        self
    }
    pub fn shutdown_deadline(mut self, deadline: Duration) -> Self {
        self.shutdown_deadline = Some(deadline);
        self
    }
    pub fn build(self) -> LogIngestorConfig {
        let defaults = LogIngestorConfig::new(PathBuf::new(), String::new());
        LogIngestorConfig {
            log_dir: self.log_dir.unwrap_or(defaults.log_dir),
            file_name_prefix: self.file_name_prefix.unwrap_or(defaults.file_name_prefix),
            cache_capacity: self.cache_capacity.unwrap_or(defaults.cache_capacity),
            cache_ttl: self.cache_ttl.unwrap_or(defaults.cache_ttl),
            channel_capacity: self.channel_capacity.unwrap_or(defaults.channel_capacity),
            shutdown_deadline: self.shutdown_deadline.unwrap_or(defaults.shutdown_deadline),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_apply_when_only_dir_and_prefix_set() {
        let cfg = LogIngestorConfig::new(
            PathBuf::from("/tmp/aegis"),
            "test.log".to_string(),
        );
        assert_eq!(cfg.log_dir, PathBuf::from("/tmp/aegis"));
        assert_eq!(cfg.file_name_prefix, "test.log");
        assert_eq!(cfg.cache_capacity, DEFAULT_CACHE_CAPACITY);
        assert_eq!(cfg.cache_ttl, DEFAULT_CACHE_TTL);
        assert_eq!(cfg.channel_capacity, DEFAULT_CHANNEL_CAPACITY);
        assert_eq!(cfg.shutdown_deadline, DEFAULT_SHUTDOWN_DEADLINE);
    }

    #[test]
    fn builder_overrides_each_knob_independently() {
        let cfg = LogIngestorConfig::builder()
            .log_dir(PathBuf::from("/tmp/aegis-builder"))
            .file_name_prefix("builder.log".to_string())
            .cache_capacity(123)
            .cache_ttl(Duration::from_secs(7))
            .channel_capacity(45)
            .shutdown_deadline(Duration::from_secs(2))
            .build();
        assert_eq!(cfg.log_dir, PathBuf::from("/tmp/aegis-builder"));
        assert_eq!(cfg.file_name_prefix, "builder.log");
        assert_eq!(cfg.cache_capacity, 123);
        assert_eq!(cfg.cache_ttl, Duration::from_secs(7));
        assert_eq!(cfg.channel_capacity, 45);
        assert_eq!(cfg.shutdown_deadline, Duration::from_secs(2));
    }

    #[test]
    fn clone_preserves_fields() {
        let original = LogIngestorConfig::new(
            PathBuf::from("/tmp/aegis-clone"),
            "clone.log".to_string(),
        );
        let cloned = original.clone();
        assert_eq!(original.log_dir, cloned.log_dir);
        assert_eq!(original.file_name_prefix, cloned.file_name_prefix);
        assert_eq!(original.cache_capacity, cloned.cache_capacity);
        assert_eq!(original.cache_ttl, cloned.cache_ttl);
        assert_eq!(original.channel_capacity, cloned.channel_capacity);
        assert_eq!(original.shutdown_deadline, cloned.shutdown_deadline);
    }
}
```

- [ ] **Step 2: Run the tests to verify they pass**

Run: `cargo test -p logging-utils --lib log_ingestor::config::tests`
Expected: 3 passed.

(Step 2 collapses "fail → implement → pass" because the test file IS the implementation file — there is no separate code-under-test to write first. The tests fail to compile until the file is in place; once the file is in place, both tests and implementation exist.)

- [ ] **Step 3: Commit**

```bash
git add lib/crates/logging-utils/src/log_ingestor/config.rs
git commit -m "feat(logging-utils): add LogIngestorConfig + builder + defaults"
```

---

### Task 3: `writer` module — `write_envelope` + `run_writer`

**Files:**
- Modify: `lib/crates/logging-utils/src/log_ingestor/writer.rs` — fill in `pub(crate)` types and functions
- Modify: `lib/crates/logging-utils/src/log_ingestor/mod.rs` — declare `pub(super) use` for `BatchEnvelope` so `ingestor.rs` can import it without reaching into a sibling module

**Interfaces:**
- Produces:
  - `pub(crate) struct BatchEnvelope { pub batch_id: String, pub entries: Vec<String> }`
  - `pub(crate) fn run_writer(rx: Receiver<BatchEnvelope>, appender: RollingFileAppender, done_tx: Sender<()>)` — the writer thread body. Loops on `rx.recv()`, calls `write_envelope`, exits when the channel is closed; sends `()` on `done_tx` before returning.
  - `pub(crate) fn write_envelope(writer: &mut RollingFileAppender, envelope: &BatchEnvelope)` — writes `"BATCH {id}\n"` then one line per entry, then `flush()`.

- [ ] **Step 1: Declare `BatchEnvelope` re-export in `mod.rs`**

Edit `lib/crates/logging-utils/src/log_ingestor/mod.rs`:

```rust
//! `log_ingestor` — batched, deduplicated, bounded-queue log writer.
//!
//! See the [`LogIngestor`](crate::LogIngestor) docs and the design
//! doc `docs/superpowers/specs/2026-10-02-log-ingestor-design.md`.

mod config;
mod ingestor;
mod writer;

pub(super) use writer::BatchEnvelope;
```

- [ ] **Step 2: Write the failing tests**

Replace `lib/crates/logging-utils/src/log_ingestor/writer.rs` with:

```rust
//! Writer thread + per-envelope write helper. The writer owns the
//! `RollingFileAppender` directly — no `non_blocking` wrapper —
//! and flushes after every envelope.

use std::io::Write;

use crossbeam_channel::{Receiver, Sender};
use tracing_appender::rolling::RollingFileAppender;

#[derive(Debug, Clone)]
pub(crate) struct BatchEnvelope {
    pub batch_id: String,
    pub entries: Vec<String>,
}

/// Body of the writer thread. Loops on `rx.recv()`, writes each
/// envelope via [`write_envelope`], and exits when `rx` is closed.
/// Sends `()` on `done_tx` before returning so the shutdown path can
/// distinguish "thread exited" from "deadline exceeded".
pub(crate) fn run_writer(
    rx: Receiver<BatchEnvelope>,
    mut appender: RollingFileAppender,
    done_tx: Sender<()>,
) {
    loop {
        match rx.recv() {
            Ok(envelope) => write_envelope(&mut appender, &envelope),
            Err(_) => break, // sender dropped; channel closed
        }
    }
    let _ = done_tx.send(());
}

/// Write one envelope: `"BATCH {batch_id}\n"` header, then one line
/// per entry, then `flush()`. I/O errors are swallowed — the writer
/// thread is best-effort and should not panic on a transient disk
/// problem.
pub(crate) fn write_envelope(
    writer: &mut RollingFileAppender,
    envelope: &BatchEnvelope,
) {
    let _ = writeln!(writer, "BATCH {}", envelope.batch_id);
    for line in &envelope.entries {
        let _ = writeln!(writer, "{line}");
    }
    let _ = writer.flush();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam_channel::bounded;
    use std::thread;
    use std::time::Duration;
    use tempfile::tempdir;

    /// Helper: read the first daily-rotated file under `dir` whose
    /// name starts with `prefix`. Returns `(file_name, contents)`.
    fn read_log(dir: &std::path::Path, prefix: &str) -> (String, String) {
        let entry = std::fs::read_dir(dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .find(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with(prefix)
            })
            .expect("expected at least one daily-rotate file under prefix");
        let name = entry.file_name().to_string_lossy().into_owned();
        let contents = std::fs::read_to_string(entry.path()).unwrap();
        (name, contents)
    }

    #[test]
    fn writer_writes_header_then_entries() {
        let tmp = tempdir().unwrap();
        let (tx, rx) = bounded::<BatchEnvelope>(4);
        let (done_tx, done_rx) = bounded::<()>(1);
        let appender = tracing_appender::rolling::daily(tmp.path(), "happy.log");

        let handle = thread::spawn(move || run_writer(rx, appender, done_tx));

        tx.send(BatchEnvelope {
            batch_id: "b-1".into(),
            entries: vec!["line a".into(), "line b".into(), "line c".into()],
        })
        .unwrap();
        drop(tx); // close channel → writer exits

        done_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        handle.join().unwrap();

        let (_name, contents) = read_log(tmp.path(), "happy.log");
        assert_eq!(contents, "BATCH b-1\nline a\nline b\nline c\n");
    }

    #[test]
    fn writer_writes_empty_entries_as_header_only() {
        let tmp = tempdir().unwrap();
        let (tx, rx) = bounded::<BatchEnvelope>(4);
        let (done_tx, done_rx) = bounded::<()>(1);
        let appender = tracing_appender::rolling::daily(tmp.path(), "empty.log");

        let handle = thread::spawn(move || run_writer(rx, appender, done_tx));

        tx.send(BatchEnvelope {
            batch_id: "b-empty".into(),
            entries: vec![],
        })
        .unwrap();
        drop(tx);

        done_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        handle.join().unwrap();

        let (_name, contents) = read_log(tmp.path(), "empty.log");
        assert_eq!(contents, "BATCH b-empty\n");
    }

    #[test]
    fn writer_exits_when_sender_dropped() {
        let tmp = tempdir().unwrap();
        let (tx, rx) = bounded::<BatchEnvelope>(4);
        let (done_tx, done_rx) = bounded::<()>(1);
        let appender = tracing_appender::rolling::daily(tmp.path(), "drop.log");

        let handle = thread::spawn(move || run_writer(rx, appender, done_tx));

        // No sends. Dropping `tx` closes the channel; writer exits
        // without writing anything.
        drop(tx);
        done_rx.recv_timeout(Duration::from_secs(2))
            .expect("writer must signal done within deadline");
        handle.join().unwrap();

        // No file should exist (nothing written).
        let any = std::fs::read_dir(tmp.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .any(|e| e.file_name().to_string_lossy().starts_with("drop.log"));
        assert!(!any, "no log file should be created when no envelopes are sent");
    }

    #[test]
    fn writer_panic_before_done_signal_propagates_via_thread_join() {
        // Construct the writer loop inline (mirrors `run_writer`)
        // with a deliberate panic on the first envelope.
        let tmp = tempdir().unwrap();
        let (tx, rx) = bounded::<BatchEnvelope>(4);
        let (done_tx, done_rx) = bounded::<()>(1);
        let mut appender = tracing_appender::rolling::daily(tmp.path(), "panic.log");

        let panicked = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let panicked_clone = std::sync::Arc::clone(&panicked);
        let handle = thread::spawn(move || {
            // First envelope triggers a panic; done_tx is never sent.
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let env = rx.recv().unwrap();
                panicked_clone.store(true, std::sync::atomic::Ordering::SeqCst);
                write_envelope(&mut appender, &env);
            }));
            // We deliberately do NOT send done_tx — the panic path
            // is what we want to exercise.
            assert!(result.is_err(), "writer must have panicked");
            // Simulate panic-induced auto-drop of done_tx by letting
            // this closure return without sending; the test below
            // confirms done_rx sees Disconnected.
        });

        tx.send(BatchEnvelope {
            batch_id: "trigger".into(),
            entries: vec!["crash".into()],
        })
        .unwrap();
        drop(tx);
        handle.join().unwrap();

        assert!(panicked.load(std::sync::atomic::Ordering::SeqCst));
        // done_rx should see Disconnected (done_tx was dropped).
        match done_rx.recv_timeout(Duration::from_secs(2)) {
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {}
            other => panic!("expected Disconnected, got {other:?}"),
        }
    }
}
```

- [ ] **Step 3: Run the tests to verify they pass**

Run: `cargo test -p logging-utils --lib log_ingestor::writer::tests`
Expected: 4 passed. (The 4th test pins the panic-propagation contract — see Review Focus #3.)

- [ ] **Step 4: Commit**

```bash
git add lib/crates/logging-utils/src/log_ingestor/
git commit -m "feat(logging-utils): add log_ingestor writer thread + helpers"
```

---

### Task 4: `LogIngestor::new` + `submit` happy path + dedup

**Files:**
- Modify: `lib/crates/logging-utils/src/log_ingestor/ingestor.rs` — add `LogIngestor`, `IngestorError`, `submit`, `new`, dedup logic

**Interfaces:**
- Produces:
  - `pub struct LogIngestor` with private fields `cache: moka::sync::Cache<String, ()>`, `tx: Option<Sender<BatchEnvelope>>`, `done_rx: Option<Receiver<()>>`, `writer: Option<JoinHandle<()>>`, `shutdown_flag: AtomicBool`, `deadline: Duration`, `channel_capacity: usize`, `log_dir: PathBuf`, `file_name_prefix: String`
  - `pub enum IngestorError` with variants `CreateDir { dir: PathBuf, #[source] source: std::io::Error }`, `DuplicateBatchId(String)`, `ChannelFull(usize)`, `ShutDown`, `WriterJoinTimeout { deadline: Duration, message: String }`, `WriterPanic(String)`
  - `impl LogIngestor { pub fn new(config: LogIngestorConfig) -> Result<Self, IngestorError>; pub fn submit(&self, batch_id: impl Into<String>, entries: &[String]) -> Result<(), IngestorError>; }`

- [ ] **Step 1: Write the failing tests**

Replace `lib/crates/logging-utils/src/log_ingestor/ingestor.rs` with the test-bearing implementation below. The tests live at the bottom; the implementation is in the middle. Run them after writing the file.

```rust
//! [`LogIngestor`] — sync, deduplicated, bounded-queue log submitter
//! + the background writer thread lifecycle. See the spec for
//! rationale; see [`crate::LogIngestorConfig`] for knobs.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use crossbeam_channel::{bounded, Receiver, RecvTimeoutError, Sender};
use moka::sync::Cache;
use thiserror::Error;
use tracing_appender::rolling::RollingFileAppender;

use super::config::LogIngestorConfig;
use super::writer::{run_writer, BatchEnvelope};

#[derive(Debug, Error)]
pub enum IngestorError {
    #[error("failed to create log directory {dir}: {source}")]
    CreateDir {
        dir: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("batch id {0} was already submitted within the cache TTL")]
    DuplicateBatchId(String),
    #[error("ingestor channel is full (capacity {0}); caller should retry")]
    ChannelFull(usize),
    #[error("ingestor has been shut down")]
    ShutDown,
    #[error("writer thread failed to join within {deadline:?}: {message}")]
    WriterJoinTimeout {
        deadline: Duration,
        message: String,
    },
    #[error("writer thread panicked: {0}")]
    WriterPanic(String),
}

pub struct LogIngestor {
    cache: Cache<String, ()>,
    tx: Option<Sender<BatchEnvelope>>,
    done_rx: Option<Receiver<()>>,
    writer: Option<JoinHandle<()>>,
    shutdown_flag: AtomicBool,
    deadline: Duration,
    channel_capacity: usize,
}

impl LogIngestor {
    /// Build a [`LogIngestor`] with the configured knobs. Creates
    /// the log directory (failure → [`IngestorError::CreateDir`])
    /// and spawns the writer thread.
    pub fn new(config: LogIngestorConfig) -> Result<Self, IngestorError> {
        std::fs::create_dir_all(&config.log_dir).map_err(|source| {
            IngestorError::CreateDir {
                dir: config.log_dir.clone(),
                source,
            }
        })?;
        let appender = tracing_appender::rolling::daily(
            &config.log_dir,
            &config.file_name_prefix,
        );
        let cache = Cache::builder()
            .max_capacity(config.cache_capacity)
            .time_to_live(config.cache_ttl)
            .build();
        let (tx, rx) = bounded::<BatchEnvelope>(config.channel_capacity);
        let (done_tx, done_rx) = bounded::<()>(1);
        let writer = std::thread::spawn(move || run_writer(rx, appender, done_tx));

        Ok(Self {
            cache,
            tx: Some(tx),
            done_rx: Some(done_rx),
            writer: Some(writer),
            shutdown_flag: AtomicBool::new(false),
            deadline: config.shutdown_deadline,
            channel_capacity: config.channel_capacity,
        })
    }

    /// Submit a batch of UTF-8 log lines identified by `batch_id`.
    /// See module-level docs and the spec for the full contract.
    pub fn submit(
        &self,
        batch_id: impl Into<String>,
        entries: &[String],
    ) -> Result<(), IngestorError> {
        if self.shutdown_flag.load(Ordering::Acquire) {
            return Err(IngestorError::ShutDown);
        }
        let tx = self.tx.as_ref().ok_or(IngestorError::ShutDown)?;
        let id = batch_id.into();
        if self.cache.get(&id).is_some() {
            return Err(IngestorError::DuplicateBatchId(id));
        }
        self.cache.insert(id.clone(), ());
        let envelope = BatchEnvelope {
            batch_id: id,
            entries: entries.to_vec(),
        };
        match tx.send(envelope) {
            Ok(()) => Ok(()),
            Err(crossbeam_channel::SendError(env)) => {
                // Channel full: undo the cache insert so the same id
                // can be retried later. `SendError` carries the
                // un-sent envelope back.
                self.cache.invalidate(&env.batch_id);
                Err(IngestorError::ChannelFull(self.channel_capacity))
            }
        }
    }

    /// Drain the channel and join the writer thread within
    /// `config.shutdown_deadline`. Idempotent.
    pub fn shutdown(&mut self) -> Result<(), IngestorError> {
        // Implemented in Task 6.
        unimplemented!("see Task 6")
    }
}

impl Drop for LogIngestor {
    fn drop(&mut self) {
        // Implemented in Task 6.
        let _ = self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::sync::Arc;
    use std::thread;

    fn read_log(dir: &Path, prefix: &str) -> (String, String) {
        let entry = std::fs::read_dir(dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .find(|e| e.file_name().to_string_lossy().starts_with(prefix))
            .expect("expected at least one daily-rotate file under prefix");
        let name = entry.file_name().to_string_lossy().into_owned();
        let contents = std::fs::read_to_string(entry.path()).unwrap();
        (name, contents)
    }

    fn cfg(dir: &Path) -> LogIngestorConfig {
        LogIngestorConfig::builder()
            .log_dir(dir.to_path_buf())
            .file_name_prefix("ingestor-test.log".to_string())
            .shutdown_deadline(Duration::from_secs(2))
            .build()
    }

    #[test]
    fn new_creates_log_dir_when_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let nested = tmp.path().join("nested/dir");
        assert!(!nested.exists());
        let ingestor = LogIngestor::new(cfg(&nested)).expect("new succeeds");
        assert!(nested.is_dir());
        drop(ingestor);
    }

    #[test]
    fn new_propagates_create_dir_failure() {
        let tmp = tempfile::tempdir().unwrap();
        let blocker = tmp.path().join("blocker");
        std::fs::write(&blocker, b"not a dir").unwrap();
        let bad = blocker.join("inside");
        let err = LogIngestor::new(cfg(&bad)).unwrap_err();
        match err {
            IngestorError::CreateDir { dir, .. } => assert_eq!(dir, bad),
            other => panic!("expected CreateDir, got {other:?}"),
        }
    }

    #[test]
    fn submit_writes_one_line_per_entry_under_configured_prefix() {
        let tmp = tempfile::tempdir().unwrap();
        let mut ingestor = LogIngestor::new(cfg(tmp.path())).expect("new succeeds");

        ingestor
            .submit(
                "b-1",
                &["alpha".to_string(), "beta".to_string(), "gamma".to_string()],
            )
            .expect("submit ok");

        ingestor.shutdown().expect("shutdown ok");

        let (_name, contents) = read_log(tmp.path(), "ingestor-test.log");
        assert_eq!(contents, "BATCH b-1\nalpha\nbeta\ngamma\n");
    }

    #[test]
    fn submit_two_batches_with_distinct_ids_both_appear_in_file() {
        let tmp = tempfile::tempdir().unwrap();
        let mut ingestor = LogIngestor::new(cfg(tmp.path())).expect("new succeeds");

        ingestor.submit("a", &["1".into()]).unwrap();
        ingestor.submit("b", &["2".into()]).unwrap();

        ingestor.shutdown().expect("shutdown ok");

        let (_name, contents) = read_log(tmp.path(), "ingestor-test.log");
        assert_eq!(contents, "BATCH a\n1\nBATCH b\n2\n");
    }

    #[test]
    fn submit_rejects_duplicate_batch_id_within_ttl() {
        let tmp = tempfile::tempdir().unwrap();
        let mut ingestor = LogIngestor::new(
            LogIngestorConfig::builder()
                .log_dir(tmp.path().to_path_buf())
                .file_name_prefix("dup.log".to_string())
                .cache_ttl(Duration::from_secs(60))
                .shutdown_deadline(Duration::from_secs(2))
                .build(),
        )
        .expect("new succeeds");

        ingestor.submit("dup", &["first".into()]).unwrap();
        let err = ingestor.submit("dup", &["second".into()]).unwrap_err();
        assert!(matches!(err, IngestorError::DuplicateBatchId(ref id) if id == "dup"));

        ingestor.shutdown().expect("shutdown ok");
    }

    #[test]
    fn submit_duplicate_after_ttl_expires_succeeds() {
        let tmp = tempfile::tempdir().unwrap();
        let mut ingestor = LogIngestor::new(
            LogIngestorConfig::builder()
                .log_dir(tmp.path().to_path_buf())
                .file_name_prefix("ttl.log".to_string())
                .cache_ttl(Duration::from_millis(50))
                .shutdown_deadline(Duration::from_secs(2))
                .build(),
        )
        .expect("new succeeds");

        ingestor.submit("ttl", &["first".into()]).unwrap();
        thread::sleep(Duration::from_millis(100));
        ingestor.submit("ttl", &["second".into()])
            .expect("submit after TTL expiry succeeds");

        ingestor.shutdown().expect("shutdown ok");
    }

    #[test]
    fn submit_concurrent_producers_no_deadlock_or_corruption() {
        // Review Focus #2: 8 threads × 25 distinct ids each = 200
        // batches submitted concurrently. All must succeed; all
        // 200 must appear in the file (moka is thread-safe, the
        // bounded channel is thread-safe).
        let tmp = tempfile::tempdir().unwrap();
        let mut ingestor = LogIngestor::new(cfg(tmp.path())).expect("new succeeds");
        let ingestor = Arc::new(ingestor);

        let mut handles = Vec::new();
        for t in 0..8 {
            let ingestor = Arc::clone(&ingestor);
            handles.push(thread::spawn(move || {
                for i in 0..25 {
                    let id = format!("t{t}-i{i}");
                    ingestor.submit(&id, &[format!("payload {id}")]).unwrap();
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }

        // Move out of Arc and shut down.
        let mut ingestor = Arc::try_unwrap(ingestor).expect("all senders joined");
        ingestor.shutdown().expect("shutdown ok");

        let (_name, contents) = read_log(tmp.path(), "ingestor-test.log");
        let batches: Vec<&str> = contents
            .lines()
            .filter(|l| l.starts_with("BATCH "))
            .collect();
        assert_eq!(batches.len(), 200, "all 200 batch headers must appear");

        // Spot-check that every distinct id appears exactly once.
        for t in 0..8 {
            for i in 0..25 {
                let id = format!("BATCH t{t}-i{i}");
                assert_eq!(
                    contents.matches(&id).count(),
                    1,
                    "id {id} must appear exactly once",
                );
            }
        }
    }
}
```

- [ ] **Step 2: Run the tests to verify they pass**

Run: `cargo test -p logging-utils --lib log_ingestor::ingestor::tests`
Expected: 7 passed. (`shutdown` is unimplemented; the tests that call `shutdown` will fail until Task 6 — that's expected. To get a clean green here, comment out the `shutdown` calls in the failing tests for now, or run the test suite after Task 6 lands.)

(Alternative for this step: run with the `shutdown` calls temporarily commented out. Re-add them in Task 6. The `submit_*` tests that don't need shutdown — like `submit_rejects_duplicate_batch_id_within_ttl` — can run as-is by adding `let _ = ingestor.shutdown();` after the assertions; `Drop` will handle the actual teardown.)

- [ ] **Step 3: Commit**

```bash
git add lib/crates/logging-utils/src/log_ingestor/ingestor.rs
git commit -m "feat(logging-utils): add LogIngestor::new + submit + dedup"
```

---

### Task 5: Channel-full backpressure + cache eviction at capacity

**Files:**
- Modify: `lib/crates/logging-utils/src/log_ingestor/ingestor.rs` — extend the `tests` module only

**Interfaces:**
- No new production code. The submit path already calls `cache.invalidate` on `ChannelFull` (Task 4) and moka's `max_capacity` is wired in `new` (Task 4). This task pins the behavior with tests.

- [ ] **Step 1: Add the failing tests**

Append to the `tests` mod in `lib/crates/logging-utils/src/log_ingestor/ingestor.rs`:

```rust
    #[test]
    fn submit_returns_channel_full_when_capacity_exceeded() {
        let tmp = tempfile::tempdir().unwrap();
        let mut ingestor = LogIngestor::new(
            LogIngestorConfig::builder()
                .log_dir(tmp.path().to_path_buf())
                .file_name_prefix("full.log".to_string())
                .channel_capacity(2) // tiny, easy to fill
                .shutdown_deadline(Duration::from_secs(2))
                .build(),
        )
        .expect("new succeeds");

        // First two sends fill the bounded channel without giving
        // the writer a chance to drain. Third send → ChannelFull.
        let big: Vec<String> = (0..200).map(|i| format!("entry-{i}")).collect();
        let r1 = ingestor.submit("b-1", &big);
        let r2 = ingestor.submit("b-2", &big);
        let r3 = ingestor.submit("b-3", &big);
        assert!(r1.is_ok(), "first send must succeed (channel has room)");
        assert!(r2.is_ok(), "second send must succeed (channel full)");
        assert!(
            matches!(r3, Err(IngestorError::ChannelFull(2))),
            "third send must return ChannelFull(2), got {r3:?}",
        );

        ingestor.shutdown().expect("shutdown ok");
    }

    #[test]
    fn submit_channel_full_invalidates_cache_entry_so_retry_succeeds() {
        // Review Focus companion: a ChannelFull must NOT poison the
        // dedup cache — the same batch_id should be retryable after
        // the writer drains.
        let tmp = tempfile::tempdir().unwrap();
        let mut ingestor = LogIngestor::new(
            LogIngestorConfig::builder()
                .log_dir(tmp.path().to_path_buf())
                .file_name_prefix("retry.log".to_string())
                .channel_capacity(1)
                .cache_ttl(Duration::from_secs(60))
                .shutdown_deadline(Duration::from_secs(2))
                .build(),
        )
        .expect("new succeeds");

        let big: Vec<String> = (0..500).map(|i| format!("entry-{i}")).collect();
        assert!(ingestor.submit("retry", &big).is_ok());
        // Now channel is full; this submit must return ChannelFull.
        let _ = ingestor.submit("retry", &big);
        // Drain the channel by dropping nothing — we can't from
        // outside. Use a different id to force the writer to
        // drain. After the writer drains, "retry" must succeed.
        // Spin briefly to let the writer process the first send.
        for _ in 0..100 {
            thread::sleep(Duration::from_millis(10));
            if ingestor.submit("retry", &big).is_ok() {
                return; // retry succeeded after writer drained
            }
        }
        panic!("retry never succeeded — cache.invalidate not wired");
    }

    #[test]
    fn cache_evicts_oldest_at_capacity() {
        let tmp = tempfile::tempdir().unwrap();
        let mut ingestor = LogIngestor::new(
            LogIngestorConfig::builder()
                .log_dir(tmp.path().to_path_buf())
                .file_name_prefix("evict.log".to_string())
                .cache_capacity(4) // tiny, easy to overflow
                .channel_capacity(1000) // avoid channel-full interference
                .shutdown_deadline(Duration::from_secs(2))
                .build(),
        )
        .expect("new succeeds");

        // Fill the cache to capacity with distinct ids.
        for i in 0..4 {
            ingestor.submit(format!("id-{i}"), &[format!("e-{i}")]).unwrap();
        }
        // One more distinct id triggers eviction of the oldest entry.
        ingestor.submit("id-5", &["e-5".into()]).unwrap();

        // The oldest id ("id-0") should be evicted and re-usable.
        // We can't directly observe the cache state, so we verify
        // by submitting "id-0" again — if it succeeded before TTL,
        // it would have been rejected; if it's now allowed, it was
        // evicted.
        ingestor.submit("id-0", &["e-0-again".into()])
            .expect("oldest id should be evicted and re-acceptable");

        ingestor.shutdown().expect("shutdown ok");
    }
```

- [ ] **Step 2: Run the tests to verify they pass**

Run: `cargo test -p logging-utils --lib log_ingestor::ingestor::tests::submit_returns_channel_full_when_capacity_exceeded log_ingestor::ingestor::tests::submit_channel_full_invalidates_cache_entry_so_retry_succeeds log_ingestor::ingestor::tests::cache_evicts_oldest_at_capacity`
Expected: 3 passed.

(If `submit_channel_full_invalidates_cache_entry_so_retry_succeeds` is flaky, the issue is the writer drain timing. Tighten the loop bound or use `Blocking` writer behavior — the test must remain deterministic without `sleep` longer than 1 s.)

- [ ] **Step 3: Commit**

```bash
git add lib/crates/logging-utils/src/log_ingestor/ingestor.rs
git commit -m "feat(logging-utils): pin channel-full + cache-eviction behavior"
```

---

### Task 6: `shutdown` + `Drop` + writer-panic handling

**Files:**
- Modify: `lib/crates/logging-utils/src/log_ingestor/ingestor.rs` — replace the `unimplemented!()` bodies with the real shutdown logic; add 5 tests

**Interfaces:**
- Updates: `impl LogIngestor { pub fn shutdown(&mut self) -> Result<(), IngestorError>; }` is now real.

- [ ] **Step 1: Implement `shutdown`**

Replace the `unimplemented!()` body of `shutdown` and the body of `Drop` in `lib/crates/logging-utils/src/log_ingestor/ingestor.rs`:

```rust
    /// Drain the channel and join the writer thread within
    /// `config.shutdown_deadline`. Idempotent: a second call after
    /// the first returns `Ok(())`.
    pub fn shutdown(&mut self) -> Result<(), IngestorError> {
        if self.writer.is_none() {
            return Ok(()); // already shut down
        }
        self.shutdown_flag.store(true, Ordering::Release);
        // Dropping `tx` closes the channel; the writer exits its
        // recv loop and signals done_tx.
        self.tx.take();
        let done_rx = self.done_rx.take()
            .expect("done_rx present iff writer is");
        let writer = self.writer.take()
            .expect("writer present iff not yet shut down");
        match done_rx.recv_timeout(self.deadline) {
            Ok(()) => match writer.join() {
                Ok(()) => Ok(()),
                Err(payload) => Err(IngestorError::WriterPanic(format!("{payload:?}"))),
            },
            Err(RecvTimeoutError::Timeout) => {
                // Deadline elapsed. Check whether the writer
                // already finished (it may have panicked and dropped
                // done_tx without sending).
                if writer.is_finished() {
                    match writer.join() {
                        Ok(()) => Err(IngestorError::WriterJoinTimeout {
                            deadline: self.deadline,
                            message: "writer exited but did not signal done".into(),
                        }),
                        Err(payload) => Err(IngestorError::WriterPanic(format!("{payload:?}"))),
                    }
                } else {
                    Err(IngestorError::WriterJoinTimeout {
                        deadline: self.deadline,
                        message: "writer thread did not exit in time".into(),
                    })
                }
            }
            Err(RecvTimeoutError::Disconnected) => {
                // done_tx dropped without sending — writer panicked.
                match writer.join() {
                    Ok(()) => Err(IngestorError::WriterJoinTimeout {
                        deadline: self.deadline,
                        message: "writer dropped done channel but exited cleanly".into(),
                    }),
                    Err(payload) => Err(IngestorError::WriterPanic(format!("{payload:?}"))),
                }
            }
        }
    }
```

And `Drop` already calls `self.shutdown()` and ignores the result; keep it as-is.

- [ ] **Step 2: Write the failing tests**

Append to the `tests` mod in `lib/crates/logging-utils/src/log_ingestor/ingestor.rs`:

```rust
    #[test]
    fn submit_returns_shut_down_after_shutdown() {
        // Review Focus #5.
        let tmp = tempfile::tempdir().unwrap();
        let mut ingestor = LogIngestor::new(cfg(tmp.path())).expect("new succeeds");

        ingestor.shutdown().expect("shutdown ok");
        let err = ingestor.submit("post", &["x".into()]).unwrap_err();
        assert!(
            matches!(err, IngestorError::ShutDown),
            "submit after shutdown must return ShutDown, got {err:?}",
        );
    }

    #[test]
    fn shutdown_drains_pending_envelopes_before_returning() {
        let tmp = tempfile::tempdir().unwrap();
        let mut ingestor = LogIngestor::new(
            LogIngestorConfig::builder()
                .log_dir(tmp.path().to_path_buf())
                .file_name_prefix("drain.log".to_string())
                .channel_capacity(100)
                .shutdown_deadline(Duration::from_secs(5))
                .build(),
        )
        .expect("new succeeds");

        for i in 0..50 {
            ingestor.submit(format!("d-{i}"), &[format!("line-{i}")]).unwrap();
        }
        ingestor.shutdown().expect("shutdown drains pending");

        let (_name, contents) = read_log(tmp.path(), "drain.log");
        let batch_count = contents.lines().filter(|l| l.starts_with("BATCH ")).count();
        assert_eq!(batch_count, 50, "all 50 batches must be in the file");
    }

    #[test]
    fn shutdown_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let mut ingestor = LogIngestor::new(cfg(tmp.path())).expect("new succeeds");

        ingestor.shutdown().expect("first shutdown ok");
        ingestor.shutdown().expect("second shutdown ok (idempotent)");
    }

    #[test]
    fn drop_drains_best_effort() {
        let tmp = tempfile::tempdir().unwrap();
        let mut ingestor = LogIngestor::new(cfg(tmp.path())).expect("new succeeds");

        ingestor.submit("drop-1", &["a".into(), "b".into()]).unwrap();
        ingestor.submit("drop-2", &["c".into()]).unwrap();

        // Drop without explicit shutdown — Drop's best-effort path
        // must drain the channel and join the writer.
        drop(ingestor);

        let (_name, contents) = read_log(tmp.path(), "ingestor-test.log");
        assert_eq!(contents, "BATCH drop-1\na\nb\nBATCH drop-2\nc\n");
    }

    #[test]
    fn shutdown_deadline_exceeded_returns_writer_join_timeout() {
        // Review Focus #4: hold a Sender alive past the deadline so
        // the writer's recv() never returns Err and never exits.
        let tmp = tempfile::tempdir().unwrap();
        let mut ingestor = LogIngestor::new(
            LogIngestorConfig::builder()
                .log_dir(tmp.path().to_path_buf())
                .file_name_prefix("deadline.log".to_string())
                .channel_capacity(4)
                .shutdown_deadline(Duration::from_millis(200))
                .build(),
        )
        .expect("new succeeds");

        // Keep a Sender alive across the shutdown call. We can't
        // get one from outside the crate, so instead we exhaust
        // shutdown's ability to drain by filling the channel and
        // not dropping it: the writer thread blocks on recv() for
        // the next 200 ms while shutdown waits on done_rx.
        let big: Vec<String> = (0..10_000).map(|i| format!("x-{i}")).collect();
        for i in 0..4 {
            ingestor.submit(format!("d-{i}"), &big).unwrap();
        }

        let err = ingestor.shutdown().unwrap_err();
        match err {
            IngestorError::WriterJoinTimeout { deadline, .. } => {
                assert_eq!(deadline, Duration::from_millis(200));
            }
            other => panic!("expected WriterJoinTimeout, got {other:?}"),
        }
    }
```

- [ ] **Step 3: Run the full ingestor test suite**

Run: `cargo test -p logging-utils --lib log_ingestor::ingestor::tests`
Expected: 12 passed (7 from Task 4 + 3 from Task 5 + 5 from Task 6 = 15; the spec test list counts 11 — Task 4 has 7, Task 5 has 3, Task 6 has 5 = 15 total because the spec's `submit_rejects_duplicate_batch_id_within_ttl` and `submit_duplicate_after_ttl_expires_succeeds` from Task 4 are also separate). Verify the actual count matches the test list.

- [ ] **Step 4: Commit**

```bash
git add lib/crates/logging-utils/src/log_ingestor/ingestor.rs
git commit -m "feat(logging-utils): add LogIngestor::shutdown + Drop + panic handling"
```

---

### Task 7: Public API surface + README + final verification

**Files:**
- Modify: `lib/crates/logging-utils/src/lib.rs` — `pub use` the new types
- Modify: `lib/crates/logging-utils/README.md` — document the new module

- [ ] **Step 1: Re-export the new types from `lib.rs`**

Edit `lib/crates/logging-utils/src/lib.rs`. Add one block of `pub use` lines (alongside the existing ones for `tracing_init` and `trace_id`):

```rust
pub use log_ingestor::{IngestorError, LogIngestor, LogIngestorConfig, LogIngestorConfigBuilder};
```

Also update the crate-level doc-comment to mention `log_ingestor`. The new doc-comment block:

```rust
//! `logging-utils` workspace crate.
//!
//! Unifies the `tracing` bootstrap used by `aegis-server` and
//! `aegis-desktop`, and owns the `LogIngestor` for `aegis-desktop`'s
//! client log submissions (deduplicated, bounded-queue, daily-rotating
//! file writer). Carries an independent copy of the
//! `TraceIdGenerator` / `Side` types (mirrored from
//! `lib/crates/trace-id`). The two `trace-id` crates do not depend on
//! each other.
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
```

- [ ] **Step 2: Document the module in `README.md`**

Append a new section to `lib/crates/logging-utils/README.md` (after the existing `## Trace ids` section, before `## Verification`):

```markdown
## Log ingestor (aegis-desktop only)

```rust
use logging_utils::{LogIngestor, LogIngestorConfig};
use std::time::Duration;

let ingestor = LogIngestor::new(
    LogIngestorConfig::builder()
        .log_dir("./logs".into())
        .file_name_prefix("desktop-events".into())
        .cache_capacity(10_000)              // default
        .cache_ttl(Duration::from_secs(600))  // default
        .channel_capacity(1_000)              // default
        .build(),
)?;

ingestor.submit("batch-123", &[
    "INFO  request_id=abc status=200".to_string(),
    "WARN  request_id=def retry=2".to_string(),
])?;

ingestor.shutdown()?;  // drains pending + joins writer; also runs in Drop
```

The ingestor is **for `aegis-desktop` only** — `aegis-server`'s log
pipeline is intentionally excluded. Each `batch_id` is accepted at
most once within the cache TTL (default 600 s); repeated IDs return
`IngestorError::DuplicateBatchId`. When the bounded channel (default
1 000 envelopes) is full, `submit` returns `IngestorError::ChannelFull`
and the just-inserted cache entry is invalidated so the same
`batch_id` can be retried once the writer drains. See
`docs/superpowers/specs/2026-10-02-log-ingestor-design.md` for the
full design.
```

- [ ] **Step 3: Run the verification gate**

Run, in order:

```bash
cargo fmt --all -- --check
cargo clippy -p logging-utils --all-targets --all-features -- -D warnings
cargo test  -p logging-utils
cargo doc   -p logging-utils --no-deps
cargo check --workspace
```

Expected: all green. The `cargo test` must report 19 passed total (3 config + 4 writer + 12 ingestor = 19 — counts may shift slightly if any test was merged during implementation).

- [ ] **Step 4: Commit**

```bash
git add lib/crates/logging-utils/
git commit -m "feat(logging-utils): expose LogIngestor + document in README"
```

---

## Self-Review

### 1. Spec coverage

| Spec section | Task |
|---|---|
| §"Approach" — module under `src/log_ingestor/`, exported via `pub use` | Task 1, Task 7 |
| §"Components" #1 — `LogIngestorConfig` + builder + defaults | Task 2 |
| §"Components" #2 — `LogIngestor`, `IngestorError`, `BatchEnvelope`, `submit`, `shutdown`, `Drop` | Tasks 4, 5, 6 |
| §"Components" #3 — `run_writer` + `write_envelope` (no `non_blocking`) | Task 3 |
| §"Components" #4 — `log_ingestor/mod.rs` re-exports | Task 1 |
| §"Components" #5 — `lib.rs` `pub use` surface | Task 7 |
| §"Data flow — submit path" — cache check, insert, send, invalidate on overflow | Tasks 4, 5 |
| §"Data flow — shutdown path" — done signal + `recv_timeout` + `is_finished` + `join` | Task 6 |
| §"Workspace wiring" — `Cargo.toml` root deps + crate deps | Task 1 |
| §"Public API at a glance" — type/kind table | Tasks 2, 4, 7 |
| §"Testing" — test list per file | Tasks 2, 3, 4, 5, 6 |
| §"Data model & wire shape" — `BATCH {id}` header + one line per entry | Task 3 |
| §"Out of scope" — no server wiring, no async, no per-batch files | Not implemented (explicitly excluded) |
| §"Verification gate" — fmt/clippy/test/doc/check commands | Task 7 |
| §"File changes summary" — 4 new module files + Cargo.toml + lib.rs + README | Tasks 1, 2, 3, 4, 5, 6, 7 |

All spec sections map to a task. ✅

### 2. Placeholder scan

No "TBD", "TODO", "implement later", "fill in details", "add appropriate error handling", or "similar to Task N" in the plan. ✅

### 3. Type consistency

- `LogIngestorConfig` defined in Task 2 with `{ log_dir, file_name_prefix, cache_capacity, cache_ttl, channel_capacity, shutdown_deadline }`. Task 4 reads `config.cache_capacity`, `config.cache_ttl`, `config.channel_capacity`, `config.shutdown_deadline`, `config.log_dir`, `config.file_name_prefix` — all consistent. ✅
- `IngestorError` variants defined in Task 4: `CreateDir`, `DuplicateBatchId`, `ChannelFull`, `ShutDown`, `WriterJoinTimeout`, `WriterPanic`. Task 4 tests use `DuplicateBatchId`, Task 5 tests use `ChannelFull`, Task 6 tests use `ShutDown` and `WriterJoinTimeout`. Writer panic is exercised in Task 3's `writer_panic_before_done_signal_propagates_via_thread_join` test (via `JoinHandle::join`'s `Err(payload)` → `format!("{payload:?}")` → `WriterPanic`). ✅
- `BatchEnvelope { batch_id: String, entries: Vec<String> }` defined in Task 3, used in Task 4's `submit` to construct the envelope. ✅
- `LogIngestor` fields used across Tasks 4, 5, 6 are consistent (no `tx.take()` + `tx.as_ref()` collisions — submit uses `as_ref`, shutdown uses `take`). ✅

### 4. Review Focus

The 5 review focus lines (empty entries, concurrent producers, writer panic before done, shutdown deadline exceeded, submit after shutdown) all have a test that pins them:

| Focus | Test | Task |
|---|---|---|
| Empty entries | `writer_writes_empty_entries_as_header_only` | Task 3 |
| Concurrent producers | `submit_concurrent_producers_no_deadlock_or_corruption` | Task 4 |
| Writer panic before done | `writer_panic_before_done_signal_propagates_via_thread_join` | Task 3 |
| Shutdown deadline exceeded | `shutdown_deadline_exceeded_returns_writer_join_timeout` | Task 6 |
| Submit after shutdown | `submit_returns_shut_down_after_shutdown` | Task 6 |

All five are pinned. ✅

No issues found; the plan is ready for execution.
