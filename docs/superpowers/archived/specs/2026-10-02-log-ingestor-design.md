# `logging-utils::log_ingestor` — Design

**Date:** 2026-10-02
**Scope:** Add a `log_ingestor` module to `lib/crates/logging-utils` that lets any caller in the workspace submit a batch of plain UTF-8 log lines identified by a `batch_id`, deduplicates repeated `batch_id`s within a TTL window via an `moka` cache, and writes accepted batches through a bounded `crossbeam_channel` to a single background writer thread. The writer uses `tracing_appender::rolling::daily` under a caller-supplied file-name prefix.

## Motivation

`aegis-desktop` accepts log submission from its frontend over Tauri commands. Today every submission hits a synchronous file write on the receiver's call path, which couples UI latency to disk I/O and lets a buggy / replaying client submit the same batch twice with no deduplication.

`logging-utils` already owns the `tracing_appender` dependency and the daily-rotation idiom (see [`2026-09-30-logging-utils-crate-design.md`](2026-09-30-logging-utils-crate-design.md)). Adding `log_ingestor` here puts the dedup cache, the bounded queue, and the daily-rotating writer in one place so `aegis-desktop` can submit batches asynchronously with replay protection without re-implementing the plumbing.

**Scope: this module is for `aegis-desktop` only.** `aegis-server` is intentionally excluded — the server's log pipeline is structured differently (per-request spans, OpenTelemetry-shaped events) and using the same ingestor there would force a square peg into a round hole. The module's API is generic enough that a future server-specific ingestor can live next to it without colliding, but no server integration is planned in this PR or its follow-ups.

This PR adds the module. It does NOT wire it into `aegis-desktop` either — the Tauri `submit` command stays on its current synchronous path until a follow-up PR replaces it with a shim over `LogIngestor::submit`. The module is built, tested, documented, and ready to be consumed.

## Approach

- New peer module `src/log_ingestor/` under `lib/crates/logging-utils`, exported via `pub use` from `lib.rs`. Same crate-root pattern the existing `tracing_init` and `trace_id` modules follow.
- `LogIngestorConfig` (with defaults) + builder-style setters.
- `LogIngestor` owns an `moka::sync::Cache<String, ()>` (dedup), a `crossbeam_channel::Sender<BatchEnvelope>` (queue), and an `Option<JoinHandle<()>>` for the writer thread.
- Sync `submit(batch_id, &entries)` returning `Result<(), IngestorError>`. No async constructor, no async API (per the `lib-crate-development` guideline: constructors stay obvious, no async ceremony).
- Background writer thread loops on `rx.recv()`, owns the `RollingFileAppender` directly (no `non_blocking` wrapper — the writer thread IS the buffer), writes one batch header line + one entry per line.
- Explicit `shutdown(&mut self)` with a configurable deadline, plus a `Drop` fallback that does a best-effort drain + join.
- Defaults: `cache_capacity = 10_000`, `cache_ttl = 600 s`, `channel_capacity = 1_000`, `shutdown_deadline = 5 s`.

## Architecture

```
lib/crates/logging-utils/src/
├── lib.rs                       # adds `pub mod log_ingestor;` + re-exports
├── tracing_init.rs              # unchanged
├── trace_id.rs                  # unchanged
├── log_ingestor.rs              # new — module surface + pub use
└── log_ingestor/                # new — submodules (no mod.rs; matches crate convention)
    ├── config.rs                # LogIngestorConfig + builder
    ├── ingestor.rs              # LogIngestor, IngestorError, BatchEnvelope
    └── writer.rs                # writer thread fn + write helpers
```

The DDD layered convention from `lib-crate-development.md` does NOT apply — this crate is observability infrastructure, not a business lib.

```
┌──────────────────────────┐
│  caller thread (HTTP,    │── submit(batch_id, &entries)
│  Tauri command, tests)   │   ↓
└──────────────┬───────────┘
               │  1. cache.get(batch_id) → reject if Some
               │  2. cache.insert(batch_id, ())
               │  3. tx.send(envelope) → Err(ChannelFull) evicts cache entry
               ▼
       ┌───────────────┐                bounded mpsc
       │ moka cache    │   ◀──────      crossbeam_channel
       │ TTL + LRU     │                capacity 1_000
       └───────────────┘                ┌──────────────────────┐
                                        │ writer thread        │
                                        │  loop {              │
                                        │    rx.recv() → write │
                                        │    if shutdown: break│
                                        │  }                   │
                                        │  done_tx.send(())    │── bounded(1) done signal
                                        │       │              │   ───────────────────▶ shutdown()
                                        │       ▼              │
                                        │ RollingFileAppender  │
                                        │ (daily, prefix, dir) │
                                        └──────────────────────┘
```

