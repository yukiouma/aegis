# Log Submitter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `logging_utils::log_submitter` — a bounded-buffer batching worker that ships aegis-desktop `tracing` events to `POST /api/log-ingest/submit`, persisting failed batches to a pending directory and retrying them newest-first.

**Architecture:** A dedicated OS thread runs a current-thread tokio runtime and blocks on a crossbeam `recv_timeout`. Events are captured by a `tracing_subscriber::fmt::Layer` whose `MakeWriter` `try_send`s each formatted event into a bounded channel, so the shipped JSON is produced by the same formatter as the local log file. On `batch_size` or on `interval` of inactivity, a flush drains the pending directory newest-first (stopping at the first failure), then submits the fresh batch.

**Tech Stack:** Rust 2024, `tracing` / `tracing-subscriber` (`fmt`, `json`), `crossbeam-channel`, `tokio` (`rt`), `ulid`, `serde_json`, `thiserror`, `async-trait`; `wiremock` + `tempfile` for tests.

**Spec:** `docs/superpowers/specs/2026-10-04-log-submitter-design.md` — the plan argues from the spec, so the spec travels with it; executors read both.

## Global Constraints

- Workspace root for all commands: `/Users/yukichen/Coding/Projects/aegis`
- Crate uses `edition = "2024"`. Every new dependency in a `Cargo.toml` carries a one-line `# why` comment — this is a hard repo rule.
- Shared deps are declared once in `[workspace.dependencies]` and inherited via `{ workspace = true }`.
- Every function, type, and module gets a `//!`/`///` doc comment. Match the existing prose density in `log_ingestor/` — it is comment-heavy and explains *why*, not *what*.
- `SubmitterError` wraps inner errors with `#[source]` so `Error::source()` keeps the chain.
- No `unwrap()` / `expect()` in non-test code. (`HttpClient::new`'s `.expect("reqwest client builds")` is the existing precedent for build-infallible cases.)
- Tests are `#[cfg(test)] mod tests` **in the same file** as the code, matching `log_ingestor/`.
- Commit after every task. End commit messages with the `Co-Authored-By: Claude Code <noreply@anthropic.com>` trailer.
- **Addendum to the approved spec, made in Task 2 and Task 5:** `LogSender` gains a provided method `is_already_ingested(&self, err) -> bool`, and the pending drain treats a `true` result as success. Rationale in Review Focus #2 below. Do not drop it.

## Review Focus

The five failure modes the spec implies that no obvious test covers. Each line has its test added to the owning task.

1. **A lost response wedges the queue forever.** The server ingests a batch, the response is lost in transit, the submitter persists the batch, and the retry gets a permanent `409 duplicate_batch_id` — which under stop-on-failure blocks every older pending file and every future batch, forever. → Task 5, test `pending_drain_continues_past_already_ingested_batch`.
2. **An empty flush earns a 400 that wedges the queue.** The server rejects empty `entries` with `validation_failed`; an interval tick with nothing buffered would send an empty batch and trigger the same permanent wedge. → Task 5, test `interval_with_empty_buffer_sends_nothing`.
3. **An entry containing a newline splits into two on reload.** The pending file round-trips through the filesystem, so a naive line-delimited format would split one JSON log line into two entries. → Task 4, test `load_round_trips_entries_with_embedded_newlines`.
4. **A full disk silently drops batches.** `fs::write` fails, and retrying in memory forever would grow unbounded; the batch must be dropped and reported. → Task 4, test `persist_into_blocked_path_returns_persist_error`.
5. **A `device_id` containing `/` escapes the pending directory.** The batch id is used as a filename. → Task 5, test `batch_id_sanitizes_device_id_for_filesystem_use`.

## File Structure

**Created**

- `lib/crates/logging-utils/src/log_submitter.rs` — module hub
- `lib/crates/logging-utils/src/log_submitter/sender.rs` — `LogSender` trait, `SubmitterError`
- `lib/crates/logging-utils/src/log_submitter/config.rs` — `LogSubmitterConfig` + builder
- `lib/crates/logging-utils/src/log_submitter/pending.rs` — `PendingStore`
- `lib/crates/logging-utils/src/log_submitter/submitter.rs` — `mint_batch_id`, `WorkerCtx`, `run_worker`, `flush`, `SubmitHandle`, `LogSubmitter`
- `lib/crates/logging-utils/src/log_submitter/layer.rs` — `SubmitWriter`, `EntryWriter`, `SubmitLayer`, `submit_layer()`
- `apps/desktop/aegis-desktop/src-tauri/src/http/log_ingest.rs` — `HttpLogSender`

**Modified**

- `lib/crates/logging-utils/Cargo.toml` — add `tokio`, `serde_json`
- `lib/crates/logging-utils/src/lib.rs` — declare + re-export the module
- `lib/crates/logging-utils/src/trace_id.rs` — add `device_prefix()`
- `lib/crates/logging-utils/src/tracing_init.rs` — `init_tracing` gains `layers`
- `lib/crates/logging-utils/README.md` — document the module
- `apps/server/aegis-server/src/run.rs` — pass `Vec::new()`
- `apps/desktop/aegis-desktop/src-tauri/src/http.rs` — `pub mod log_ingest;`
- `apps/desktop/aegis-desktop/src-tauri/src/lib.rs` — reorder `setup`, build submitter + sender

---

### Task 1: `TraceIdGenerator::device_prefix()`

**Files:**
- Modify: `lib/crates/logging-utils/src/trace_id.rs` (add method inside the existing `impl TraceIdGenerator` block, after `server_side`)

**Interfaces:**
- Consumes: nothing
- Produces: `pub fn device_prefix(&self) -> Option<&str>` on `TraceIdGenerator` — Task 10 uses it to source the submitter's `device_id`.

- [ ] **Step 1: Write the failing test**

Append to the existing `mod tests` in `trace_id.rs`:

```rust
    #[test]
    fn device_prefix_returns_the_supplied_prefix() {
        let g = TraceIdGenerator::new(Some("desktop-abc".to_string()));
        assert_eq!(g.device_prefix(), Some("desktop-abc"));
    }

    #[test]
    fn device_prefix_is_none_when_constructed_without_one() {
        let g = TraceIdGenerator::new(None);
        assert_eq!(g.device_prefix(), None);
    }

    #[test]
    fn device_prefix_survives_clone() {
        let g = TraceIdGenerator::new(Some("fixed".to_string()));
        let cloned = g.clone();
        assert_eq!(cloned.device_prefix(), Some("fixed"));
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p logging-utils device_prefix`
Expected: FAIL — `no method named device_prefix found for &TraceIdGenerator`

- [ ] **Step 3: Write minimal implementation**

Add inside `impl TraceIdGenerator`, after `server_side`:

```rust
    /// The device prefix this generator was built with, or `None`
    /// when constructed without one.
    ///
    /// Callers that need a stable per-install identifier outside of
    /// trace ids — the log submitter's `device_id`, which becomes
    /// the first segment of every batch id — read it from here
    /// rather than re-reading the prefix file, so a batch id and a
    /// trace id from the same install always share that segment.
    pub fn device_prefix(&self) -> Option<&str> {
        self.device_prefix.as_deref()
    }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p logging-utils device_prefix`
Expected: PASS (3 tests)

- [ ] **Step 5: Commit**

```bash
git add lib/crates/logging-utils/src/trace_id.rs
git commit -m "$(cat <<'EOF'
feat(logging-utils): expose TraceIdGenerator::device_prefix

Callers outside trace-id construction (the log submitter's
device_id) need the persisted per-install prefix. Expose it as a
borrowing accessor so no caller re-reads the prefix file and a
batch id and a trace id from the same install always agree.

Co-Authored-By: Claude Code <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: `LogSender` trait and `SubmitterError`

**Files:**
- Create: `lib/crates/logging-utils/src/log_submitter.rs` (module hub, minimal for now)
- Create: `lib/crates/logging-utils/src/log_submitter/sender.rs`

**Interfaces:**
- Consumes: nothing
- Produces:
  - `pub enum SubmitterError` with variants `CreateDir { dir: PathBuf, source: io::Error }`, `Persist { batch_id: String, source: io::Error }`, `Send { batch_id: String, source: Box<dyn Error + Send + Sync> }`, `ChannelClosed`, `WorkerPanic(String)`, `WorkerJoinTimeout { deadline: Duration, message: String }`
  - `#[async_trait] pub trait LogSender: Debug + Send + Sync` with required `async fn send(&self, batch_id: &str, log_entries: Vec<String>) -> Result<(), SubmitterError>` and provided `fn is_already_ingested(&self, _err: &SubmitterError) -> bool { false }`
  - Both are re-exported at `logging_utils::log_submitter::{LogSender, SubmitterError}`. Tasks 3–9 depend on these exact names.

- [ ] **Step 1: Write the failing test**

Create `lib/crates/logging-utils/src/log_submitter/sender.rs` with the tests first:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Debug, Default)]
    struct Silent;

    #[async_trait::async_trait]
    impl LogSender for Silent {
        async fn send(
            &self,
            _batch_id: &str,
            _log_entries: Vec<String>,
        ) -> Result<(), SubmitterError> {
            Ok(())
        }
    }

    #[test]
    fn is_already_ingested_defaults_to_false() {
        let s = Silent;
        let err = SubmitterError::ChannelClosed;
        assert!(!s.is_already_ingested(&err));
    }

    #[test]
    fn send_error_reports_the_batch_id() {
        let err = SubmitterError::Send {
            batch_id: "dev-01HQ".into(),
            source: Box::new(std::io::Error::other("boom")),
        };
        assert!(err.to_string().contains("dev-01HQ"));
    }

    #[test]
    fn send_error_keeps_the_source_chain() {
        let io = std::io::Error::other("disk gone");
        let err = SubmitterError::Send {
            batch_id: "b".into(),
            source: Box::new(io),
        };
        assert!(std::error::Error::source(&err).is_some());
    }

    #[test]
    fn worker_panic_error_renders_the_payload() {
        let err = SubmitterError::WorkerPanic("thread died".into());
        assert!(err.to_string().contains("thread died"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p logging-utils log_submitter`
Expected: FAIL — module does not exist

- [ ] **Step 3: Write minimal implementation**

Prepend to `sender.rs`, above the test module:

```rust
//! The transport seam for [`crate::LogSubmitter`]. The submitter
//! never speaks HTTP itself; it hands a batch id and its entries to
//! a [`LogSender`] and interprets the result.

use std::fmt::Debug;
use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;
use thiserror::Error;

/// Everything that can go wrong between minting a batch and getting
/// it accepted by the sink. One error type per boundary, mirroring
/// `IngestorError` on the server side; each variant wraps its inner
/// cause with `#[source]`.
#[derive(Debug, Error)]
pub enum SubmitterError {
    #[error("failed to create pending directory {dir}: {source}")]
    CreateDir {
        dir: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to persist batch {batch_id} to the pending directory: {source}")]
    Persist {
        batch_id: String,
        #[source]
        source: std::io::Error,
    },
    #[error("submitting batch {batch_id} failed: {source}")]
    Send {
        batch_id: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    #[error("the submitter worker is gone")]
    ChannelClosed,
    #[error("submitter worker thread panicked: {0}")]
    WorkerPanic(String),
    #[error("submitter worker thread failed to join within {deadline:?}: {message}")]
    WorkerJoinTimeout {
        deadline: Duration,
        message: String,
    },
}

/// Delivers a batch of log lines to the sink.
///
/// `Debug` is a supertrait (not just a bound) so `LogSubmitterConfig`
/// can derive `Debug` alongside `Clone`.
#[async_trait]
pub trait LogSender: Debug + Send + Sync {
    /// Submit one batch. `batch_id` identifies the batch for
    /// idempotency; a sink that has already accepted a given id must
    /// say so rather than accepting it twice.
    async fn send(
        &self,
        batch_id: &str,
        log_entries: Vec<String>,
    ) -> Result<(), SubmitterError>;

    /// Classify `err` as "the sink already has this batch".
    ///
    /// This exists because the server dedups on `batch_id`: if a
    /// batch is ingested but the response is lost in transit, the
    /// submitter persists it and the retry is answered `409
    /// duplicate_batch_id` forever. Treating that as success deletes
    /// the pending file and lets the drain continue; treating it as
    /// a failure would wedge the queue permanently, because the
    /// drain stops at the first failure.
    ///
    /// Defaults to `false`, which is correct for sinks without
    /// server-side dedup.
    fn is_already_ingested(&self, _err: &SubmitterError) -> bool {
        false
    }
}
```

Create `log_submitter.rs`:

```rust
//! `log_submitter` — batched, bounded-queue, disk-backed log
//! submitter for `aegis-desktop`. The producer-side counterpart to
//! `log_ingestor`, which is the server-side consumer.
//!
//! See `docs/superpowers/specs/2026-10-04-log-submitter-design.md`.

mod sender;

pub use sender::{LogSender, SubmitterError};
```

- [ ] **Step 4: Add `async-trait` to the crate**

In `lib/crates/logging-utils/Cargo.toml`, add to `[dependencies]`:

```toml
# `async-trait` gives `LogSender` an object-safe async method, so
# the submitter can hold an `Arc<dyn LogSender>` and stay
# runtime-agnostic.
async-trait = { workspace = true }
```

- [ ] **Step 5: Declare the module in `lib.rs`**

In `lib/crates/logging-utils/src/lib.rs`, change:

```rust
pub mod log_ingestor;
pub mod log_submitter;
pub mod trace_id;
pub mod tracing_init;

pub use log_ingestor::{IngestorError, LogIngestor, LogIngestorConfig, LogIngestorConfigBuilder};
pub use log_submitter::{LogSender, SubmitterError};
```

- [ ] **Step 6: Run test to verify it passes**

Run: `cargo test -p logging-utils log_submitter`
Expected: PASS (4 tests)

- [ ] **Step 7: Commit**

```bash
git add lib/crates/logging-utils/Cargo.toml lib/crates/logging-utils/src/lib.rs lib/crates/logging-utils/src/log_submitter.rs lib/crates/logging-utils/src/log_submitter/sender.rs
git commit -m "$(cat <<'EOF'
feat(logging-utils): add LogSender trait + SubmitterError

The transport seam for log_submitter. LogSender is object-safe
(async-trait) so the submitter holds an Arc<dyn LogSender> and
stays runtime-agnostic; SubmitterError is the single boundary
error, each variant wrapping its cause with #[source].

is_already_ingested is a provided method defaulting to false: the
server dedups on batch_id, so a batch whose response was lost in
transit retries as a permanent 409. Under stop-on-failure that
would wedge the queue forever, so the sender gets a say in
whether a failure means "already there".

Co-Authored-By: Claude Code <noreply@anthropic.com>
EOF
)"
```

---

### Task 3: `LogSubmitterConfig`

**Files:**
- Create: `lib/crates/logging-utils/src/log_submitter/config.rs`
- Modify: `lib/crates/logging-utils/src/log_submitter.rs`

**Interfaces:**
- Consumes: `LogSender` (Task 2)
- Produces: `LogSubmitterConfig { device_id: String, sender: Arc<dyn LogSender>, batch_size: usize, interval: Duration, buffer_capacity: usize, pending_dir: PathBuf, max_pending_files: usize, shutdown_deadline: Duration }` plus `LogSubmitterConfig::new(device_id, sender, pending_dir)`, `::builder()`, and `LogSubmitterConfigBuilder` with one method per knob. Task 4 needs `pending_dir` + `max_pending_files`; Task 5 needs `batch_size` + `interval`; Task 6 needs `buffer_capacity` + `shutdown_deadline`.

- [ ] **Step 1: Write the failing test**

Create `config.rs` containing only this test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[derive(Debug, Default)]
    struct NullSender;

    #[async_trait::async_trait]
    impl LogSender for NullSender {
        async fn send(
            &self,
            _batch_id: &str,
            _log_entries: Vec<String>,
        ) -> Result<(), SubmitterError> {
            Ok(())
        }
    }

    fn sender() -> Arc<dyn LogSender> {
        Arc::new(NullSender)
    }

    #[test]
    fn new_applies_documented_defaults() {
        let cfg = LogSubmitterConfig::new(
            "dev-1".to_string(),
            sender(),
            PathBuf::from("/tmp/aegis-pending"),
        );
        assert_eq!(cfg.device_id, "dev-1");
        assert_eq!(cfg.pending_dir, PathBuf::from("/tmp/aegis-pending"));
        assert_eq!(cfg.batch_size, 200);
        assert_eq!(cfg.interval, Duration::from_secs(60));
        assert_eq!(cfg.buffer_capacity, 1_000);
        assert_eq!(cfg.max_pending_files, 10);
        assert_eq!(cfg.shutdown_deadline, Duration::from_secs(5));
    }

    #[test]
    fn builder_overrides_each_knob_independently() {
        let cfg = LogSubmitterConfig::builder()
            .device_id("dev-2".to_string())
            .sender(sender())
            .pending_dir(PathBuf::from("/tmp/p2"))
            .batch_size(7)
            .interval(Duration::from_millis(250))
            .buffer_capacity(9)
            .max_pending_files(3)
            .shutdown_deadline(Duration::from_secs(2))
            .build();
        assert_eq!(cfg.device_id, "dev-2");
        assert_eq!(cfg.pending_dir, PathBuf::from("/tmp/p2"));
        assert_eq!(cfg.batch_size, 7);
        assert_eq!(cfg.interval, Duration::from_millis(250));
        assert_eq!(cfg.buffer_capacity, 9);
        assert_eq!(cfg.max_pending_files, 3);
        assert_eq!(cfg.shutdown_deadline, Duration::from_secs(2));
    }

    #[test]
    fn builder_fills_unset_knobs_with_defaults() {
        let cfg = LogSubmitterConfig::builder()
            .device_id("dev-3".to_string())
            .sender(sender())
            .pending_dir(PathBuf::from("/tmp/p3"))
            .build();
        assert_eq!(cfg.batch_size, 200);
        assert_eq!(cfg.buffer_capacity, 1_000);
    }

    #[test]
    fn clone_preserves_scalar_fields() {
        let original = LogSubmitterConfig::new(
            "dev-4".to_string(),
            sender(),
            PathBuf::from("/tmp/p4"),
        );
        let cloned = original.clone();
        assert_eq!(original.device_id, cloned.device_id);
        assert_eq!(original.batch_size, cloned.batch_size);
        assert_eq!(original.pending_dir, cloned.pending_dir);
        assert_eq!(original.max_pending_files, cloned.max_pending_files);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p logging-utils log_submitter::config`
Expected: FAIL — `LogSubmitterConfig` not found

- [ ] **Step 3: Write minimal implementation**

Prepend to `config.rs`:

```rust
//! Knobs for [`crate::LogSubmitter`]. Defaults are pinned in
//! [`LogSubmitterConfig::new`] and match the values named in the
//! design spec.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use super::sender::LogSender;

/// Entries per batch before a size-triggered flush.
const DEFAULT_BATCH_SIZE: usize = 200;
/// Idle window before a timer-triggered flush.
const DEFAULT_INTERVAL: Duration = Duration::from_secs(60);
/// Bounded channel capacity between the submit layer and the worker.
const DEFAULT_BUFFER_CAPACITY: usize = 1_000;
/// Ceiling on persisted failed batches; the oldest is evicted past it.
const DEFAULT_MAX_PENDING_FILES: usize = 10;
/// Bound on the shutdown join.
const DEFAULT_SHUTDOWN_DEADLINE: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
pub struct LogSubmitterConfig {
    /// Per-install identifier, and the first segment of every batch id.
    pub device_id: String,
    /// The transport the worker submits batches through.
    pub sender: Arc<dyn LogSender>,
    /// Flush once this many entries are buffered.
    pub batch_size: usize,
    /// Flush after this much inactivity on the buffer.
    pub interval: Duration,
    /// Bounded channel capacity between the layer and the worker.
    pub buffer_capacity: usize,
    /// Where failed batches are persisted, and where they are
    /// retried from.
    pub pending_dir: PathBuf,
    /// Ceiling on pending files; the oldest is evicted past it.
    pub max_pending_files: usize,
    /// Bound on the shutdown join.
    pub shutdown_deadline: Duration,
}

impl LogSubmitterConfig {
    /// Build a config with every optional knob at its default.
    pub fn new(device_id: String, sender: Arc<dyn LogSender>, pending_dir: PathBuf) -> Self {
        Self {
            device_id,
            sender,
            batch_size: DEFAULT_BATCH_SIZE,
            interval: DEFAULT_INTERVAL,
            buffer_capacity: DEFAULT_BUFFER_CAPACITY,
            pending_dir,
            max_pending_files: DEFAULT_MAX_PENDING_FILES,
            shutdown_deadline: DEFAULT_SHUTDOWN_DEADLINE,
        }
    }

    pub fn builder() -> LogSubmitterConfigBuilder {
        LogSubmitterConfigBuilder::default()
    }
}

#[derive(Debug, Clone, Default)]
pub struct LogSubmitterConfigBuilder {
    device_id: Option<String>,
    sender: Option<Arc<dyn LogSender>>,
    batch_size: Option<usize>,
    interval: Option<Duration>,
    buffer_capacity: Option<usize>,
    pending_dir: Option<PathBuf>,
    max_pending_files: Option<usize>,
    shutdown_deadline: Option<Duration>,
}

impl LogSubmitterConfigBuilder {
    pub fn device_id(mut self, v: String) -> Self {
        self.device_id = Some(v);
        self
    }
    pub fn sender(mut self, v: Arc<dyn LogSender>) -> Self {
        self.sender = Some(v);
        self
    }
    pub fn batch_size(mut self, v: usize) -> Self {
        self.batch_size = Some(v);
        self
    }
    pub fn interval(mut self, v: Duration) -> Self {
        self.interval = Some(v);
        self
    }
    pub fn buffer_capacity(mut self, v: usize) -> Self {
        self.buffer_capacity = Some(v);
        self
    }
    pub fn pending_dir(mut self, v: PathBuf) -> Self {
        self.pending_dir = Some(v);
        self
    }
    pub fn max_pending_files(mut self, v: usize) -> Self {
        self.max_pending_files = Some(v);
        self
    }
    pub fn shutdown_deadline(mut self, v: Duration) -> Self {
        self.shutdown_deadline = Some(v);
        self
    }

    pub fn build(self) -> LogSubmitterConfig {
        let d = LogSubmitterConfig::new(String::new(), {
            // A placeholder sender is only ever read for its
            // defaults, which are all scalars; `build` panics below
            // if the caller never supplied one.
            Arc::new(NoopSender) as Arc<dyn LogSender>
        }, PathBuf::new());
        LogSubmitterConfig {
            device_id: self.device_id.unwrap_or(d.device_id),
            sender: self.sender.unwrap_or(d.sender),
            batch_size: self.batch_size.unwrap_or(d.batch_size),
            interval: self.interval.unwrap_or(d.interval),
            buffer_capacity: self.buffer_capacity.unwrap_or(d.buffer_capacity),
            pending_dir: self.pending_dir.unwrap_or(d.pending_dir),
            max_pending_files: self.max_pending_files.unwrap_or(d.max_pending_files),
            shutdown_deadline: self.shutdown_deadline.unwrap_or(d.shutdown_deadline),
        }
    }
}
```

`build` above needs a `NoopSender`; simpler and clearer is to make the builder's `build` fall back to a private zero-arg placeholder. Define it just above `LogSubmitterConfigBuilder`:

```rust
/// Placeholder used only to source scalar defaults in
/// `LogSubmitterConfigBuilder::build`. Never reaches a worker: the
/// builder's `build` overwrites it with the caller's sender.
#[derive(Debug)]
struct NoopSender;

#[async_trait::async_trait]
impl LogSender for NoopSender {
    async fn send(
        &self,
        _batch_id: &str,
        _log_entries: Vec<String>,
    ) -> Result<(), SubmitterError> {
        Ok(())
    }
}
```

and `use super::sender::{LogSender, SubmitterError};`.

- [ ] **Step 4: Export from the module hub**

In `log_submitter.rs`, change to:

```rust
mod config;
mod sender;

pub use config::{LogSubmitterConfig, LogSubmitterConfigBuilder};
pub use sender::{LogSender, SubmitterError};
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test -p logging-utils log_submitter::config`
Expected: PASS (4 tests)

- [ ] **Step 6: Commit**

```bash
git add lib/crates/logging-utils/src/log_submitter.rs lib/crates/logging-utils/src/log_submitter/config.rs
git commit -m "$(cat <<'EOF'
feat(logging-utils): add LogSubmitterConfig + builder

Batch size, idle interval, buffer capacity, pending dir, pending
file ceiling, and shutdown deadline. Mirrors the LogIngestorConfig
+ builder shape so the two modules in this crate read the same way.

Co-Authored-By: Claude Code <noreply@anthropic.com>
EOF
)"
```

---

### Task 4: `PendingStore` — on-disk failed-batch queue

**Files:**
- Create: `lib/crates/logging-utils/src/log_submitter/pending.rs`
- Modify: `lib/crates/logging-utils/Cargo.toml`
- Modify: `lib/crates/logging-utils/src/log_submitter.rs`

**Interfaces:**
- Consumes: `SubmitterError` (Task 2)
- Produces: `pub(crate) struct PendingStore` with `new(dir: PathBuf, max_files: usize) -> Self`, `create_dir(&self) -> Result<(), SubmitterError>`, `persist(&self, batch_id: &str, entries: &[String]) -> Result<(), SubmitterError>`, `list_newest_first(&self) -> Vec<String>`, `load(&self, batch_id: &str) -> Result<Vec<String>, SubmitterError>`, `remove(&self, batch_id: &str)`. Task 5 drives all of these.

- [ ] **Step 1: Write the failing test**

Create `pending.rs` with only the test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn store(dir: &std::path::Path, max_files: usize) -> PendingStore {
        let s = PendingStore::new(dir.to_path_buf(), max_files);
        s.create_dir().unwrap();
        s
    }

    fn entries(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn create_dir_makes_the_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("pending");
        let s = PendingStore::new(dir.clone(), 4);
        s.create_dir().unwrap();
        assert!(dir.is_dir());
    }

    #[test]
    fn create_dir_reports_failure_for_a_blocked_path() {
        let tmp = tempfile::tempdir().unwrap();
        let blocker = tmp.path().join("blocker");
        std::fs::write(&blocker, b"not a dir").unwrap();
        let s = PendingStore::new(blocker.join("nested"), 4);
        let err = s.create_dir().unwrap_err();
        assert!(matches!(err, SubmitterError::CreateDir { .. }));
    }

    #[test]
    fn persist_names_the_file_after_the_batch_id() {
        let tmp = tempfile::tempdir().unwrap();
        let s = store(tmp.path(), 4);
        s.persist("dev-A01HQ", &entries(&["a", "b"])).unwrap();
        assert!(tmp.path().join("dev-A01HQ").is_file());
    }

    #[test]
    fn load_round_trips_entries_with_embedded_newlines() {
        let tmp = tempfile::tempdir().unwrap();
        let s = store(tmp.path(), 4);
        let tricky = entries(&["{\"level\":\"info\"}\n", "tail\r\nmore", "quo\"te"]);
        s.persist("dev-newlines", &tricky).unwrap();
        assert_eq!(s.load("dev-newlines").unwrap(), tricky);
    }

    #[test]
    fn list_newest_first_is_reverse_lexicographic() {
        let tmp = tempfile::tempdir().unwrap();
        let s = store(tmp.path(), 10);
        for id in ["dev-A01HQ", "dev-A05HQ", "dev-A03HQ"] {
            s.persist(id, &entries(&["x"])).unwrap();
        }
        assert_eq!(
            s.list_newest_first(),
            vec!["dev-A05HQ".to_string(), "dev-A03HQ".to_string(), "dev-A01HQ".to_string()]
        );
    }

    #[test]
    fn list_ignores_non_batch_files() {
        let tmp = tempfile::tempdir().unwrap();
        let s = store(tmp.path(), 10);
        s.persist("dev-A01HQ", &entries(&["x"])).unwrap();
        std::fs::write(tmp.path().join("notes.txt"), b"ignore me").unwrap();
        assert_eq!(s.list_newest_first(), vec!["dev-A01HQ".to_string()]);
    }

    #[test]
    fn remove_deletes_and_is_a_noop_when_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let s = store(tmp.path(), 4);
        s.persist("dev-A01HQ", &entries(&["x"])).unwrap();
        s.remove("dev-A01HQ");
        assert!(!tmp.path().join("dev-A01HQ").exists());
        s.remove("dev-A01HQ");
    }

    #[test]
    fn persist_past_the_cap_evicts_the_oldest() {
        let tmp = tempfile::tempdir().unwrap();
        let s = store(tmp.path(), 2);
        for id in ["dev-A01HQ", "dev-A02HQ", "dev-A03HQ"] {
            s.persist(id, &entries(&["x"])).unwrap();
        }
        let left = s.list_newest_first();
        assert_eq!(left.len(), 2, "cap enforced, got {left:?}");
        assert_eq!(left, vec!["dev-A03HQ".to_string(), "dev-A02HQ".to_string()]);
    }

    #[test]
    fn load_reports_failure_for_a_missing_batch() {
        let tmp = tempfile::tempdir().unwrap();
        let s = store(tmp.path(), 4);
        assert!(s.load("dev-nope").is_err());
    }

    #[test]
    fn persist_into_a_blocked_path_returns_persist_error() {
        let tmp = tempfile::tempdir().unwrap();
        let blocker = tmp.path().join("blocker");
        std::fs::write(&blocker, b"not a dir").unwrap();
        let s = PendingStore::new(blocker, 4);
        let err = s.persist("dev-A01HQ", &entries(&["x"])).unwrap_err();
        assert!(
            matches!(err, SubmitterError::Persist { ref batch_id, .. } if batch_id == "dev-A01HQ"),
            "got {err:?}"
        );
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p logging-utils log_submitter::pending`
Expected: FAIL — `PendingStore` not found

- [ ] **Step 3: Write minimal implementation**

Prepend to `pending.rs`:

```rust
//! The on-disk queue of batches that failed to submit.
//!
//! One file per batch, named for the batch id. Because the batch id
//! ends in a ULID, a reverse lexicographic listing of the directory
//! is exactly newest-first — the retry order the spec asks for,
//! with no mtime reads and no clock involved.
//!
//! The on-disk format is a JSON array of strings, not one-entry-per-
//! line. A formatted log event is a single line of JSON, but that
//! is a property of the *caller*, not a guarantee we make to the
//! file: an entry containing a literal newline would split into two
//! lines and silently become two entries on reload. JSON escapes
//! newlines, so the round trip is exact.

use std::fs;
use std::path::{Path, PathBuf};

use super::sender::SubmitterError;

/// The queue of batches awaiting a successful submit.
pub(crate) struct PendingStore {
    dir: PathBuf,
    max_files: usize,
}

impl PendingStore {
    pub(crate) fn new(dir: PathBuf, max_files: usize) -> Self {
        Self { dir, max_files }
    }

    /// Create the pending directory. Idempotent.
    pub(crate) fn create_dir(&self) -> Result<(), SubmitterError> {
        fs::create_dir_all(&self.dir).map_err(|source| SubmitterError::CreateDir {
            dir: self.dir.clone(),
            source,
        })
    }

    /// Write `entries` under `batch_id`, then enforce the file cap.
    pub(crate) fn persist(&self, batch_id: &str, entries: &[String]) -> Result<(), SubmitterError> {
        let encoded = serde_json::to_string(entries).map_err(|source| SubmitterError::Persist {
            batch_id: batch_id.to_string(),
            source,
        })?;
        fs::write(self.dir.join(batch_id), encoded).map_err(|source| SubmitterError::Persist {
            batch_id: batch_id.to_string(),
            source,
        })?;
        self.enforce_cap();
        Ok(())
    }

    /// Batch ids, newest first.
    pub(crate) fn list_newest_first(&self) -> Vec<String> {
        let Ok(dir) = fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        let mut ids: Vec<String> = dir
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        ids.sort();
        ids.reverse();
        ids
    }

    /// Read back a persisted batch.
    pub(crate) fn load(&self, batch_id: &str) -> Result<Vec<String>, SubmitterError> {
        let path = self.dir.join(batch_id);
        let raw = fs::read_to_string(&path).map_err(|source| SubmitterError::Persist {
            batch_id: batch_id.to_string(),
            source,
        })?;
        serde_json::from_str(&raw).map_err(|source| SubmitterError::Persist {
            batch_id: batch_id.to_string(),
            source,
        })
    }

    /// Delete a persisted batch. Missing files are not an error — the
    /// caller's intent ("this batch is no longer pending") already
    /// holds.
    pub(crate) fn remove(&self, batch_id: &str) {
        let _ = fs::remove_file(self.dir.join(batch_id));
    }

    /// Bound the backlog. Past `max_files`, the oldest ids are
    /// evicted, so a long outage degrades into "keep the most recent
    /// batches" instead of filling the disk.
    fn enforce_cap(&self) {
        let ids = self.list_newest_first();
        for stale in ids.into_iter().skip(self.max_files) {
            self.remove(&stale);
        }
    }
}
```

Note: `serde_json::Error` is not `std::io::Error`, so the two `map_err` closures above will not type-check as written. Change the `persist`/`load` error mapping to wrap a `std::io::Error`:

```rust
        let encoded = serde_json::to_string(entries).map_err(|e| SubmitterError::Persist {
            batch_id: batch_id.to_string(),
            source: std::io::Error::other(e),
        })?;
```

…and the same substitution in `load`'s `from_str` arm. `std::io::Error::other` accepts any `Into<Box<dyn Error + Send + Sync>>`, which `serde_json::Error` satisfies.

- [ ] **Step 4: Add `serde_json` to the crate**

In `lib/crates/logging-utils/Cargo.toml`, add to `[dependencies]`:

```toml
# `serde_json` encodes/decodes the pending-dir file format (a JSON
# array of log entries) and the tracing layer's per-event
# formatting. Already a workspace dep.
serde_json = { workspace = true }
```

- [ ] **Step 5: Declare the module**

In `log_submitter.rs`, add `mod pending;` and change the re-exports to:

```rust
mod config;
mod pending;
mod sender;
```

(`PendingStore` stays `pub(crate)` — it is not re-exported.)

- [ ] **Step 6: Run test to verify it passes**

Run: `cargo test -p logging-utils log_submitter::pending`
Expected: PASS (10 tests)

- [ ] **Step 7: Commit**

```bash
git add lib/crates/logging-utils/Cargo.toml lib/crates/logging-utils/src/log_submitter.rs lib/crates/logging-utils/src/log_submitter/pending.rs
git commit -m "$(cat <<'EOF'
feat(logging-utils): add PendingStore for failed-batch persistence

One JSON file per batch, named for the batch id. JSON rather than
line-delimited because an entry containing a literal newline would
split into two entries on reload; the format has to round-trip
whatever the layer hands us, not assume it is single-line.

Batch ids end in a ULID, so a reverse lexicographic listing is
newest-first — the spec's retry order, with no mtime and no clock.
Past max_files the oldest are evicted, bounding the backlog during
a long outage.

Co-Authored-By: Claude Code <noreply@anthropic.com>
EOF
)"
```

---

### Task 5: batch id minting and the flush / drain worker

**Files:**
- Create: `lib/crates/logging-utils/src/log_submitter/submitter.rs`

**Interfaces:**
- Consumes: `LogSender`, `SubmitterError` (Task 2), `PendingStore` (Task 4)
- Produces: `pub(crate) fn mint_batch_id(device_id: &str) -> String`, `pub(crate) struct WorkerCtx { device_id: String, batch_size: usize, interval: Duration, pending: PendingStore, sender: Arc<dyn LogSender> }`, `pub(crate) async fn run_worker(rx: crossbeam_channel::Receiver<String>, ctx: WorkerCtx, done_tx: crossbeam_channel::Sender<()>)`, `pub(crate) async fn flush(ctx: &WorkerCtx, batch: &mut Vec<String>)`. Task 6 wraps `run_worker` in a thread.

- [ ] **Step 1: Write the failing test**

Create `submitter.rs` with only the test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam_channel::Receiver;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};

    /// Records every accepted batch, and can be told to fail for a
    /// specific batch id (or all of them).
    #[derive(Debug, Default)]
    struct RecordingSender {
        accepted: Mutex<Vec<(String, Vec<String>)>>,
        fail_ids: Mutex<Vec<String>>,
        fail_all: AtomicBool,
        ingested: Mutex<Vec<String>>,
    }

    impl RecordingSender {
        fn shared() -> Arc<RecordingSender> {
            Arc::new(RecordingSender::default())
        }
        fn accepted(&self) -> Vec<(String, Vec<String>)> {
            self.accepted.lock().unwrap().clone()
        }
        fn ingested(&self) -> Vec<String> {
            self.ingested.lock().unwrap().clone()
        }
        fn fail_for(&self, ids: &[&str]) {
            *self.fail_ids.lock().unwrap() =
                ids.iter().map(|s| s.to_string()).collect();
        }
        fn fail_everything(&self) {
            self.fail_all.store(true, Ordering::Release);
        }
    }

    #[async_trait::async_trait]
    impl LogSender for RecordingSender {
        async fn send(
            &self,
            batch_id: &str,
            log_entries: Vec<String>,
        ) -> Result<(), SubmitterError> {
            if self.fail_all.load(Ordering::Acquire)
                || self.fail_ids.lock().unwrap().iter().any(|i| i == batch_id)
            {
                return Err(SubmitterError::Send {
                    batch_id: batch_id.to_string(),
                    source: Box::new(std::io::Error::other("scripted failure")),
                });
            }
            self.accepted.lock().unwrap().push((batch_id.to_string(), log_entries));
            Ok(())
        }
    }

    /// Sender that fails a batch the sink already holds — the
    /// "response was lost in transit" shape.
    #[derive(Debug, Default)]
    struct AlreadyIngestedSender {
        seen: Mutex<Vec<String>>,
    }

    #[async_trait::async_trait]
    impl LogSender for AlreadyIngestedSender {
        async fn send(
            &self,
            batch_id: &str,
            _log_entries: Vec<String>,
        ) -> Result<(), SubmitterError> {
            self.seen.lock().unwrap().push(batch_id.to_string());
            Err(SubmitterError::Send {
                batch_id: batch_id.to_string(),
                source: Box::new(std::io::Error::other("409 duplicate_batch_id")),
            })
        }
        fn is_already_ingested(&self, _err: &SubmitterError) -> bool {
            true
        }
    }

    fn ctx(
        sender: Arc<dyn LogSender>,
        pending: &PendingStore,
        batch_size: usize,
    ) -> WorkerCtx {
        WorkerCtx {
            device_id: "dev-test".to_string(),
            batch_size,
            interval: Duration::from_millis(50),
            pending: pending.clone_store(),
            sender,
        }
    }

    /// Run the worker to completion on the current thread's runtime.
    async fn drive(rx: Receiver<String>, ctx: WorkerCtx) {
        let (done_tx, done_rx) = crossbeam_channel::bounded(1);
        let handle = tokio::task::spawn(run_worker(rx, ctx, done_tx));
        done_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        handle.await.unwrap();
    }

    fn temp_pending(max_files: usize) -> (tempfile::TempDir, PendingStore) {
        let tmp = tempfile::tempdir().unwrap();
        let store = PendingStore::new(tmp.path().join("pending"), max_files);
        store.create_dir().unwrap();
        (tmp, store)
    }

    fn entries(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("line-{i}")).collect()
    }

    // ---- batch id ----

    #[test]
    fn batch_id_is_device_id_plus_ulid() {
        let id = mint_batch_id("dev-abc");
        assert!(id.starts_with("dev-abc-01"), "got {id}");
        let tail = id.rsplit('-').next().unwrap();
        assert_eq!(tail.len(), 26, "ULID tail should be 26 chars, got {tail:?}");
    }

    #[test]
    fn batch_ids_are_unique_within_a_process() {
        let mut ids = std::collections::HashSet::new();
        for _ in 0..10_000 {
            assert!(ids.insert(mint_batch_id("dev-abc")), "duplicate batch id");
        }
        assert_eq!(ids.len(), 10_000);
    }

    #[test]
    fn batch_id_sanitizes_device_id_for_filesystem_use() {
        let id = mint_batch_id("../../etc/passwd");
        assert!(!id.contains('/'), "separator leaked into the id: {id}");
        assert!(id.starts_with("..______etc_passwd-"), "got {id}");
    }

    #[test]
    fn batch_id_falls_back_when_device_id_is_empty() {
        let id = mint_batch_id("");
        assert!(id.starts_with("device-"), "got {id}");
    }

    // ---- batching ----

    #[tokio::test]
    async fn batch_size_triggers_a_flush() {
        let sender = RecordingSender::shared();
        let (_tmp, store) = temp_pending(4);
        let c = ctx(sender.clone(), &store, 3);
        let (tx, rx) = crossbeam_channel::bounded(8);
        for e in entries(3) {
            tx.send(e).unwrap();
        }
        drop(tx);
        drive(rx, c).await;
        let accepted = sender.accepted();
        assert_eq!(accepted.len(), 1, "expected one batch, got {accepted:?}");
        assert_eq!(accepted[0].1, entries(3));
    }

    #[tokio::test]
    async fn interval_flushes_a_partial_batch() {
        let sender = RecordingSender::shared();
        let (_tmp, store) = temp_pending(4);
        let c = ctx(sender.clone(), &store, 100);
        let (tx, rx) = crossbeam_channel::bounded(8);
        tx.send("only-one".to_string()).unwrap();
        drop(tx);
        drive(rx, c).await;
        assert_eq!(sender.accepted().len(), 1);
    }

    #[tokio::test]
    async fn interval_with_empty_buffer_sends_nothing() {
        let sender = RecordingSender::shared();
        let (_tmp, store) = temp_pending(4);
        let c = ctx(sender.clone(), &store, 100);
        let (tx, rx) = crossbeam_channel::bounded(8);
        drop(tx);
        drive(rx, c).await;
        assert!(
            sender.accepted().is_empty(),
            "an empty batch would earn a 400 and wedge the drain: {:?}",
            sender.accepted()
        );
    }

    // ---- failure + pending ----

    #[tokio::test]
    async fn failed_fresh_batch_is_persisted_under_its_batch_id() {
        let sender = RecordingSender::shared();
        sender.fail_everything();
        let (tmp, store) = temp_pending(4);
        let c = ctx(sender.clone(), &store, 2);
        let (tx, rx) = crossbeam_channel::bounded(8);
        for e in entries(2) {
            tx.send(e).unwrap();
        }
        drop(tx);
        drive(rx, c).await;
        let ids = store.list_newest_first();
        assert_eq!(ids.len(), 1, "expected one persisted batch, got {ids:?}");
        assert!(ids[0].starts_with("dev-test-01"), "got {ids:?}");
        assert_eq!(store.load(&ids[0]).unwrap(), entries(2));
        drop(tmp);
    }

    #[tokio::test]
    async fn pending_batches_are_retried_newest_first_and_removed_on_success() {
        let dir = tempfile::tempdir().unwrap();
        let store = PendingStore::new(dir.path().to_path_buf(), 10);
        store.create_dir().unwrap();
        store.persist("dev-A01HQ", &["old".to_string()]).unwrap();
        store.persist("dev-A09HQ", &["new".to_string()]).unwrap();

        let sender = RecordingSender::shared();
        let c = ctx(sender.clone(), &store, 100);
        let (tx, rx) = crossbeam_channel::bounded(8);
        drop(tx);
        drive(rx, c).await;

        let ids: Vec<String> = sender.accepted().into_iter().map(|(id, _)| id).collect();
        assert_eq!(ids, vec!["dev-A09HQ".to_string(), "dev-A01HQ".to_string()]);
        assert!(store.list_newest_first().is_empty(), "pending dir should drain");
    }

    #[tokio::test]
    async fn drain_stops_at_the_first_failure_and_holds_the_fresh_batch() {
        let dir = tempfile::tempdir().unwrap();
        let store = PendingStore::new(dir.path().to_path_buf(), 10);
        store.create_dir().unwrap();
        store.persist("dev-A01HQ", &["older".to_string()]).unwrap();
        store.persist("dev-A09HQ", &["newer".to_string()]).unwrap();

        let sender = RecordingSender::shared();
        sender.fail_for(&["dev-A09HQ"]);
        let c = ctx(sender.clone(), &store, 1);
        let (tx, rx) = crossbeam_channel::bounded(8);
        tx.send("fresh".to_string()).unwrap();
        drop(tx);
        drive(rx, c).await;

        assert!(sender.accepted().is_empty(), "nothing should be accepted: {:?}", sender.accepted());
        let left = store.list_newest_first();
        assert_eq!(
            left,
            vec!["dev-A09HQ".to_string(), "dev-A01HQ".to_string()],
            "the older pending file must not be touched"
        );
    }

    #[tokio::test]
    async fn pending_drain_continues_past_already_ingested_batch() {
        let dir = tempfile::tempdir().unwrap();
        let store = PendingStore::new(dir.path().to_path_buf(), 10);
        store.create_dir().unwrap();
        store.persist("dev-A01HQ", &["older".to_string()]).unwrap();
        store.persist("dev-A09HQ", &["newer".to_string()]).unwrap();

        let sender = Arc::new(AlreadyIngestedSender::default());
        let c = ctx(sender.clone(), &store, 1);
        let (tx, rx) = crossbeam_channel::bounded(8);
        tx.send("fresh".to_string()).unwrap();
        drop(tx);
        drive(rx, c).await;

        let seen = sender.seen.lock().unwrap().clone();
        assert_eq!(
            seen,
            vec!["dev-A09HQ".to_string(), "dev-A01HQ".to_string(), seen.last().cloned().unwrap()],
            "drain must continue past a 409, not stop on it"
        );
        assert!(
            store.list_newest_first().is_empty(),
            "an already-ingested batch is done; its file must go"
        );
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p logging-utils log_submitter::submitter`
Expected: FAIL — `mint_batch_id` not found

- [ ] **Step 3: Write the implementation**

Prepend to `submitter.rs`:

```rust
//! Batch id minting and the worker's flush loop.
//!
//! `run_worker` is a free function taking its dependencies so it can
//! be driven directly from tests without spawning a thread; Task 6
//! wraps it in one.

use std::sync::Arc;
use std::time::Duration;

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender};
use ulid::Ulid;

use super::pending::PendingStore;
use super::sender::{LogSender, SubmitterError};

/// Mint a batch id: `{device_id}-{ULID}`.
///
/// The ULID half is what makes the id unique within a process (a
/// 48-bit millisecond timestamp plus an 80-bit monotonic counter and
/// random component) and lexicographically time-sortable, which is
/// how the pending dir finds the newest batch without reading mtimes.
///
/// Uniqueness is not cosmetic. The server rejects a repeated id with
/// `409 duplicate_batch_id`, and the drain stops at its first
/// failure — a collision would re-persist unchanged and wedge the
/// queue on every future cycle.
pub(crate) fn mint_batch_id(device_id: &str) -> String {
    // The batch id becomes a filename, so anything a filesystem
    // could read as a path separator (or as anything else special)
    // is folded to `_`.
    let safe: String = device_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let safe = if safe.is_empty() {
        "device".to_string()
    } else {
        safe
    };
    format!("{safe}-{}", Ulid::generate())
}

/// Everything the worker needs that is not the channel.
pub(crate) struct WorkerCtx {
    pub(crate) device_id: String,
    pub(crate) batch_size: usize,
    pub(crate) interval: Duration,
    pub(crate) pending: PendingStore,
    pub(crate) sender: Arc<dyn LogSender>,
}

/// Body of the worker task. Runs on the submitter's own
/// current-thread runtime.
///
/// The blocking `recv_timeout` is deliberate and safe here: this
/// runtime exists to run this one task, and the block is only
/// reached between `send().await` calls, so an in-flight submit
/// never stalls the receiver — it parks in the crossbeam buffer
/// until the loop comes back around.
pub(crate) async fn run_worker(rx: Receiver<String>, ctx: WorkerCtx, done_tx: Sender<()>) {
    let mut batch: Vec<String> = Vec::with_capacity(ctx.batch_size);
    loop {
        match rx.recv_timeout(ctx.interval) {
            Ok(entry) => {
                batch.push(entry);
                if batch.len() >= ctx.batch_size {
                    flush(&ctx, &mut batch).await;
                }
            }
            Err(RecvTimeoutError::Timeout) => flush(&ctx, &mut batch).await,
            Err(RecvTimeoutError::Disconnected) => {
                flush(&ctx, &mut batch).await;
                break;
            }
        }
    }
    let _ = done_tx.send(());
}

/// Drain the pending directory newest-first, then submit `batch`.
///
/// The drain stops at its first failure: newer-first ordering means
/// a batch the server will never accept would otherwise block every
/// older one and every future batch behind it. The consequence is
/// head-of-line blocking during a sustained outage, which the spec
/// accepts in exchange for the ordering guarantee.
async fn flush(ctx: &WorkerCtx, batch: &mut Vec<String>) {
    for batch_id in ctx.pending.list_newest_first() {
        let Ok(entries) = ctx.pending.load(&batch_id) else {
            // Unreadable or corrupt — it will never become
            // submittable, so drop it rather than retry it forever.
            ctx.pending.remove(&batch_id);
            continue;
        };
        match ctx.sender.send(&batch_id, entries).await {
            Ok(()) => ctx.pending.remove(&batch_id),
            Err(err) => {
                if ctx.sender.is_already_ingested(&err) {
                    // The sink already holds this batch, so the
                    // entries are not lost — the file is just
                    // stale. Delete it and keep draining.
                    ctx.pending.remove(&batch_id);
                    continue;
                }
                return;
            }
        }
    }

    if batch.is_empty() {
        return;
    }
    let batch_id = mint_batch_id(&ctx.device_id);
    let entries = std::mem::take(batch);
    if let Err(err) = ctx.sender.send(&batch_id, entries.clone()).await {
        if ctx.sender.is_already_ingested(&err) {
            return;
        }
        if let Err(persist_err) = ctx.pending.persist(&batch_id, &entries) {
            // The disk refused the batch. There is nowhere left to
            // keep it, and retrying in memory would grow without
            // bound, so it is dropped — loudly, because a silent
            // loss is the worst outcome here.
            tracing::warn!(
                batch_id = %batch_id,
                error = %persist_err,
                send_error = %err,
                "log batch could not be sent or persisted; dropping it"
            );
        }
    }
}
```

- [ ] **Step 4: Make `PendingStore` cheaply cloneable for tests**

The test helper calls `pending.clone_store()`. Add to `pending.rs`, inside `impl PendingStore`:

```rust
    /// A second handle to the same directory. The store is just a
    /// path and a cap, so this is a plain clone.
    #[cfg(test)]
    pub(crate) fn clone_store(&self) -> PendingStore {
        PendingStore {
            dir: self.dir.clone(),
            max_files: self.max_files,
        }
    }
```

- [ ] **Step 5: Declare the module**

In `log_submitter.rs`, add `mod submitter;`.

- [ ] **Step 6: Add `tokio` to the crate**

In `lib/crates/logging-utils/Cargo.toml`, add to `[dependencies]`:

```toml
# `tokio` supplies the current-thread runtime the worker thread
# runs the async `LogSender` on. `rt` is all that is needed — the
# batching timer is crossbeam's `recv_timeout`, not a tokio timer.
tokio = { workspace = true, features = ["rt"] }
```

- [ ] **Step 7: Run test to verify it passes**

Run: `cargo test -p logging-utils log_submitter::submitter`
Expected: PASS (13 tests)

- [ ] **Step 8: Commit**

```bash
git add lib/crates/logging-utils/Cargo.toml lib/crates/logging-utils/src/log_submitter.rs lib/crates/logging-utils/src/log_submitter/pending.rs lib/crates/logging-utils/src/log_submitter/submitter.rs
git commit -m "$(cat <<'EOF'
feat(logging-utils): batch id minting + the flush/drain worker

run_worker batches on size or on buffer inactivity and flushes by
draining the pending dir newest-first, stopping at the first
failure, then submitting the fresh batch and persisting it on
failure.

Two guards that the obvious implementation gets wrong: an empty
buffer never flushes (the server rejects empty entries with a 400,
which under stop-on-failure would wedge the queue), and a send
error the sender recognises as "already ingested" counts as
success so a lost response does not become a permanent 409.

device_id is sanitised on the way into a batch id, because the id
is a filename.

Co-Authored-By: Claude Code <noreply@anthropic.com>
EOF
)"
```

---

### Task 6: `LogSubmitter` lifecycle and `SubmitHandle`

**Files:**
- Modify: `lib/crates/logging-utils/src/log_submitter/submitter.rs` (append)
- Modify: `lib/crates/logging-utils/src/log_submitter.rs` (re-export)

**Interfaces:**
- Consumes: `run_worker`, `WorkerCtx` (Task 5), `LogSubmitterConfig` (Task 3)
- Produces: `pub struct LogSubmitter` with `new(config: LogSubmitterConfig) -> Result<Self, SubmitterError>`, `submit(&self, entry: String) -> Result<(), SubmitterError>`, `dropped(&self) -> u64`, `handle(&self) -> SubmitHandle`, `shutdown(&mut self) -> Result<(), SubmitterError>`, plus `Drop`. And `pub struct SubmitHandle` (Clone) with `submit(&self, entry: String) -> Result<(), SubmitterError>` and `dropped(&self) -> u64`. Task 7's `SubmitWriter` holds a `SubmitHandle`; Task 10 holds a `LogSubmitter`.

- [ ] **Step 1: Write the failing test**

Append a second test module to `submitter.rs`:

```rust
#[cfg(test)]
mod lifecycle_tests {
    use super::*;
    use super::tests_support::{RecordingSender, make_config};

    fn wait_for(sender: &RecordingSender, n: usize) {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while sender.accepted().len() < n {
            assert!(
                std::time::Instant::now() < deadline,
                "timed out waiting for {n} batches; got {:?}",
                sender.accepted()
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn submit_delivers_an_entry_to_the_sender() {
        let sender = RecordingSender::shared();
        let mut sub = LogSubmitter::new(make_config(sender.clone(), 2)).unwrap();
        sub.submit("one".into()).unwrap();
        sub.submit("two".into()).unwrap();
        wait_for(&sender, 1);
        let accepted = sender.accepted();
        assert_eq!(accepted[0].1, vec!["one".to_string(), "two".to_string()]);
        sub.shutdown().unwrap();
    }

    #[test]
    fn dropped_counts_entries_lost_to_a_full_buffer() {
        let sender = RecordingSender::shared();
        let mut sub = LogSubmitter::new(make_config(sender, 1_000_000)).unwrap();
        // Fill the buffer far past capacity without ever letting the
        // worker drain it.
        for i in 0..20_000 {
            let _ = sub.submit(format!("flood-{i}"));
        }
        assert!(
            sub.dropped() > 0,
            "a full buffer should be counted, not silently swallowed"
        );
        sub.shutdown().unwrap();
    }

    #[test]
    fn shutdown_is_idempotent() {
        let sender = RecordingSender::shared();
        let mut sub = LogSubmitter::new(make_config(sender, 100)).unwrap();
        sub.shutdown().unwrap();
        sub.shutdown().unwrap();
    }

    #[test]
    fn submit_after_shutdown_reports_channel_closed() {
        let sender = RecordingSender::shared();
        let mut sub = LogSubmitter::new(make_config(sender, 100)).unwrap();
        sub.shutdown().unwrap();
        let err = sub.submit("late".into()).unwrap_err();
        assert!(matches!(err, SubmitterError::ChannelClosed), "got {err:?}");
    }

    #[test]
    fn handle_shares_the_dropped_counter() {
        let sender = RecordingSender::shared();
        let mut sub = LogSubmitter::new(make_config(sender, 100)).unwrap();
        let handle = sub.handle();
        assert_eq!(handle.dropped(), sub.dropped());
        handle.submit("via-handle".into()).unwrap();
        sub.shutdown().unwrap();
    }
}
```

To support this, the `RecordingSender` from Step 1 of Task 5 must move into a `#[cfg(test)] pub(crate) mod tests_support` with `pub(crate)` items, and Task 5's own tests import it from there. Do that as part of this step:

```rust
#[cfg(test)]
pub(crate) mod tests_support {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};

    #[derive(Debug, Default)]
    pub(crate) struct RecordingSender {
        accepted: Mutex<Vec<(String, Vec<String>)>>,
        fail_ids: Mutex<Vec<String>>,
        fail_all: AtomicBool,
    }

    impl RecordingSender {
        pub(crate) fn shared() -> Arc<RecordingSender> {
            Arc::new(RecordingSender::default())
        }
        pub(crate) fn accepted(&self) -> Vec<(String, Vec<String>)> {
            self.accepted.lock().unwrap().clone()
        }
        pub(crate) fn fail_for(&self, ids: &[&str]) {
            *self.fail_ids.lock().unwrap() = ids.iter().map(|s| s.to_string()).collect();
        }
        pub(crate) fn fail_everything(&self) {
            self.fail_all.store(true, Ordering::Release);
        }
    }

    #[async_trait::async_trait]
    impl LogSender for RecordingSender {
        async fn send(
            &self,
            batch_id: &str,
            log_entries: Vec<String>,
        ) -> Result<(), SubmitterError> {
            if self.fail_all.load(Ordering::Acquire)
                || self.fail_ids.lock().unwrap().iter().any(|i| i == batch_id)
            {
                return Err(SubmitterError::Send {
                    batch_id: batch_id.to_string(),
                    source: Box::new(std::io::Error::other("scripted failure")),
                });
            }
            self.accepted.lock().unwrap().push((batch_id.to_string(), log_entries));
            Ok(())
        }
    }

    /// A sender plus a `LogSubmitterConfig` wired to a temp dir.
    /// `batch_size` and `interval` are tuned for tests; the deadline
    /// is generous so a loaded CI box does not produce flakes.
    pub(crate) fn make_config(
        sender: Arc<RecordingSender>,
        buffer_capacity: usize,
    ) -> super::super::config::LogSubmitterConfig {
        let dir = std::env::temp_dir().join(format!(
            "aegis-submitter-test-{}-{}",
            std::process::id(),
            ulid::Ulid::generate()
        ));
        let mut cfg = super::super::config::LogSubmitterConfig::new(
            "dev-test".to_string(),
            sender,
            dir.clone(),
        );
        cfg.batch_size = 2;
        cfg.interval = Duration::from_millis(50);
        cfg.buffer_capacity = buffer_capacity;
        cfg.shutdown_deadline = Duration::from_secs(5);
        cfg
    }
}
```

and update Task 5's `mod tests` to `use super::tests_support::RecordingSender;` plus `use super::tests_support::RecordingSender as _SenderUnused;` — no; simply delete the local `RecordingSender` definition from Task 5's test module and import it.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p logging-utils log_submitter::submitter::lifecycle_tests`
Expected: FAIL — `LogSubmitter` not found

- [ ] **Step 3: Write the implementation**

Append to `submitter.rs`, after `flush`:

```rust
/// The producer side, shared with the `tracing` layer.
///
/// Cheap to clone and holds no thread handle, so the layer can keep
/// one for the process lifetime without pinning the worker alive.
#[derive(Clone)]
pub struct SubmitHandle {
    inner: Arc<Inner>,
}

struct Inner {
    tx: crossbeam_channel::Sender<String>,
    dropped: std::sync::atomic::AtomicU64,
}

impl SubmitHandle {
    /// Hand one formatted entry to the worker.
    ///
    /// A full buffer is **not** an error: this is called from a
    /// `tracing` layer's write path, where the only honest options
    /// are to drop the line or to block the thread that emitted it.
    /// The drop is counted instead, so the loss is observable via
    /// [`SubmitHandle::dropped`].
    pub fn submit(&self, entry: String) -> Result<(), SubmitterError> {
        match self.inner.tx.try_send(entry) {
            Ok(()) => Ok(()),
            Err(crossbeam_channel::TrySendError::Full(_)) => {
                self.inner.dropped.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                Ok(())
            }
            Err(crossbeam_channel::TrySendError::Disconnected(_)) => Err(SubmitterError::ChannelClosed),
        }
    }

    /// Entries lost to a full buffer since startup.
    pub fn dropped(&self) -> u64 {
        self.inner.dropped.load(std::sync::atomic::Ordering::Relaxed)
    }
}

/// The batching worker. Owns the thread; manage it for the process
/// lifetime or it stops submitting.
pub struct LogSubmitter {
    tx: Option<crossbeam_channel::Sender<String>>,
    done_rx: Option<crossbeam_channel::Receiver<()>>,
    worker: Option<std::thread::JoinHandle<()>>,
    handle: SubmitHandle,
    deadline: Duration,
}

impl LogSubmitter {
    /// Create the pending directory and spawn the worker thread.
    pub fn new(config: LogSubmitterConfig) -> Result<Self, SubmitterError> {
        let pending = PendingStore::new(config.pending_dir.clone(), config.max_pending_files);
        pending.create_dir()?;

        let (tx, rx) = crossbeam_channel::bounded::<String>(config.buffer_capacity);
        let (done_tx, done_rx) = crossbeam_channel::bounded::<()>(1);
        let ctx = WorkerCtx {
            device_id: config.device_id.clone(),
            batch_size: config.batch_size,
            interval: config.interval,
            pending,
            sender: config.sender,
        };

        // A current-thread runtime with `enable_all` cannot fail in
        // practice; `HttpClient::new` makes the same bet for its
        // client. `expect` keeps a second error variant off the
        // public surface for a case that does not occur.
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("current-thread runtime builds");

        let worker = std::thread::spawn(move || {
            runtime.block_on(run_worker(rx, ctx, done_tx));
        });

        let handle = SubmitHandle {
            inner: Arc::new(Inner {
                tx: tx.clone(),
                dropped: std::sync::atomic::AtomicU64::new(0),
            }),
        };

        Ok(Self {
            tx: Some(tx),
            done_rx: Some(done_rx),
            worker: Some(worker),
            handle,
            deadline: config.shutdown_deadline,
        })
    }

    /// A cloneable producer handle for the `tracing` layer.
    pub fn handle(&self) -> SubmitHandle {
        self.handle.clone()
    }

    /// See [`SubmitHandle::submit`].
    pub fn submit(&self, entry: String) -> Result<(), SubmitterError> {
        self.handle.submit(entry)
    }

    /// See [`SubmitHandle::dropped`].
    pub fn dropped(&self) -> u64 {
        self.handle.dropped()
    }

    /// Drain the buffer and join the worker within
    /// `shutdown_deadline`. Idempotent; also runs from `Drop`.
    pub fn shutdown(&mut self) -> Result<(), SubmitterError> {
        if self.worker.is_none() {
            return Ok(());
        }
        // Dropping `tx` closes the channel; the worker sees
        // `Disconnected`, flushes what it has, and signals done.
        self.tx.take();
        let done_rx = self.done_rx.take().expect("done_rx present iff worker is");
        let worker = self
            .worker
            .take()
            .expect("worker present iff not yet shut down");

        match done_rx.recv_timeout(self.deadline) {
            Ok(()) => match worker.join() {
                Ok(()) => Ok(()),
                Err(payload) => Err(SubmitterError::WorkerPanic(panic_message(&payload))),
            },
            Err(_) => {
                if worker.is_finished() {
                    match worker.join() {
                        Ok(()) => Err(SubmitterError::WorkerJoinTimeout {
                            deadline: self.deadline,
                            message: "worker exited without signalling done".into(),
                        }),
                        Err(payload) => Err(SubmitterError::WorkerPanic(panic_message(&payload))),
                    }
                } else {
                    Err(SubmitterError::WorkerJoinTimeout {
                        deadline: self.deadline,
                        message: "worker thread did not exit in time".into(),
                    })
                }
            }
        }
    }
}

impl Drop for LogSubmitter {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

/// Best-effort conversion of a panic payload to a UTF-8 string;
/// mirrors the helper in `log_ingestor`.
fn panic_message(payload: &Box<dyn std::any::Any + Send + 'static>) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        format!("{payload:?}")
    }
}
```

- [ ] **Step 4: Import the config type and re-export**

At the top of `submitter.rs`, add `use super::config::LogSubmitterConfig;`.

In `log_submitter.rs`, change the re-exports to:

```rust
mod config;
mod pending;
mod sender;
mod submitter;

pub use config::{LogSubmitterConfig, LogSubmitterConfigBuilder};
pub use sender::{LogSender, SubmitterError};
pub use submitter::{LogSubmitter, SubmitHandle};
```

and in `lib.rs` change the re-export line to:

```rust
pub use log_submitter::{LogSender, LogSubmitter, LogSubmitterConfig, LogSubmitterConfigBuilder, SubmitHandle, SubmitterError};
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test -p logging-utils log_submitter`
Expected: PASS (18 tests)

- [ ] **Step 6: Commit**

```bash
git add lib/crates/logging-utils/src/lib.rs lib/crates/logging-utils/src/log_submitter.rs lib/crates/logging-utils/src/log_submitter/submitter.rs
git commit -m "$(cat <<'EOF'
feat(logging-utils): add LogSubmitter lifecycle + SubmitHandle

A dedicated OS thread runs a current-thread tokio runtime hosting
run_worker, so shutdown is a synchronous join rather than a
Drop-time block_on that would panic inside a runtime. shutdown is
idempotent and runs from Drop, matching LogIngestor.

SubmitHandle is the cheap producer side the tracing layer holds. A
full buffer counts the drop instead of erroring: the layer runs in
a write path where the only alternatives are dropping the line or
blocking the emitting thread.

Co-Authored-By: Claude Code <noreply@anthropic.com>
EOF
)"
```

---

### Task 7: the submit layer

**Files:**
- Create: `lib/crates/logging-utils/src/log_submitter/layer.rs`
- Modify: `lib/crates/logging-utils/src/log_submitter.rs`

**Interfaces:**
- Consumes: `SubmitHandle` (Task 6)
- Produces: `pub struct SubmitWriter { handle: SubmitHandle }`, `pub struct EntryWriter`, `pub type SubmitLayer = tracing_subscriber::fmt::Layer<tracing_subscriber::Registry, tracing_subscriber::fmt::Json, SubmitWriter>`, `pub fn submit_layer(handle: SubmitHandle) -> SubmitLayer`. Task 8 and Task 10 consume `submit_layer`.

- [ ] **Step 1: Write the failing test**

Create `layer.rs` with only the test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::log_submitter::tests_support::{RecordingSender, make_config};
    use crate::log_submitter::LogSubmitter;
    use std::sync::Arc;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    /// Install a registry carrying only the submit layer, emit, and
    /// return the submitter driving it.
    fn capture(sender: Arc<RecordingSender>) -> LogSubmitter {
        let submitter = LogSubmitter::new(make_config(sender, 256)).unwrap();
        let layer = submit_layer(submitter.handle());
        let _ = tracing_subscriber::registry()
            .with(layer)
            .try_init();
        submitter
    }

    fn await_entries(sender: &RecordingSender, n: usize) -> Vec<String> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            let flat: Vec<String> = sender
                .accepted()
                .into_iter()
                .flat_map(|(_, entries)| entries)
                .collect();
            if flat.len() >= n {
                return flat;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "timed out waiting for {n} entries; got {flat:?}"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    #[test]
    fn captured_entry_is_the_json_formatted_event() {
        let sender = RecordingSender::shared();
        let mut submitter = capture(sender.clone());
        tracing::info!(request_id = "abc", "hello from the layer");
        let flat = await_entries(&sender, 1);
        let value: serde_json::Value =
            serde_json::from_str(&flat[0]).expect("entry should be valid JSON");
        assert_eq!(value["message"], "hello from the layer");
        assert_eq!(value["request_id"], "abc");
        submitter.shutdown().unwrap();
    }

    #[test]
    fn two_events_produce_two_entries() {
        let sender = RecordingSender::shared();
        let mut submitter = capture(sender.clone());
        tracing::info!("first");
        tracing::info!("second");
        let flat = await_entries(&sender, 2);
        let messages: Vec<_> = flat
            .iter()
            .map(|e| serde_json::from_str::<serde_json::Value>(e).unwrap()["message"].as_str().unwrap().to_string())
            .collect();
        assert!(messages.contains(&"first".to_string()), "got {messages:?}");
        assert!(messages.contains(&"second".to_string()), "got {messages:?}");
        submitter.shutdown().unwrap();
    }

    #[test]
    fn captured_entries_carry_no_trailing_newline() {
        let sender = RecordingSender::shared();
        let mut submitter = capture(sender.clone());
        tracing::info!("trimmed");
        let flat = await_entries(&sender, 1);
        assert!(!flat[0].ends_with('\n'), "got {:?}", flat[0]);
        assert!(!flat[0].ends_with('\r'), "got {:?}", flat[0]);
        submitter.shutdown().unwrap();
    }
}
```

Note: the file layer and the submit layer must produce the *same* format, so this test's expectations (a `message` key, and the event's own fields at the top level) are the contract that `init_tracing` must not break in Task 8.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p logging-utils log_submitter::layer`
Expected: FAIL — `submit_layer` not found

- [ ] **Step 3: Write the implementation**

Prepend to `layer.rs`:

```rust
//! The `tracing` layer that feeds the submitter.
//!
//! This deliberately does **not** hand-roll a JSON serializer. It
//! installs a second `tracing_subscriber::fmt::Layer` configured
//! exactly like the file layer and swaps the writer, so a shipped
//! entry is byte-identical to the corresponding line in the local
//! log file. Two formatters would drift; one formatter with two
//! writers cannot.

use std::io;

use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::fmt::format::Json;
use tracing_subscriber::fmt::Layer as FmtLayer;
use tracing_subscriber::Registry;

use super::submitter::SubmitHandle;

/// The concrete layer type produced by [`submit_layer`].
pub type SubmitLayer = FmtLayer<Registry, Json, SubmitWriter>;

/// Mints one [`EntryWriter`] per event, which is what makes each
/// entry exactly one formatted event rather than a coalesced blob.
#[derive(Clone, Debug)]
pub struct SubmitWriter {
    handle: SubmitHandle,
}

impl<'a> MakeWriter<'a> for SubmitWriter {
    type Writer = EntryWriter;
    fn make_writer(&'a self, _meta: &tracing::Metadata<'a>) -> Self::Writer {
        EntryWriter {
            handle: self.handle.clone(),
            buf: Vec::new(),
        }
    }
}

/// Accumulates one event's bytes, then submits them on drop.
#[derive(Debug)]
pub struct EntryWriter {
    handle: SubmitHandle,
    buf: Vec<u8>,
}

impl io::Write for EntryWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buf.extend_from_slice(buf);
        // Always report success. Returning an error here would make
        // `fmt::Layer` treat a full submit buffer as a logging
        // failure; the honest response to backpressure is to drop
        // the line, which `SubmitHandle` counts.
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Drop for EntryWriter {
    fn drop(&mut self) {
        if self.buf.is_empty() {
            return;
        }
        let text = String::from_utf8_lossy(&self.buf);
        let entry = text.trim_end_matches(['\r', '\n']);
        if entry.is_empty() {
            return;
        }
        let _ = self.handle.submit(entry.to_string());
    }
}

/// Build the layer. Configure it to match the file layer:
/// `.json().with_current_span(true).with_span_list(false)`.
pub fn submit_layer(handle: SubmitHandle) -> SubmitLayer {
    tracing_subscriber::fmt::layer()
        .json()
        .with_current_span(true)
        .with_span_list(false)
        .with_writer(SubmitWriter { handle })
}
```

- [ ] **Step 4: Make `tests_support` reachable from the layer's tests**

`tests_support` currently lives inside `submitter.rs`. Make it crate-visible and re-export it for tests. In `submitter.rs` change `mod tests_support` to `pub(crate) mod tests_support` (it is already behind `#[cfg(test)]`). In `log_submitter.rs` add:

```rust
#[cfg(test)]
pub(crate) use submitter::tests_support;
```

- [ ] **Step 5: Declare the module**

In `log_submitter.rs`, add `mod layer;` and extend the re-exports:

```rust
pub use layer::{SubmitLayer, SubmitWriter, submit_layer};
```

- [ ] **Step 6: Run test to verify it passes**

Run: `cargo test -p logging-utils log_submitter::layer`
Expected: PASS (3 tests)

- [ ] **Step 7: Commit**

```bash
git add lib/crates/logging-utils/src/log_submitter.rs lib/crates/logging-utils/src/log_submitter/layer.rs
git commit -m "$(cat <<'EOF'
feat(logging-utils): add the submit tracing layer

A second fmt::Layer with the file layer's exact configuration and
a MakeWriter swapped in, so a shipped entry is byte-identical to
the local log line. Hand-rolling a field visitor would have meant
two formatters to keep in step.

The writer reports success even when the buffer is full: an
io::Error here would make fmt::Layer treat backpressure as a
logging failure, and the layer runs in a write path where the only
other option is blocking the emitting thread.

Co-Authored-By: Claude Code <noreply@anthropic.com>
EOF
)"
```

---

### Task 8: `init_tracing` accepts extra layers

**Files:**
- Modify: `lib/crates/logging-utils/src/tracing_init.rs:42-57` (the `init_tracing` body) and the three call sites in its test module
- Modify: `apps/server/aegis-server/src/run.rs` (the `init_tracing` call)

**Interfaces:**
- Consumes: `Box<dyn Layer<Registry> + Send + Sync>` items — Task 10 passes `submit_layer(..)`.
- Produces: `pub fn init_tracing(config: &LoggingConfig, layers: Vec<Box<dyn Layer<Registry> + Send + Sync>>) -> Result<LogGuard, LoggingInitError>`

- [ ] **Step 1: Write the failing test**

In `tracing_init.rs`'s test module, update the `cfg` helper and add a test:

```rust
    fn cfg(dir: &Path, prefix: &str) -> LoggingConfig {
        LoggingConfig {
            log_dir: dir.to_path_buf(),
            file_name_prefix: prefix.to_string(),
        }
    }

    fn no_layers() -> Vec<Box<dyn tracing_subscriber::Layer<tracing_subscriber::Registry> + Send + Sync>> {
        Vec::new()
    }
```

Then change every existing `init_tracing(&cfg(...))` to `init_tracing(&cfg(...), no_layers())`, and add:

```rust
    #[test]
    fn init_tracing_installs_extra_layers() {
        use std::sync::{Arc, Mutex};
        use tracing_subscriber::layer::SubscriberExt;

        let tmp = tempfile::tempdir().unwrap();
        let _g = lock_env();
        let _lvl = set_env("AEGIS_LOG_LEVEL", "info");

        #[derive(Clone, Default)]
        struct Counter(Arc<Mutex<usize>>);
        impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for Counter {
            fn on_event(&self, _ev: &tracing::Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
                *self.0.lock().unwrap() += 1;
            }
        }

        let counter = Counter::default();
        let _guard = init_tracing(&cfg(tmp.path(), "aegis-layers.log"), vec![Box::new(counter.clone())])
            .expect("init_tracing succeeds");
        tracing::info!("counted");
        drop(_guard);
        assert_eq!(*counter.0.lock().unwrap(), 1, "the extra layer should see the event");
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p logging-utils tracing_init`
Expected: FAIL — `init_tracing` takes 1 argument but 2 were supplied

- [ ] **Step 3: Write the implementation**

Replace the body of `init_tracing`:

```rust
pub fn init_tracing(
    config: &LoggingConfig,
    layers: Vec<Box<dyn Layer<Registry> + Send + Sync>>,
) -> Result<LogGuard, LoggingInitError> {
    std::fs::create_dir_all(&config.log_dir).map_err(|source| LoggingInitError::CreateDir {
        dir: config.log_dir.clone(),
        source,
    })?;
    let file_appender = tracing_appender::rolling::daily(&config.log_dir, &config.file_name_prefix);
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    let fmt_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_current_span(true)
        .with_span_list(false)
        .with_writer(non_blocking);
    // The filter goes on the *registry*, not on the file layer, so
    // it gates every layer. Left on `fmt_layer` it would apply only
    // to the file, and the submit layer would ship DEBUG events
    // while the process is running at `info`.
    let _ = tracing_subscriber::registry()
        .with(build_filter())
        .with(fmt_layer)
        .with(layers)
        .try_init();
    Ok(LogGuard(guard))
}
```

and change the imports at the top of the file to:

```rust
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{EnvFilter, Layer, Registry};
```

- [ ] **Step 4: Update the server call site**

In `apps/server/aegis-server/src/run.rs`, find the `init_tracing` call and append the second argument:

```rust
    let _guard = logging_utils::init_tracing(
        &logging_utils::LoggingConfig {
            log_dir: log_dir.clone(),
            file_name_prefix: "aegis-server.log".into(),
        },
        // The server is the sink for client logs, not a client: it
        // has nothing to submit.
        Vec::new(),
    )
    .map_err(|e| format!("init_tracing: {e}"))?;
```

(Preserve whatever the existing call site actually says for the config and error handling — only the new argument is the change.)

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p logging-utils tracing_init && cargo test -p aegis-server`
Expected: PASS

- [ ] **Step 6: Run clippy and fmt**

Run: `cargo clippy -p logging-utils --all-targets --all-features -- -D warnings && cargo clippy -p aegis-server --all-targets --all-features -- -D warnings && cargo fmt --all -- --check`
Expected: clean

- [ ] **Step 7: Commit**

```bash
git add lib/crates/logging-utils/src/tracing_init.rs apps/server/aegis-server/src/run.rs
git commit -m "$(cat <<'EOF'
feat(logging-utils): let init_tracing take extra layers

Callers that need a second sink — the desktop's log submitter —
pass a boxed Layer; the server passes an empty vec and is
unchanged.

The EnvFilter moves from the file layer onto the registry. Left
on the fmt layer it would gate only the file, and the submit layer
would ship DEBUG events while the process runs at info.

Co-Authored-By: Claude Code <noreply@anthropic.com>
EOF
)"
```

---

### Task 9: `HttpLogSender` in aegis-desktop

**Files:**
- Create: `apps/desktop/aegis-desktop/src-tauri/src/http/log_ingest.rs`
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/http.rs` (add `pub mod log_ingest;`)

