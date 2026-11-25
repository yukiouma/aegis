# aegis-server X-Trace-ID Extraction — Design

**Date:** 2026-09-29
**Scope:** Extract the inbound `X-Trace-ID` request header on every `aegis-server` HTTP request and attach the value as a `trace_id` field on the per-request `tracing` span. The desktop already mints and sends this header from `HttpClient::send`; this change is the server-side counterpart.

## Motivation

The `aegis-server` already wraps every request in a `tower_http::trace::TraceLayer` span and writes JSON log lines to `$AEGIS_LOG_DIR/aegis-server.log.YYYY-MM-DD`. The `trace-id` crate exists in the workspace but the server does not depend on it yet. As a result, server log lines have no correlation id matching the desktop log lines that minted the request.

The desktop already emits `X-Trace-ID: C-<prefix>-<ulid>` on every outbound command → request (see `docs/superpowers/specs/2026-09-29-aegis-desktop-logging-design.md`). The server needs to read that header and surface it on its request span so the two log streams join on a single id.

## Approach

- Add `trace-id` as a dependency of `aegis-server`.
- Build a small `TraceIdMakeSpan` implementing `tower_http::trace::MakeSpan<axum::body::Body>`. It extracts `X-Trace-ID` from the request headers, validates it lightly, and falls back to `TraceIdGenerator::server_side()` when the header is missing or malformed.
- Plug the custom span maker into the existing `TraceLayer` via `.make_span_with(...)`. Every request now emits a single `http_request` span carrying `trace_id`, `method`, and `path` fields.

## Architecture

```
src-tauri/src/http/client.rs   ────  X-Trace-ID: C-desktop-01H... ────▶ HTTP
                                                                │
                                                                ▼
                                       tower_http::trace::TraceLayer::new_for_http()
                                          .make_span_with(TraceIdMakeSpan::new(generator))
                                                                │
                                                                ▼
                                              tracing::info_span!(
                                                  "http_request",
                                                  trace_id = %id,
                                                  method = %method,
                                                  path   = %path,
                                              )
                                                                │
                                                                ▼
                          every event under that span carries trace_id
                       (sqlx queries, handler tracing::info! / error!, …)
```

## Components

### 1. New: `apps/server/aegis-server/src/transport/http/trace_id.rs`

Owns the header lookup, the validation rule, and the `MakeSpan` impl. The header/validation pair is a top-level free helper so it is unit-testable without an axum request type.

```rust
use axum::body::Body;
use axum::http::{HeaderMap, Request};
use tower_http::trace::MakeSpan;
use tracing::{info_span, Span};
use trace_id::TraceIdGenerator;

/// Header name. HTTP headers are case-insensitive but `HeaderMap`
/// stores them in lowercase; using the lowercase form is the
/// project-wide convention for `HeaderName` literals.
pub(crate) const X_TRACE_ID_HEADER: &str = "x-trace-id";

/// Maximum accepted length of an inbound `X-Trace-ID` value. Anything
/// longer is treated as malformed — defensive against header-smuggling
/// attempts and keeps the JSON log field bounded.
pub(crate) const MAX_TRACE_ID_LEN: usize = 128;

/// Span name produced for every request. Single span wraps the whole
/// exchange (tower-http's default produces a request/response pair —
/// we replace it with one span).
const SPAN_NAME: &str = "http_request";

pub(crate) struct TraceIdMakeSpan {
    generator: TraceIdGenerator,
}

impl TraceIdMakeSpan {
    pub(crate) fn new(generator: TraceIdGenerator) -> Self {
        Self { generator }
    }
}

impl MakeSpan<Body> for TraceIdMakeSpan {
    fn make_span(&mut self, request: &Request<Body>) -> Span {
        let trace_id = extract_trace_id(request.headers(), &self.generator);
        info_span!(
            SPAN_NAME,
            trace_id = %trace_id,
            method   = %request.method(),
            path     = %request.uri().path(),
        )
    }
}

/// Extract `X-Trace-ID` from the request headers, falling back to a
/// freshly-minted server-side id when the header is missing,
/// malformed UTF-8, empty, too long, or contains non-graphic bytes.
pub(crate) fn extract_trace_id(headers: &HeaderMap, generator: &TraceIdGenerator) -> String {
    headers
        .get(X_TRACE_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .filter(|s| is_valid_trace_id(s))
        .map(str::to_owned)
        .unwrap_or_else(|| generator.server_side())
}

/// A trace id is valid when it is 1..=MAX_TRACE_ID_LEN ASCII graphic
/// characters. Permissive enough to accept the desktop's
/// `C-<prefix>-<ulid>` ids and any reasonable client variant; strict
/// enough to keep the JSON log field bounded and printable.
pub(crate) fn is_valid_trace_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= MAX_TRACE_ID_LEN
        && s.chars().all(|c| c.is_ascii_graphic())
}
```

### 2. Modified: `apps/server/aegis-server/src/transport/http.rs`

One line added: `pub mod trace_id;`. No re-export — `TraceIdMakeSpan` is an internal layer, never reaches the public `transport` surface.

### 3. Modified: `apps/server/aegis-server/src/transport/http/router.rs`

