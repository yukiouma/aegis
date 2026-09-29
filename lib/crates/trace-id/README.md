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

let generator = TraceIdGenerator::new(Some("desktop".to_string()));
let client_id = generator.client_side();   // e.g. "C-desktop-01H9XQ8Z6VK3FJ4P5N2W7Y0T8CB"
let server_id = generator.server_side();   // e.g. "S-desktop-01H9XQ9A2DF4GJ7M5P1R3V6X9BC"
```

Construct a generator once per process or per request-handling task and
reuse it — the generator is stateless.

## Verification

```bash
cargo test  -p trace-id
cargo clippy -p trace-id --all-targets --all-features -- -D warnings
cargo fmt   --all -- --check
```