**Interfaces:**
- Consumes: `LogSender`, `SubmitterError` (Task 2), `ApiError` (existing), `HttpClient` (existing)
- Produces: `pub struct HttpLogSender { client: Arc<HttpClient> }` with `new(client: Arc<HttpClient>) -> Self`. Task 10 wraps it in `Arc` and puts it in `LogSubmitterConfig::sender`.

- [ ] **Step 1: Write the failing test**

Create `log_ingest.rs` with only the test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::client::MemoryStore;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn sender_for(server: &MockServer) -> HttpLogSender {
        HttpLogSender::new(Arc::new(HttpClient::new(
            server.uri(),
            Arc::new(MemoryStore::default()),
        )))
    }

    fn entries() -> Vec<String> {
        vec!["{\"level\":\"info\"}".to_string(), "{\"level\":\"warn\"}".to_string()]
    }

    #[tokio::test]
    async fn sends_a_camel_case_body_to_the_submit_endpoint() {
        let server = MockServer::start().await;
        server
            .register(
                Mock::given(method("POST"))
                    .and(path("/api/log-ingest/submit"))
                    .and(header("authorization", "Bearer AT"))
                    .respond_with(
                        ResponseTemplate::new(200)
                            .set_body_json(serde_json::json!({"batchId": "dev-1", "accepted": 2})),
                    ),
            )
            .await;
        let store = Arc::new(MemoryStore::default());
        store.set_access_token("AT").await.unwrap();
        let s = HttpLogSender::new(Arc::new(HttpClient::new(
            server.uri(),
            store,
        )));

        s.send("dev-1", entries()).await.unwrap();

        let requests = server.received_requests().await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
        assert_eq!(body["batchId"], "dev-1");
        assert_eq!(body["entries"].as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn maps_a_server_error_to_submit_error_send() {
        let server = MockServer::start().await;
        server
            .register(
                Mock::given(method("POST"))
                    .and(path("/api/log-ingest/submit"))
                    .respond_with(ResponseTemplate::new(500).set_body_string("boom")),
            )
            .await;
        let err = sender_for(&server).send("dev-1", entries()).await.unwrap_err();
        match &err {
            SubmitterError::Send { batch_id, source } => {
                assert_eq!(batch_id, "dev-1");
                let api = source
                    .downcast_ref::<ApiError>()
                    .expect("the ApiError should survive as the source");
                assert!(matches!(api, ApiError::Http { status: 500, .. }), "got {api:?}");
            }
            other => panic!("expected Send, got {other:?}"),
        }
    }

    #[test]
    fn recognizes_a_duplicate_batch_id_as_already_ingested() {
        let s = HttpLogSender::new(Arc::new(HttpClient::new(
            "http://127.0.0.1:1".into(),
            Arc::new(MemoryStore::default()),
        )));
        let duplicate = SubmitterError::Send {
            batch_id: "dev-1".into(),
            source: Box::new(ApiError::Http {
                status: 409,
                code: "duplicate_batch_id".into(),
                message: "already ingested".into(),
            }),
        };
        let other = SubmitterError::Send {
            batch_id: "dev-1".into(),
            source: Box::new(ApiError::Http {
                status: 500,
                code: "internal".into(),
                message: "boom".into(),
            }),
        };
        assert!(s.is_already_ingested(&duplicate));
        assert!(!s.is_already_ingested(&other));
        assert!(!s.is_already_ingested(&SubmitterError::ChannelClosed));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p aegis-desktop --lib log_ingest`
Expected: FAIL — `HttpLogSender` not found

- [ ] **Step 3: Write the implementation**

Prepend to `log_ingest.rs`:

```rust
//! `LogSender` over the existing `HttpClient`.
//!
//! There is deliberately no second request client here. `HttpClient`
//! already carries the Bearer header and the 401 auto-refresh; a
//! second one would silently stop submitting the moment an access
//! token expired — which, for a log submitter, is exactly when you
//! most want the logs.

use std::sync::Arc;

use async_trait::async_trait;
use logging_utils::{LogSender, SubmitterError};
use serde::{Deserialize, Serialize};

use super::client::HttpClient;
use super::dto::ApiError;

/// The server's error `code` for a batch it has already ingested.
const DUPLICATE_BATCH_ID: &str = "duplicate_batch_id";

/// Submits batches to `POST /api/log-ingest/submit`.
#[derive(Debug)]
pub struct HttpLogSender {
    client: Arc<HttpClient>,
}

/// The wire body. `camelCase` because the server's DTO is.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SubmitBody<'a> {
    batch_id: &'a str,
    entries: &'a [String],
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SubmitResponse {
    #[allow(dead_code)]
    batch_id: String,
    #[allow(dead_code)]
    accepted: usize,
}

impl HttpLogSender {
    pub fn new(client: Arc<HttpClient>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl LogSender for HttpLogSender {
    async fn send(
        &self,
        batch_id: &str,
        log_entries: Vec<String>,
    ) -> Result<(), SubmitterError> {
        let body = SubmitBody {
            batch_id,
            entries: &log_entries,
        };
        let _: SubmitResponse = self
            .client
            .request(
                reqwest::Method::POST,
                "/api/log-ingest/submit",
                Some(&body),
            )
            .await
            .map_err(|source| SubmitterError::Send {
                batch_id: batch_id.to_string(),
                source: Box::new(source),
            })?;
        Ok(())
    }

    fn is_already_ingested(&self, err: &SubmitterError) -> bool {
        match err {
            SubmitterError::Send { source, .. } => source.downcast_ref::<ApiError>().is_some_and(
                |e| matches!(e, ApiError::Http { code, .. } if code == DUPLICATE_BATCH_ID),
            ),
            _ => false,
        }
    }
}
```

- [ ] **Step 4: Register the module**

In `apps/desktop/aegis-desktop/src-tauri/src/http.rs`, add `pub mod log_ingest;` in alphabetical position (after `pub mod healthz;`).

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p aegis-desktop --lib log_ingest`
Expected: PASS (3 tests)

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/http.rs apps/desktop/aegis-desktop/src-tauri/src/http/log_ingest.rs
git commit -m "$(cat <<'EOF'
feat(aegis-desktop): add HttpLogSender over the existing HttpClient

Wraps HttpClient rather than building a second reqwest client: the
existing client already attaches the Bearer header and
auto-refreshes on 401, and a fresh one would quietly stop
submitting exactly when a token expires.

is_already_ingested downcasts the ApiError and reports
duplicate_batch_id as "the server has it", so a batch whose
response was lost in transit is deleted from the pending dir on
retry instead of blocking the drain forever.

Co-Authored-By: Claude Code <noreply@anthropic.com>
EOF
)"
```

---

### Task 10: wire the submitter into the desktop app

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/lib.rs:127-175` (the `setup` closure)
- Modify: `lib/crates/logging-utils/README.md`

**Interfaces:**
- Consumes: `LogSubmitter`, `LogSubmitterConfig`, `submit_layer`, `SubmitHandle` (Tasks 3, 6, 7), `HttpLogSender` (Task 9), `init_tracing(&cfg, layers)` (Task 8), `TraceIdGenerator::device_prefix()` (Task 1)
- Produces: nothing downstream — this is the wiring that makes the feature live.

- [ ] **Step 1: Reorder and extend `setup`**

Replace the body of the `.setup(|app| { ... })` closure in `lib.rs` with:

```rust
        .setup(|app| {
            // Order matters: the submitter needs the HTTP client, and
            // the submit layer must be installed in the same
            // `init_tracing` call that installs the file layer —
            // `try_init` only takes effect once per process. So the
            // client is built first, and the only logs we lose are
            // the ones emitted before `setup` runs, which the
            // current code already loses.
            let store = app
                .store("auth.bin")
                .map_err(|e| format!("failed to open auth.bin store: {e}"))?;
            let tokens = Arc::new(http::client::TauriStore::new(store));
            let client =
                http::client::HttpClient::new(http::config::BASE_URL.to_string(), tokens);
            app.manage(Arc::new(client.clone()));

            // Per-install device prefix: persisted in app-data dir
            // so a workstation keeps a stable middle segment on
            // every trace id it emits, and on every batch id the
            // submitter mints. load_or_create is best-effort and
            // falls back to a freshly-minted, non-persisted prefix
            // on I/O errors.
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

            // Batched submission of this process's own logs to the
            // server. Failure mode is a bounded backlog under
            // `pending-logs`, drained newest-first.
            let submitter = logging_utils::LogSubmitter::new(
                logging_utils::LogSubmitterConfig::builder()
                    .device_id(
                        generator
                            .device_prefix()
                            .unwrap_or("desktop")
                            .to_string(),
                    )
                    .sender(Arc::new(http::log_ingest::HttpLogSender::new(Arc::new(
                        client,
                    ))))
                    .pending_dir(app_data_dir.join("pending-logs"))
                    .build(),
            )
            .map_err(|e| format!("log submitter init: {e}"))?;

            // Tracing init: prefer $AEGIS_LOG_DIR; fall back to
            // <app_data_dir>/logs. LogGuard is stashed in managed
            // state so the buffered writer lives for the process.
            let log_dir = std::env::var("AEGIS_LOG_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| app_data_dir.join("logs"));
            let log_guard = logging_utils::init_tracing(
                &logging_utils::LoggingConfig {
                    log_dir: log_dir.clone(),
                    file_name_prefix: "aegis-desktop.log".into(),
                },
                vec![Box::new(logging_utils::submit_layer(
                    submitter.handle(),
                ))],
            )
            .map_err(|e| format!("init_tracing: {e}"))?;
            tracing::info!(
                log_dir = %log_dir.display(),
                "aegis-desktop tracing initialised"
            );
            app.manage(log_guard);
            app.manage(submitter);
            Ok(())
        })
```

Two notes on the code above:

- `client` is cloned into managed state *and* moved into the sender, so the command shims and the submitter share one `HttpClient` (and therefore one refresh lock).
- The submitter is managed *after* `init_tracing` so it outlives the log guard's flush path; the `submit_layer` holds a `SubmitHandle`, not the submitter, so the layer does not keep the worker thread alive on its own.

- [ ] **Step 2: Build the desktop crate**

Run: `cargo check -p aegis-desktop`
Expected: clean. If `client` is not `Clone`, wrap it in an `Arc` at construction instead and adjust the two uses above to share that `Arc`.

- [ ] **Step 3: Run the full desktop test suite**

Run: `cargo test -p aegis-desktop --lib`
Expected: PASS

- [ ] **Step 4: Document the module**

Add to `lib/crates/logging-utils/README.md`, after the "Log ingestor (aegis-desktop only)" section:

````markdown
## Log submitter (aegis-desktop only)

The client-side counterpart to `LogIngestor`: a bounded buffer, a
batching worker on its own thread, and a pending directory for
batches the server would not take.

```rust
use logging_utils::{LogSubmitter, LogSubmitterConfig, submit_layer};

// The sender is whatever can reach the server. In the desktop app
// that is `HttpLogSender`, which wraps the existing `HttpClient` so
// submissions inherit its 401 auto-refresh.
let submitter = LogSubmitter::new(
    LogSubmitterConfig::builder()
        .device_id("k3nQ7Rt9".into())
        .sender(sender)
        .pending_dir(app_data_dir.join("pending-logs"))
        .build(),
)?;

// `submit_layer` formats entries with the same `fmt::Layer`
// configuration as the file layer, so a shipped entry is
// byte-identical to the local log line.
init_tracing(
    &LoggingConfig { log_dir, file_name_prefix: "aegis-desktop.log".into() },
    vec![Box::new(submit_layer(submitter.handle()))],
)?;

submitter.shutdown()?;  // drains + joins; also runs in Drop
```

Batches flush at `batch_size` (200) or after `interval` (60 s) of
buffer inactivity. A failed batch is written to
`{pending_dir}/{batch_id}`; later cycles retry newest-first and
delete on success. `batch_id` is `{device_id}-{ULID}`, which is
what makes "newest first" a plain reverse sort of the directory.

Two behaviors worth knowing before debugging one:

- **The drain stops at the first failure.** A single batch the
  server will never accept blocks every older pending file and
  every future batch behind it. A batch the server reports as
  already ingested is the exception: `LogSender::is_already_ingested`
  lets the sender say so, and the file is deleted instead.
- **A full buffer drops the entry** and increments
  `LogSubmitter::dropped()`. The layer runs in a logging write
  path, where blocking the emitting thread is not an option.
````

- [ ] **Step 5: Run the whole verification gate**

```bash
cargo test  -p logging-utils
cargo clippy -p logging-utils --all-targets --all-features -- -D warnings
cargo test  -p aegis-desktop --lib
cargo test  -p aegis-server
cargo test --workspace
cargo check --workspace
cargo fmt --all -- --check
```
Expected: all pass, clippy clean, fmt clean.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/lib.rs lib/crates/logging-utils/README.md
git commit -m "$(cat <<'EOF'
feat(aegis-desktop): ship client logs to the server in batches

Builds the HTTP client and the submitter before init_tracing, so
the submit layer is installed in the same call as the file layer —
try_init only takes effect once per process, and the two must
share one formatter. The command shims and the submitter share a
single HttpClient, so both get the same 401 auto-refresh.

The batch id reuses the persisted per-install device prefix that
already produces every trace id's middle segment, via a new
device_prefix() accessor, so no second read of the prefix file.

Co-Authored-By: Claude Code <noreply@anthropic.com>
EOF
)"
```

---

## Self-Review

**1. Spec coverage.** Every spec section maps to a task: `LogSender` + `SubmitterError` → 2; `LogSubmitterConfig` → 3; `PendingStore` (persist / newest-first / remove / cap) → 4; batch identity, batching, flush ordering, error handling, shutdown → 5 and 6; capture + `init_tracing` signature → 7 and 8; desktop `HttpLogSender` + device id → 9 and 10 plus 1. Backpressure and amplification → `max_pending_files` in 3/4 and `dropped()` in 6. README → 10.

**2. Placeholder scan.** No TBD/TODO. Two spots deliberately show a correction inline rather than shipping broken code: the `serde_json::Error` → `std::io::Error::other` mapping in Task 4 Step 3, and the `client` `Clone` caveat in Task 10 Step 2.

**3. Type consistency.** `SubmitterError` variants used by name in Tasks 4, 5, 6, 9 all match Task 2. `WorkerCtx` fields set in Task 5's test helper match Task 5's definition and Task 6's construction. `PendingStore::clone_store` is added in Task 5 Step 4 because Task 5's test helper needs it — noted in that step. `SubmitHandle` is defined in Task 6 and first *used* by `SubmitWriter` in Task 7; `layer.rs` imports `super::submitter::SubmitHandle`, which is `pub` in a `pub(crate)`-reachable module, so this resolves.

**4. Review Focus.** All five lines are pinned to tests: #1 → `pending_drain_continues_past_already_ingested_batch` (Task 5), #2 → `interval_with_empty_buffer_sends_nothing` (Task 5), #3 → `load_round_trips_entries_with_embedded_newlines` (Task 4), #4 → `persist_into_blocked_path_returns_persist_error` (Task 4), #5 → `batch_id_sanitizes_device_id_for_filesystem_use` (Task 5).