## Components

### 1. `src/log_ingestor/config.rs`

Knobs for `LogIngestor::new`. The caller resolves the directory (env var, `AppHandle::path().app_data_dir()`, test tempdir) and supplies the file-name prefix exactly like `LoggingConfig`.

```rust
use std::path::PathBuf;
use std::time::Duration;

/// Knobs for [`crate::LogIngestor`]. Constructed via
/// [`LogIngestorConfig::new`] (applies defaults) or
/// [`LogIngestorConfig::builder`] (override each knob independently).
#[derive(Debug, Clone)]
pub struct LogIngestorConfig {
    /// Directory the daily-rotated log file lives under. Created with
    /// `create_dir_all` if missing — failure returns
    /// [`IngestorError::CreateDir`].
    pub log_dir: PathBuf,
    /// Prefix handed to `tracing_appender::rolling::daily`. The lib
    /// produces `{file_name_prefix}.YYYY-MM-DD`.
    pub file_name_prefix: String,
    /// Max entries in the dedup cache. Once exceeded, the least
    /// recently used entry is evicted (moka TinyLFU policy). Default
    /// 10 000.
    pub cache_capacity: u64,
    /// TTL for dedup cache entries. Default 600 s.
    pub cache_ttl: Duration,
    /// Capacity of the bounded mpsc channel from producers to the
    /// writer thread. Default 1 000.
    pub channel_capacity: usize,
    /// Deadline for `shutdown` / `Drop` to join the writer thread.
    /// Default 5 s.
    pub shutdown_deadline: Duration,
}

impl LogIngestorConfig {
    pub fn new(log_dir: PathBuf, file_name_prefix: String) -> Self;
    pub fn builder() -> LogIngestorConfigBuilder;
}

pub struct LogIngestorConfigBuilder { /* … */ }
impl LogIngestorConfigBuilder {
    pub fn log_dir(self, dir: PathBuf) -> Self;
    pub fn file_name_prefix(self, prefix: String) -> Self;
    pub fn cache_capacity(self, capacity: u64) -> Self;
    pub fn cache_ttl(self, ttl: Duration) -> Self;
    pub fn channel_capacity(self, capacity: usize) -> Self;
    pub fn shutdown_deadline(self, deadline: Duration) -> Self;
    pub fn build(self) -> LogIngestorConfig;
}
```

Default values are applied in `LogIngestorConfig::new` and in each builder method's `or_default` step (single source of truth — a `const DEFAULTS: Self` static). Defaults:

| Knob | Default |
|---|---|
| `cache_capacity` | 10 000 |
| `cache_ttl` | 600 s |
| `channel_capacity` | 1 000 |
| `shutdown_deadline` | 5 s |

### 2. `src/log_ingestor/ingestor.rs`

```rust
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::thread::JoinHandle;
use crossbeam_channel::{Receiver, Sender};
use moka::sync::Cache;

pub struct LogIngestor {
    cache: Cache<String, ()>,
    tx: Option<Sender<BatchEnvelope>>,
    done_rx: Option<Receiver<()>>,
    writer: Option<JoinHandle<()>>,
    shutdown_flag: Arc<AtomicBool>,
    deadline: Duration,
}

pub(crate) struct BatchEnvelope {
    pub batch_id: String,
    pub entries: Vec<String>,
}

impl LogIngestor {
    /// Build a [`LogIngestor`] with the configured knobs. Creates the
    /// log directory (failure → [`IngestorError::CreateDir`]) and
    /// spawns the writer thread. The writer thread holds the
    /// `Receiver` end of the channel.
    pub fn new(config: LogIngestorConfig) -> Result<Self, IngestorError>;

    /// Submit a batch of UTF-8 log lines identified by `batch_id`.
    /// Rejects with [`IngestorError::DuplicateBatchId`] if the same
    /// `batch_id` was submitted within the cache TTL. Rejects with
    /// [`IngestorError::ChannelFull`] if the bounded channel is full
    /// (caller should retry). Rejects with [`IngestorError::ShutDown`]
    /// if the ingestor has been shut down.
    pub fn submit(
        &self,
        batch_id: impl Into<String>,
        entries: &[String],
    ) -> Result<(), IngestorError>;

    /// Drain the channel and join the writer thread within
    /// `config.shutdown_deadline`. Idempotent: a second call after
    /// the first is a no-op returning `Ok(())`.
    pub fn shutdown(&mut self) -> Result<(), IngestorError>;
}
```

