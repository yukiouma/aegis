# `logging-utils::log_submitter` — Client-Side Batch Submitter

**Date:** 2026-10-04
**Follows from:** [`2026-10-02-log-ingestor-server-sink-design.md`](2026-10-02-log-ingestor-server-sink-design.md)
**Owner:** aegis-desktop

## Motivation

The previous spec shipped the server side of client-log ingestion:
`POST /api/log-ingest/submit` routes `(batchId, entries)` through
`logging_utils::LogIngestor` into a server-side daily-rotated file.
It explicitly deferred the client:

> **Out of scope** — `aegis-desktop` client-side wiring. The desktop
> will call this endpoint from a separate work thread in a follow-up
> PR, not via a Tauri command.

This spec closes that gap. A new `log_submitter` module in
`logging-utils` owns a bounded buffer, batches on size or on an
idle interval, submits over a `LogSender` trait, and persists failed
batches to a pending directory for retry. `aegis-desktop` wires it in
with a `tracing` layer and an HTTP sender that reuses the existing
`HttpClient` (and therefore inherits its 401 auto-refresh).

`log_submitter` is the **producer**; `log_ingestor` is the
**consumer**. They are independent modules — no shared types beyond
the wire shape.

## Scope

**In:**

- `logging_utils::log_submitter` — config, sender trait, pending-dir
  persistence, batching worker, submit layer
- `init_tracing` gains a `layers` parameter
- `aegis-desktop`: `HttpLogSender`, `SubmitLayer` installation, device
  id plumbing

**Out:**

- Any change to `log_ingestor` or the server endpoint
- Compression, archival, log shipping
- Server-side retry behaviour
- A Tauri command to trigger a manual flush
- Auth beyond the existing bearer token

## Module layout

New sibling to `log_ingestor`. Nothing under `log_ingestor/` is
touched.

```text
lib/crates/logging-utils/src/
  log_submitter.rs              module hub
  log_submitter/config.rs       LogSubmitterConfig + builder
  log_submitter/sender.rs       LogSender trait + SubmitterError
  log_submitter/pending.rs      persist / list newest-first / remove / cap
  log_submitter/submitter.rs    LogSubmitter — thread, worker loop, flush
  log_submitter/layer.rs        SubmitLayer (fmt::Layer + MakeWriter)
  tracing_init.rs               EDIT — init_tracing gains a `layers` param
```

## `LogSender`

The transport seam. `aegis-desktop` supplies the only implementation
in this repo; tests supply in-memory fakes.

```rust
#[async_trait]
pub trait LogSender: Send + Sync {
    async fn send(
        &self,
        batch_id: &str,
        log_entries: Vec<String>,
    ) -> Result<(), SubmitterError>;
}
```

The error type is `SubmitterError` rather than a bare `()`, matching
the one-error-type-per-boundary convention this repo already follows
for `IngestorError` / `ApiError`. The desktop's impl maps `ApiError`
into `SubmitterError::Send` and preserves it as `#[source]`.

## Configuration

```rust
pub struct LogSubmitterConfig {
    pub device_id: String,
    pub sender: Arc<dyn LogSender>,
    pub batch_size: usize,          // default 200
    pub interval: Duration,         // default 60 s
    pub buffer_capacity: usize,     // default 1_000
    pub pending_dir: PathBuf,
    pub max_pending_files: usize,   // default 10
    pub shutdown_deadline: Duration,// default 5 s
}
```

Follows the `LogIngestorConfig` + builder shape so the two modules
read the same way.