Replace
```rust
.layer(tower_http::trace::TraceLayer::new_for_http())
```
with
```rust
.layer(
    tower_http::trace::TraceLayer::new_for_http()
        .make_span_with(TraceIdMakeSpan::new(TraceIdGenerator::new(None))),
)
```

The generator is constructed inside `router()` so it lives as long as the `TraceLayer`. `device_prefix = None` is intentional — the server is not a specific device, and any prefix would just be redundant host metadata.

### 5. Modified: `apps/server/aegis-server/Cargo.toml`

Add one dependency (alphabetical, before `tracing`):
```toml
# `trace-id` is the workspace crate that mints `C-/S-…` ids. We
# use `server_side()` as the fallback when the inbound request has
# no `X-Trace-ID` (or sends one we can't trust).
trace-id = { path = "../../../../lib/crates/trace-id" }
```

## Data flow

1. Inbound HTTP request hits the top-level `Router`.
2. `TraceLayer` enters its service stack; `TraceIdMakeSpan::make_span(&request)` runs.
3. `extract_trace_id(request.headers(), &self.generator)` returns either the inbound `X-Trace-ID` (when present and valid) or a fresh `S-<ulid>`.
4. `tracing::info_span!("http_request", trace_id = %id, method = %method, path = %path)` is returned and entered by `TraceLayer`.
5. The handler runs inside that span; every `tracing::event!` / `info!` / `error!` inside inherits `trace_id` automatically (no manual `record()` needed).
6. `TraceLayer` exits the span when the response is sent; the JSON log line emitted at that point carries the same fields.

No wire-shape change: no request header is required, no response header is added, no DTO is touched.

## Key decisions locked in

- **Header name**: `X-Trace-ID`. Matches the desktop-side outbound naming and the project convention.
- **Fallback**: server-side `TraceIdGenerator::server_side()` when header is missing or malformed. No 400, no skipped field — every request span has a `trace_id`.
- **No response header echo.** Server is observability-only.
- **Span name**: `http_request` (single span wraps the whole exchange).
- **Validation**: `1..=128` ASCII-graphic characters. Permissive enough to accept desktop ids (`C-desktop-01H…`) and any reasonable client variant; strict enough to keep the JSON log field bounded and printable.
- **`device_prefix = None`** on the server-side generator. The server is not a specific device; the prefix exists so the desktop can keep ids stable across launches.

## Testing strategy

### Unit tests in `apps/server/aegis-server/src/transport/http/trace_id.rs` (`#[cfg(test)] mod tests`)

Pure unit tests, no axum runtime needed:
- `extract_trace_id_uses_header_when_present_and_valid` — `X-Trace-ID: C-desktop-01H9XQ8Z6VK3FJ4P5N2W7Y0T8CB` round-trips verbatim.
- `extract_trace_id_generates_when_header_missing` — no header → returns a string starting with `S-`.
- `extract_trace_id_generates_when_header_empty` — empty value → falls back to a server-side id (starts with `S-`).
- `extract_trace_id_generates_when_header_too_long` — value of `MAX_TRACE_ID_LEN + 1` chars → falls back.
- `extract_trace_id_generates_when_header_non_graphic` — value containing `\n` → falls back.
- `is_valid_trace_id_accepts_desktop_id_format` — sanity: `C-desktop-01H9XQ8Z6VK3FJ4P5N2W7Y0T8CB` is accepted.
- `make_span_includes_required_field_names` — calls `make_span(&request)` on a synthetic request, asserts the returned `Span`'s metadata contains `trace_id`, `method`, and `path` as field names. (Field *values* are not introspectable via `Span::metadata` alone; the value-source unit tests above cover the correctness invariant.)

### Integration test in `apps/server/aegis-server/src/transport/http/router.rs` (existing `tests` module)

- `request_with_trace_id_header_succeeds` — drives `GET /healthz` with `X-Trace-ID: C-desktop-01ABCDEF01234567890ABCDEF` and asserts `200 OK`. Proves the new layer does not break the existing path; value-level assertion is already covered by the unit tests.

## Trade-offs / things we accept

- **`make_span` runs before route matching**, so `path` is the raw URI path, not the matched route pattern. That's the right tradeoff for log volume — `path` is more informative than the route ID when scanning JSON logs.
- **The span replaces tower-http's default request/response pair** because `make_span_with` returns one span. tower-http's default emits `started` / `response received` / `failed` events at debug/info; those are gone. If we want back the lifecycle events we can chain `.on_request(...)` / `.on_response(...)` separately — out of scope for this change.
- **`HeaderMap` lookup is case-insensitive in practice** (`HeaderName::eq` ignores case). We use the lowercase form `"x-trace-id"` because that is the project convention for `HeaderName` literals; the desktop side writes the same lowercase form on outbound requests.

## Verification gate, before any PR

```bash
cargo fmt --all -- --check
cargo clippy -p aegis-server --all-targets --all-features -- -D warnings
cargo test  -p aegis-server
cargo check --workspace
```

## File changes summary

- New: `apps/server/aegis-server/src/transport/http/trace_id.rs` (~70 lines including tests)
- Modified: `apps/server/aegis-server/src/transport/http.rs` (one `pub mod` line)
- Modified: `apps/server/aegis-server/src/transport/http/router.rs` (one-line layer edit + one `use` line + one new integration test)
- Modified: `apps/server/aegis-server/Cargo.toml` (one `trace-id` dep line)