`IngestorError`:

```rust
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
    WriterJoinTimeout { deadline: Duration, message: String },
    #[error("writer thread panicked: {0}")]
    WriterPanic(String),
}
```

### 3. `src/log_ingestor/writer.rs`

```rust
use crossbeam_channel::{Receiver, Sender};
use tracing_appender::rolling::RollingFileAppender;

pub(crate) fn run_writer(
    rx: Receiver<BatchEnvelope>,
    appender: RollingFileAppender,
    shutdown_flag: Arc<AtomicBool>,
    done_tx: Sender<()>,
) {
    let mut writer = appender; // owns the daily-rotating file handle
    loop {
        match rx.recv() {
            Ok(envelope) => write_envelope(&mut writer, &envelope),
            Err(_) if shutdown_flag.load(Acquire) => break, // sender dropped after shutdown
            Err(_) => break, // sender dropped unexpectedly — exit cleanly
        }
    }
    let _ = done_tx.send(());
}

fn write_envelope(
    writer: &mut RollingFileAppender,
    envelope: &BatchEnvelope,
) {
    use std::io::Write;
    // `batch_id` is intentionally NOT written — see "Data model &
    // wire shape" below.
    for line in &envelope.entries {
        let _ = writeln!(writer, "{line}");
    }
    let _ = writer.flush();
}
```

The writer does NOT use `tracing_appender::non_blocking`. It owns the `RollingFileAppender` directly, writes synchronously, and flushes after every envelope. The thread is the buffer — there's no second buffer to wait on.

### 4. `src/log_ingestor/mod.rs`

```rust
//! `log_ingestor` — batched, deduplicated, bounded-queue log writer.
//!
//! See the `LogIngestor` docs and the design doc
//! `docs/superpowers/specs/2026-10-02-log-ingestor-design.md` for the
//! full rationale.

mod config;
mod ingestor;
mod writer;

pub use config::{LogIngestorConfig, LogIngestorConfigBuilder};
pub use ingestor::{IngestorError, LogIngestor};
```

### 5. `src/lib.rs` (edit)

Add one line and one `pub use`:

```rust
pub mod log_ingestor;

pub use log_ingestor::{IngestorError, LogIngestor, LogIngestorConfig, LogIngestorConfigBuilder};
// existing:
pub mod trace_id;
pub mod tracing_init;
pub use trace_id::{Side, TraceIdGenerator};
pub use tracing_init::{LogGuard, LoggingConfig, LoggingInitError, build_filter, init_tracing};
```

## Data flow

### Submit path (one thread, sync)

```
submit(batch_id, &entries):
    1. if shutdown_flag.load(Acquire) → Err(ShutDown)
    2. let id = batch_id.into();
    3. if cache.get(&id).is_some() → Err(DuplicateBatchId(id))
    4. cache.insert(id.clone(), ());
    5. let envelope = BatchEnvelope { batch_id: id, entries: entries.to_vec() };
    6. match tx.send(envelope) {
         Ok(()) => Ok(()),
         Err(SendError(env)) => {
             // channel full: undo the cache insert so the same id
             // can be retried later
             cache.invalidate(&env.batch_id);
             Err(ChannelFull(channel_capacity))
         }
       }
```

The cache-invalidate-on-overflow is the one piece of state machinery that isn't free — it costs an extra `moka` operation under sustained backpressure. The trade-off is documented: a flood of `ChannelFull` writes shouldn't pollute the dedup cache for the next 10 minutes, so the producer can retry with the same id once the writer catches up.

### Shutdown path

`std::thread::JoinHandle::join` blocks indefinitely, so a deadline-bounded join uses a second `crossbeam_channel` as a "done" signal. The writer sends `()` on a `bounded(1)` channel when its loop exits; the `shutdown` path waits for that send with `recv_timeout(deadline)`, then calls `handle.join()` to retrieve any panic payload.