`buffer_capacity` bounds the in-memory channel between the layer and
the worker. `max_pending_files` bounds the on-disk backlog — see
[Backpressure and amplification](#backpressure-and-amplification).

## Batch identity

```text
{batch_id} = {device_id}-{ULID}
```

```text
WjK3nQ7Rt9-A01HQ0Z8K4M2X7B9C4D6E8F0G2H
└─ device_id ─┘ └────── ULID ──────┘
              48-bit ms timestamp + 80-bit monotonic randomness
```

The `ulid` dependency is already present in this crate (it builds
trace ids). A ULID *is* a timestamp value, so this satisfies
"device id + current timestamp" while adding two properties the
system needs:

1. **Uniqueness within a process.** The ULID's monotonic component
   guarantees it. This is not cosmetic: the server answers a repeated
   `batchId` with `409 duplicate_batch_id`, and under
   [stop-on-failure](#flush-ordering) a duplicate would re-persist
   unchanged and wedge the pending queue on every future cycle.
2. **Free newest-first ordering.** The batch id is the pending
   filename, so a reverse lexicographic sort of the directory is
   exactly newest-first. No mtime reads, no clock skew.

## Data flow

### Capture

`init_tracing` stacks the submit layer above the existing fmt layer,
so every event passes through the **same** `fmt::Layer` configuration
twice — once to the rolling file, once through `SubmitWriter`. The
shipped JSON is therefore byte-identical to the local log file and
cannot drift from it.

`SubmitWriter: MakeWriter` mints one `EntryWriter` per event. The
writer buffers that event's bytes and, on `Drop`, trims the trailing
newline and `try_send`s the entry as a `String`.

**The writer always returns `Ok(n)` from `write`.** A full channel must
not surface as an `io::Error` — that would make `fmt::Layer` error
out on a log line. A dropped send increments an `AtomicU64` readable
via `LogSubmitter::dropped()`.

### Batching

A dedicated OS thread runs a current-thread tokio runtime and blocks
on the crossbeam receiver:

- `recv_timeout(interval)` returns an entry → push to the in-memory
  batch; flush when `len == batch_size`
- `recv_timeout` times out → flush whatever is buffered
- channel disconnected → flush once, then exit

The blocking `recv_timeout` is safe on this thread: the runtime's
only task is the worker loop, and the block is only reached between
`send().await` calls, so an in-flight send never stalls the receiver.

**Interpretation to confirm:** the timer resets on every *received
entry*, so `interval` measures inactivity, not time since the last
flush. Under steady low-rate traffic that never reaches `batch_size`,
the batch holds until traffic pauses. The alternative — a fixed
cadence measured from the last flush — would bound that, at the cost
of a slightly more complex loop. This spec takes the inactivity
reading because it matches "interval … after the last submission" and
is the natural shape of `recv_timeout`.

### Flush ordering

On flush, with a freshly minted `batch_id`:

1. Walk the pending directory in **reverse lexicographic order**. For
   each file: read its entries, `sender.send(file_batch_id, entries)`.
   On success, `remove_file` and continue to the next-newest.
2. On the **first** failure, stop the entire flush. Older pending
   files are untouched, and the fresh in-memory batch is neither sent
   nor persisted — it stays in memory for the next cycle.
3. If the drain completed, submit the fresh batch. On failure, write
   it to `{pending_dir}/{batch_id}` and drop it from memory; disk is
   now the copy.

Step 2 means a single permanently-rejected batch (e.g. one that the
server refuses with `400 validation_failed` on every attempt) blocks
the whole queue behind it. This is the accepted trade for strict
newest-first ordering; a give-up counter is the natural follow-up if
it proves too sharp in practice.

## Backpressure and amplification

The submit layer ships **every** event, with no target-based filter.
That includes the submitter's own lines — two per `HttpClient::send`
from its `#[tracing::instrument]`, plus any failure diagnostic. They
ride back in the *next* batch.

The growth is linear (each batch carries the previous batch's lines,
plus a small constant), not exponential, and a healthy server absorbs
it. The failure mode is the outage case: every failed cycle persists
a few more entries and emits a few more log lines, so the pending
directory grows without bound while the server is down.

`max_pending_files` (default 10) bounds this. When a persist would
exceed the cap, the **oldest** file is evicted first. This keeps the
"ship everything" property intact while making the outage degradation
predictable and visible via `dropped()`.

During an outage the in-memory buffer also fills, since stop-on-failure
holds the fresh batch. `try_send` starts dropping at that point, and
`dropped()` reports it.

## `LogSubmitter` public surface

```rust
impl LogSubmitter {
    pub fn new(config: LogSubmitterConfig) -> Result<Self, SubmitterError>;
    pub fn submit(&self, entry: String) -> Result<(), SubmitterError>;
    pub fn dropped(&self) -> u64;
    pub fn shutdown(&self) -> Result<(), SubmitterError>;
}
```

`new` spawns the worker thread, creates the pending dir, and returns.
`submit` is the producer side — a `try_send` into the bounded channel,
returning `ChannelClosed` if the worker is gone and discarding on
`Full` (the count lands in `dropped()`).

`handle()` returns a cheap `Arc`-backed clone of the submit side for
the `SubmitLayer` to hold, so the layer does not keep the whole
submitter — its flush thread and shutdown bookkeeping — alive. In
practice the desktop manages the submitter for the process anyway; the
indirection exists so the layer can be constructed before the
submitter is moved into managed state.

## Error handling

`SubmitterError` is this module's single boundary error, matching
`IngestorError`.

| Variant | Raised by | Meaning |
|---|---|---|
| `Send { batch_id, source }` | desktop sender impl | `HttpClient` returned `ApiError` — network, 4xx/5xx, or refresh failure |
| `CreateDir { dir, source }` | `LogSubmitter::new` | pending dir could not be created |
| `Persist { batch_id, source }` | flush | failed batch could not be written to the pending dir |
| `ChannelClosed` | `submit` | the worker thread is gone |
| `WorkerPanic` | `shutdown` | the worker thread panicked |
| `WorkerJoinTimeout` | `shutdown` | the join exceeded `shutdown_deadline` |

Each variant wraps the inner error with `#[source]`, so
`Error::source()` keeps the chain.

If persisting a failed batch *itself* fails (disk full, permissions),
the entries are dropped rather than retried in memory forever — they
are unrecoverable — and the `Persist` error is logged.

## Shutdown

`shutdown()` sets a stop flag, disconnects the crossbeam channel, and
joins the worker thread under `shutdown_deadline`. In-flight
`send().await` calls are allowed to finish; the join is what bounds
them.

`shutdown()` is idempotent and also runs from `Drop` — the same
contract `LogIngestor` already uses.

## `init_tracing` signature change

```rust
pub fn init_tracing(
    config: &LoggingConfig,
    layers: Vec<Box<dyn Layer<Registry> + Send + Sync>>,
) -> Result<LogGuard, LoggingInitError>
```

The subscriber type is pinned to `Registry`; `init_tracing` composes
`registry().with(filter).with(fmt_layer).with(layers)`. `try_init`
idempotency is unchanged — a second call is still a no-op.

Call sites:

- `aegis-server` `run.rs` → `init_tracing(&config, Vec::new())`
- `aegis-desktop` `lib.rs` → `init_tracing(&config, vec![Box::new(submit_layer)])`

## Desktop wiring

`lib.rs::setup` reorders so the submit layer is live from the first
log line:

1. Open the `auth.bin` store → `TauriStore` → `HttpClient` — **moved
   above** `init_tracing`
2. `HttpLogSender::new(Arc::new(client))`
3. `LogSubmitter::new(LogSubmitterConfig { device_id, sender,
   pending_dir: app_data_dir.join("pending-logs"), .. })`
4. `init_tracing(&cfg, vec![Box::new(SubmitLayer::new(submitter.handle()))])`
5. `app.manage(submitter)` — keeps it alive for the process

The only logs lost are those emitted before `setup` runs, which is
already true of the current code.

### `http/log_ingest.rs` — `HttpLogSender`

Wraps `Arc<HttpClient>` and calls:

```rust
client.request::<LogIngestSubmitRequest, LogIngestSubmitResponse>(
    reqwest::Method::POST,
    "/api/log-ingest/submit",
    Some(&LogIngestSubmitRequest { batch_id, entries }),
).await
```

**No second reqwest client.** Reusing `HttpClient` is what inherits
the 401 auto-refresh-and-retry path; a fresh client would silently
lose refreshed sessions and log submissions once a token expired.

The body is `#[serde(rename_all = "camelCase")]` — `batch_id` becomes
`batchId` on the wire, matching the server DTO. The response is
discarded beyond the `Result`.

### Device id

`TraceIdGenerator` keeps `device_prefix` private with no accessor, so
the desktop cannot read it back. Add:

```rust
impl TraceIdGenerator {
    pub fn device_prefix(&self) -> Option<&str> { … }
}
```

and pass `generator.device_prefix().unwrap_or("desktop")` into the
config. One source of truth — no second read of the
`aegis-desktop-device-prefix` file, and the batch id's first segment
matches the middle segment of every trace id from the same install.

## Testing

`log_submitter` uses in-memory fakes — a `RecordingSender` capturing
`(batch_id, entries)` and a `FailingSender`. No wiremock is needed;
this is pure unit territory.

**`config.rs`**

- defaults applied when only the required fields are set
- builder overrides each knob independently
- `clone` preserves fields

**`pending.rs`**

- persist writes `{pending_dir}/{batch_id}`
- listing is reverse lexicographic (newest first) for a directory of
  known batch ids
- `remove` deletes the file and is a no-op when absent
- exceeding `max_pending_files` evicts the oldest
- persist into a read-only / blocked path yields `Persist`

**`submitter.rs`**

- `batch_size` entries trigger a flush
- `interval` elapsing under `batch_size` triggers a flush
- a failed send writes a file named `{batch_id}` into the pending dir
- the next cycle retries the newest pending file first and removes it
  on success
- **stop-on-failure:** with two pending files where the newest fails,
  the older file is untouched and the fresh in-memory batch is neither
  sent nor persisted
- a full buffer increments `dropped()` and does not panic the layer
- `shutdown` joins the worker and is idempotent

**`layer.rs`**

- a captured entry parses as JSON and carries the event's fields
- two events produce two entries (not one coalesced blob)
- a full buffer still reports the event as written to the file layer

**Desktop `http/log_ingest.rs`** (wiremock, matching the existing
`http/client.rs` test style)

- posts to `/api/log-ingest/submit` with the camelCase body
- carries the Bearer header (inherits the client behaviour)
- maps a non-2xx response to `SubmitterError::Send`

**`trace_id.rs`**

- `device_prefix()` returns the supplied prefix, and `None` when
  constructed without one

## Verification gate

```bash
cargo test  -p logging-utils
cargo clippy -p logging-utils --all-targets --all-features -- -D warnings
cargo test  -p aegis-desktop --lib
cargo test  -p aegis-server
cargo test --workspace
cargo check --workspace
cargo fmt --all -- --check
```

## Acceptance criteria

1. `LogSubmitter` batches into a bounded buffer and flushes on
   `batch_size` or on `interval` of inactivity, whichever comes first.
2. Each batch carries a `{device_id}-{ULID}` id, unique per process.
3. A failed submission is persisted to `{pending_dir}/{batch_id}`.
4. Later cycles retry pending files newest-first, delete on success,
   and stop the whole flush on the first failure.
5. `max_pending_files` bounds the on-disk backlog, evicting oldest.
6. `init_tracing` accepts extra layers; `aegis-server` passes an empty
   vec and its behaviour is unchanged.
7. `aegis-desktop` installs the submit layer, ships the same JSON
   format as the local file, and posts through the existing
   `HttpClient`.
8. All tests, clippy, fmt, and the workspace check pass.