```
shutdown(&mut self):
    if self.writer.is_none(): return Ok(());   // already shut down
    shutdown_flag.store(true, Release);
    drop(self.tx.take());                       // closes the main channel
    let done_rx = self.done_rx.take().unwrap(); // bounded(1) done signal
    match done_rx.recv_timeout(self.deadline) {
        Ok(()) => match self.writer.take().unwrap().join() {
            Ok(()) => Ok(()),
            Err(payload) => Err(WriterPanic(format!("{payload:?}"))),
        },
        Err(RecvTimeout) => Err(WriterJoinTimeout {
            deadline: self.deadline,
            message: "writer thread did not exit in time".into(),
        }),
    }
```

`LogIngestor` holds the `Sender` end of the done channel inside the writer closure so it's dropped alongside the writer. The receiver lives in `LogIngestor` and is consumed by `shutdown`.

`Drop` calls `shutdown` and ignores the `Result` — best-effort drain + join. The deadline still applies. Anything in flight after the deadline is dropped; this is the documented trade-off.

### Writer thread lifecycle

```
run_writer spawned at LogIngestor::new
    │
    ├── rx.recv() → write_envelope  (blocks on empty channel)
    ├── rx.recv() Err + shutdown_flag = true → break
    └── rx.recv() Err + shutdown_flag = false → break (sender dropped early)
```

The thread is joined by `shutdown` (explicit) or `Drop` (fallback). The join is bounded by `shutdown_deadline`.

## Workspace wiring

### Root `Cargo.toml`

Add to `[workspace.dependencies]`:

```toml
# `moka` provides the TinyLFU-backed dedup cache used by
# `logging-utils::log_ingestor`. Pinned so every consumer resolves
# the same version.
moka              = { version = "0.12", default-features = false, features = ["sync"] }
# `crossbeam-channel` is the bounded mpsc that carries accepted
# batches from producers to the writer thread. Pinned so every
# consumer resolves the same version.
crossbeam-channel = "0.5"
```

### `lib/crates/logging-utils/Cargo.toml`

```toml
[dependencies]
tracing            = { workspace = true }
tracing-subscriber = { workspace = true }
tracing-appender   = { workspace = true }
thiserror          = { workspace = true }
ulid               = { workspace = true }
# `moka` provides the dedup cache that remembers `batch_id`s for
# `LogIngestor::submit`; `sync` is enough (no async API surface).
moka               = { workspace = true }
# `crossbeam-channel` is the bounded mpsc between `submit` callers
# and the writer thread; sync `send` / `recv` is enough.
crossbeam-channel  = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
```

## Public API at a glance

| Symbol | Kind | Notes |
|---|---|---|
| `LogIngestorConfig` | struct | `{ log_dir, file_name_prefix, cache_capacity, cache_ttl, channel_capacity, shutdown_deadline }` |
| `LogIngestorConfigBuilder` | struct | Override each knob from the default |
| `LogIngestor` | struct | Owns cache, sender, writer handle, shutdown flag |
| `LogIngestor::new` | fn | `(&LogIngestorConfig) -> Result<Self, IngestorError>` |
| `LogIngestor::submit` | fn | `(&self, batch_id, &entries) -> Result<(), IngestorError>` |
| `LogIngestor::shutdown` | fn | `(&mut self) -> Result<(), IngestorError>` |
| `IngestorError` | enum | `CreateDir`, `DuplicateBatchId`, `ChannelFull`, `ShutDown`, `WriterJoinTimeout`, `WriterPanic` |

## Testing

Tests live as `mod tests` inside each module file (`config.rs`, `ingestor.rs`, `writer.rs`). No `tests/` directory integration tests — the crate has no I/O dependencies beyond the local filesystem, and the `mod tests` pattern keeps each test colocated with the type it exercises. All file-writing tests use `tempfile::tempdir()` (already a dev-dep).

| File | Tests |
|---|---|
| `config.rs` | `defaults_apply_when_only_dir_and_prefix_set`, `builder_overrides_each_knob_independently`, `clone_preserves_fields` |
| `ingestor.rs` | `submit_writes_one_line_per_entry_under_configured_prefix`, `submit_two_batches_with_distinct_ids_both_appear_in_file`, `submit_rejects_duplicate_batch_id_within_ttl`, `submit_duplicate_after_ttl_expires_succeeds`, `submit_returns_channel_full_when_capacity_exceeded`, `submit_returns_shut_down_after_shutdown`, `cache_evicts_oldest_at_capacity`, `shutdown_drains_pending_envelopes_before_returning`, `shutdown_is_idempotent`, `drop_drains_best_effort`, `writer_thread_panic_propagates_via_shutdown` |
| `writer.rs` | `writer_creates_log_dir_when_missing`, `writer_writes_a_single_envelope_blocking_then_continues`, `writer_stops_when_sender_dropped_and_channel_closed` |

### Test patterns

**File-content assertion.** Submit a known batch, `shutdown` (or `drop`) the ingestor, walk the temp dir for `"{prefix}.YYYY-MM-DD"`, `read_to_string`, assert content. The writer flushes per envelope so there's no buffered drain to wait for.

**`ChannelFull` trigger.** Use a tiny channel capacity (e.g. 2). Fill it with full envelopes that block the writer thread (the writer is blocking I/O so it processes them quickly — but `submit` is fast and the writer flushes between envelopes). Loop until `Err(ChannelFull)` is observed; deterministic without sleeping.

**TTL expiry.** Use a tiny TTL (e.g. 50 ms) and a `std::thread::sleep(100)` between the two submits. Sync-only, no async machinery needed.

**Writer panic.** Wrap the writer thread's `write_envelope` in a test seam (e.g. an `Arc<AtomicBool>` "panic on next envelope" flag); assert `shutdown` returns `Err(WriterPanic(_))`.

**`Drop` fallback.** Verify by constructing an ingestor, submitting one envelope, and dropping without calling `shutdown`. Then read the file — the envelope must be present.

## Data model & wire shape

No DDL, no DTO, no request/response changes. The crate writes to the local filesystem via `tracing_appender::rolling::daily`. On-disk format:

```
<line 1>
<line 2>
…
<line 1>
…
```

`batch_id` is intentionally **not** written. The dedup cache in `LogIngestor::submit` guarantees at-most-once delivery per id, so re-deriving identity from the file is not needed downstream. Plain UTF-8, LF line endings, one flush per batch. No JSON rendering — the caller decides the line format upstream.

## Out of scope (explicitly NOT in this PR)

- **Wiring `LogIngestor` into `aegis-desktop`.** Deferred — the desktop Tauri `submit` command keeps its existing synchronous write path until a follow-up PR replaces it with a shim over `LogIngestor::submit`.
- **Any `aegis-server` integration.** The server is explicitly excluded from this module's consumer set; a server-shaped ingestor can live next to it later if needed.
- **Async API surface.** Sync only; no `tokio::mpsc`, no `async fn submit`.
- **Per-batch file routing.** One daily-rotated file per `LogIngestor`.
- **Structured / JSON rendering of entries.** Plain UTF-8 lines, one entry per line.
- **Compression, archival, log shipping.** Plain files via `RollingFileAppender`.
- **A pre-built `LogIngestor` instance for tests.** Tests construct their own with `tempfile::tempdir()` and small knobs.

## Verification gate

```bash
cargo fmt --all -- --check
cargo clippy -p logging-utils --all-targets --all-features -- -D warnings
cargo test  -p logging-utils
cargo doc   -p logging-utils --no-deps
cargo check --workspace       # confirms the new member compiles alongside the rest
```

## File changes summary

- New: `lib/crates/logging-utils/src/log_ingestor/mod.rs`
- New: `lib/crates/logging-utils/src/log_ingestor/config.rs`
- New: `lib/crates/logging-utils/src/log_ingestor/ingestor.rs`
- New: `lib/crates/logging-utils/src/log_ingestor/writer.rs`
- Modified: `Cargo.toml` (workspace root) — add `moka` + `crossbeam-channel` to `[workspace.dependencies]`
- Modified: `lib/crates/logging-utils/Cargo.toml` — add `moka` + `crossbeam-channel` deps
- Modified: `lib/crates/logging-utils/src/lib.rs` — declare module + re-export
- Modified: `lib/crates/logging-utils/README.md` — document the new module
- **Unchanged:** `lib/crates/logging-utils/src/tracing_init.rs`, `trace_id.rs`
- **Unchanged:** `apps/server/aegis-server/`, `apps/desktop/aegis-desktop/`
