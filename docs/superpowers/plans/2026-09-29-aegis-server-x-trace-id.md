# aegis-server X-Trace-ID Extraction Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extract the inbound `X-Trace-ID` request header on every `aegis-server` HTTP request and attach the value as a `trace_id` field on the per-request `tracing` span, with a server-side id fallback.

**Architecture:** A small new file `apps/server/aegis-server/src/transport/http/trace_id.rs` owns a `TraceIdMakeSpan` that implements `tower_http::trace::MakeSpan<axum::body::Body>`. It extracts `X-Trace-ID` from the request headers, validates it lightly, and falls back to `TraceIdGenerator::server_side()` when the header is missing or malformed. The existing `tower_http::trace::TraceLayer` in `router.rs` is updated to use this span maker via `.make_span_with(...)`. No wire-shape change, no response-header echo.

**Tech Stack:** Rust 2024 edition; `axum = 0.8`; `tower-http = { workspace = true, features = ["trace"] }`; `trace-id` workspace crate; `tracing` + `tracing-subscriber` (workspace); `http` (via `axum::http`).

## Global Constraints

- `aegis-server` `Cargo.toml` already lists `tower-http` with the `trace` feature; `tower_http::trace::MakeSpan` is in scope.
- The `trace-id` workspace crate exposes `TraceIdGenerator::new(Option<String>) -> Self`, `client_side() -> String`, `server_side() -> String`. `server_side()` returns `"S-<ulid>"` (no device prefix) when constructed with `None`.
- Header lookup uses the lowercase form (`"x-trace-id"`) per the project convention for `HeaderName` literals; `HeaderMap` matches case-insensitively.
- Validation: `1..=128` chars, every char `is_ascii_graphic()`.
- Span name: `http_request`. Span fields: `trace_id`, `method`, `path`.
- No response-header echo. No DTO changes. No new public types outside `transport/http::trace_id` (which stays `pub(crate)`).
- The desktop side already sends `X-Trace-ID: C-<prefix>-<ulid>` on every outbound request; no change there.
- The plan keeps every `aegis-server` existing test green. The existing router tests do not send `X-Trace-ID`, so they exercise the server-side fallback path.

---

## File Structure

- **Create**: `apps/server/aegis-server/src/transport/http/trace_id.rs` — owns `TraceIdMakeSpan`, `extract_trace_id`, `is_valid_trace_id`, constants, and tests. Single responsibility: turn an inbound `Request` into a `Span` carrying `trace_id`.
- **Modify**: `apps/server/aegis-server/src/transport/http.rs` — one-line `pub mod trace_id;` so `router.rs` can reach the new module.
- **Modify**: `apps/server/aegis-server/src/transport/http/router.rs` — replace the default `TraceLayer` with one that uses `TraceIdMakeSpan`; add one new integration test.
- **Modify**: `apps/server/aegis-server/Cargo.toml` — add the `trace-id` workspace dep.

`trace_id.rs` is self-contained and easy to test in isolation. `router.rs` only sees the new module through one import + one layer line; the new integration test follows the existing pattern in `router::tests`.

---

## Task 1: Add `trace-id` workspace dependency

**Files:**
- Modify: `apps/server/aegis-server/Cargo.toml`

**Interfaces:**
- Consumes: nothing.
- Produces: `trace-id` available as `{ path = "..." }` in `aegis-server`'s dependency list; `cargo check -p aegis-server` resolves.

- [ ] **Step 1: Open `apps/server/aegis-server/Cargo.toml`**

Locate the `[dependencies]` block. It currently lists deps in alphabetical order; `tower-http` and `tracing` are consecutive near the bottom.

- [ ] **Step 2: Insert the `trace-id` dep**

Add this block immediately after the `tower-http` entry and before the `tokio` entry (keeps the existing alphabetical order — `tower-http` < `trace-id` < `tracing`):

```toml
# `trace-id` is the workspace crate that mints `C-/S-…` ids. We
# use `server_side()` as the fallback when the inbound request has
# no `X-Trace-ID` (or sends one we can't trust).
trace-id = { path = "../../../lib/crates/trace-id" }
```

The resulting dependency block reads (new line bolded in review only):

```toml
tower-http = { workspace = true, features = ["trace"] }
trace-id = { path = "../../../lib/crates/trace-id" }
tokio = { workspace = true, features = ["macros", "rt-multi-thread", "signal"] }
```

- [ ] **Step 3: Verify the workspace resolves**

Run: `cargo check -p aegis-server`
Expected: PASS — `trace-id` is recognised as a path dep, no errors about missing crate.

- [ ] **Step 4: Commit**

```bash
git add apps/server/aegis-server/Cargo.toml
git commit -m "deps(server): add trace-id workspace dependency"
```

---

## Task 2: Add the `trace_id` module skeleton with constants and `is_valid_trace_id`

**Files:**
- Create: `apps/server/aegis-server/src/transport/http/trace_id.rs`

**Interfaces:**
- Consumes: nothing (standalone helper).
- Produces:
  - `pub(crate) const X_TRACE_ID_HEADER: &str = "x-trace-id";`
  - `pub(crate) const MAX_TRACE_ID_LEN: usize = 128;`
  - `pub(crate) fn is_valid_trace_id(s: &str) -> bool`

- [ ] **Step 1: Create the file with the failing tests + module skeleton (no implementation yet)**

Write this into `apps/server/aegis-server/src/transport/http/trace_id.rs`. The module is registered in Step 2 and the `is_valid_trace_id` body is the `unimplemented!()` stub so the tests fail at this stage:

```rust
//! Extracts the `X-Trace-ID` request header into a `tracing::Span`.
//!
//! The custom [`MakeSpan`](tower_http::trace::MakeSpan) impl below
//! plugs into `tower_http::trace::TraceLayer` so every request
//! span carries a `trace_id` field. When the inbound header is
//! missing or malformed, the span falls back to a freshly-minted
//! server-side id so observability is unconditional.
//!
//! Validation is intentionally permissive: 1..=128 ASCII graphic
//! characters. Strict enough to keep the JSON log field bounded
//! and printable, permissive enough to accept the desktop's
//! `C-<prefix>-<ulid>` ids and any reasonable client variant.

use axum::body::Body;
use axum::http::{HeaderMap, Request};
use tower_http::trace::MakeSpan;
use tracing::{info_span, Span};
use trace_id::TraceIdGenerator;

/// Header name. `HeaderMap` matches case-insensitively; the
/// lowercase form is the project-wide convention for
/// `HeaderName` literals.
pub(crate) const X_TRACE_ID_HEADER: &str = "x-trace-id";

/// Maximum accepted length of an inbound `X-Trace-ID` value.
/// Defensive against header-smuggling attempts; also keeps the JSON
/// log field bounded.
pub(crate) const MAX_TRACE_ID_LEN: usize = 128;

/// Span name produced for every request. Single span wraps the
/// whole exchange (tower-http's default produces a request /
/// response pair — we replace it with one span).
const SPAN_NAME: &str = "http_request";

/// A trace id is valid when it is 1..=MAX_TRACE_ID_LEN ASCII graphic
/// characters.
pub(crate) fn is_valid_trace_id(s: &str) -> bool {
    unimplemented!("implemented in Step 3")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_valid_trace_id_accepts_desktop_format() {
        // The desktop mints ids like
        // "C-desktop-01H9XQ8Z6VK3FJ4P5N2W7Y0T8CB"; the hyphens are
        // ASCII graphic, length is well under the cap.
        assert!(is_valid_trace_id("C-desktop-01H9XQ8Z6VK3FJ4P5N2W7Y0T8CB"));
    }

    #[test]
    fn is_valid_trace_id_accepts_server_format() {
        assert!(is_valid_trace_id("S-01H9XQ8Z6VK3FJ4P5N2W7Y0T8CB"));
    }

    #[test]
    fn is_valid_trace_id_rejects_empty() {
        assert!(!is_valid_trace_id(""));
    }

    #[test]
    fn is_valid_trace_id_rejects_overlong() {
        // 129 'x' characters — one past the cap.
        let huge = "x".repeat(MAX_TRACE_ID_LEN + 1);
        assert!(!is_valid_trace_id(&huge));
    }

    #[test]
    fn is_valid_trace_id_rejects_control_chars() {
        assert!(!is_valid_trace_id("bad\nvalue"));
        assert!(!is_valid_trace_id("bad\tvalue"));
        assert!(!is_valid_trace_id("bad\0value"));
    }

    #[test]
    fn is_valid_trace_id_accepts_max_length() {
        // Exactly 128 'x' characters — at the cap, must be accepted.
        let max = "x".repeat(MAX_TRACE_ID_LEN);
        assert!(is_valid_trace_id(&max));
    }
}
```

- [ ] **Step 2: Wire the module into `transport/http.rs`**

Open `apps/server/aegis-server/src/transport/http.rs`. Add `pub mod trace_id;` to the module list, alphabetically placed between `pub mod terminology;` and `pub mod user;`. After the edit, the block reads:

```rust
pub mod terminology;
pub mod trace_id;
pub mod user;
```

- [ ] **Step 3: Run the tests to confirm they fail**

Run: `cargo test -p aegis-server --lib transport::http::trace_id::tests`
Expected: FAIL — every test panics with `"not yet implemented"` because the body is `unimplemented!()`. Each test reports the panic from the unimplemented stub.

- [ ] **Step 4: Implement `is_valid_trace_id`**

Edit the body of `is_valid_trace_id` in `apps/server/aegis-server/src/transport/http/trace_id.rs`:

```rust
pub(crate) fn is_valid_trace_id(s: &str) -> bool {
    !s.is_empty() && s.len() <= MAX_TRACE_ID_LEN && s.chars().all(|c| c.is_ascii_graphic())
}
```

- [ ] **Step 5: Run the tests to confirm they pass**

Run: `cargo test -p aegis-server --lib transport::http::trace_id::tests`
Expected: 6 PASS (the `is_valid_trace_id` group).

- [ ] **Step 6: Commit**

```bash
git add apps/server/aegis-server/src/transport/http/trace_id.rs apps/server/aegis-server/src/transport/http.rs
git commit -m "feat(server): add trace_id module with is_valid_trace_id helper"
```

---

## Task 3: Add `extract_trace_id` with TDD

**Files:**
- Modify: `apps/server/aegis-server/src/transport/http/trace_id.rs`

**Interfaces:**
- Consumes: `is_valid_trace_id` (defined in Task 2), `X_TRACE_ID_HEADER`, `MAX_TRACE_ID_LEN`.
- Produces:
  - `pub(crate) fn extract_trace_id(headers: &HeaderMap, generator: &TraceIdGenerator) -> String`

- [ ] **Step 1: Add the failing tests**

In `apps/server/aegis-server/src/transport/http/trace_id.rs`, append the following block to the `mod tests` block (after the `is_valid_trace_id_accepts_max_length` test). The test cases drive the new `extract_trace_id` helper against `HeaderMap`s.

```rust
    fn headers_with(kv: &[(&'static str, &'static str)]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for (k, v) in kv {
            h.insert(*k, (*v).parse().expect("valid HeaderValue"));
        }
        h
    }

    #[test]
    fn extract_trace_id_uses_header_when_present_and_valid() {
        let headers = headers_with(&[("x-trace-id", "C-desktop-01H9XQ8Z6VK3FJ4P5N2W7Y0T8CB")]);
        let gen = TraceIdGenerator::new(None);
        assert_eq!(
            extract_trace_id(&headers, &gen),
            "C-desktop-01H9XQ8Z6VK3FJ4P5N2W7Y0T8CB"
        );
    }

    #[test]
    fn extract_trace_id_generates_when_header_missing() {
        let headers = HeaderMap::new();
        let gen = TraceIdGenerator::new(None);
        let id = extract_trace_id(&headers, &gen);
        assert!(id.starts_with("S-"), "expected server-side fallback, got {id:?}");
    }

    #[test]
    fn extract_trace_id_generates_when_header_empty() {
        let headers = headers_with(&[("x-trace-id", "")]);
        let gen = TraceIdGenerator::new(None);
        let id = extract_trace_id(&headers, &gen);
        assert!(id.starts_with("S-"), "expected server-side fallback, got {id:?}");
    }

    #[test]
    fn extract_trace_id_generates_when_header_too_long() {
        let huge = "x".repeat(MAX_TRACE_ID_LEN + 1);
        let headers = headers_with(&[("x-trace-id", &huge)]);
        let gen = TraceIdGenerator::new(None);
        let id = extract_trace_id(&headers, &gen);
        assert!(id.starts_with("S-"), "expected server-side fallback, got {id:?}");
    }

    #[test]
    fn extract_trace_id_generates_when_header_non_graphic() {
        let headers = headers_with(&[("x-trace-id", "bad\nvalue")]);
        let gen = TraceIdGenerator::new(None);
        let id = extract_trace_id(&headers, &gen);
        assert!(id.starts_with("S-"), "expected server-side fallback, got {id:?}");
    }

    #[test]
    fn extract_trace_id_generates_distinct_ids_across_calls() {
        // Sanity check that we are falling back to the generator and
        // not returning a constant.
        let headers = HeaderMap::new();
        let gen = TraceIdGenerator::new(None);
        let a = extract_trace_id(&headers, &gen);
        let b = extract_trace_id(&headers, &gen);
        assert_ne!(a, b);
    }
```

- [ ] **Step 2: Run the tests to confirm they fail**

Run: `cargo test -p aegis-server --lib transport::http::trace_id::tests::extract_trace_id`
Expected: FAIL — `extract_trace_id` is not defined; the compiler reports an unresolved import / undefined function error.

- [ ] **Step 3: Implement `extract_trace_id`**

In `apps/server/aegis-server/src/transport/http/trace_id.rs`, add this implementation just below `is_valid_trace_id`:

```rust
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
```

- [ ] **Step 4: Run the tests to confirm they pass**

Run: `cargo test -p aegis-server --lib transport::http::trace_id::tests`
Expected: 12 PASS (6 `is_valid_trace_id` + 6 `extract_trace_id`).

- [ ] **Step 5: Commit**

```bash
git add apps/server/aegis-server/src/transport/http/trace_id.rs
git commit -m "feat(server): add extract_trace_id helper with fallback"
```

---

## Task 4: Add `TraceIdMakeSpan` with TDD

**Files:**
- Modify: `apps/server/aegis-server/src/transport/http/trace_id.rs`

**Interfaces:**
- Consumes: `extract_trace_id`, `SPAN_NAME`, `TraceIdGenerator`.
- Produces:
  - `pub(crate) struct TraceIdMakeSpan { generator: TraceIdGenerator }`
  - `impl TraceIdMakeSpan { pub(crate) fn new(generator: TraceIdGenerator) -> Self }`
  - `impl tower_http::trace::MakeSpan<Body> for TraceIdMakeSpan`

- [ ] **Step 1: Add the failing test**

In `apps/server/aegis-server/src/transport/http/trace_id.rs`, append the following test to `mod tests`. It drives `make_span` against a synthetic request and asserts the field names on the resulting `Span` metadata.

```rust
    #[test]
    fn make_span_includes_required_field_names() {
        use axum::body::Body;

        let headers = headers_with(&[("x-trace-id", "C-desktop-01H9XQ8Z6VK3FJ4P5N2W7Y0T8CB")]);
        let request = Request::builder()
            .method("GET")
            .uri("/healthz")
            .headers(headers)
            .body(Body::empty())
            .expect("valid request");

        let mut maker = TraceIdMakeSpan::new(TraceIdGenerator::new(None));
        let span = maker.make_span(&request);

        let metadata = span.metadata().expect("span has metadata");
        let field_names: Vec<&'static str> =
            metadata.fields().iter().map(|f| f.name()).collect();
        assert!(field_names.contains(&"trace_id"), "fields were {field_names:?}");
        assert!(field_names.contains(&"method"), "fields were {field_names:?}");
        assert!(field_names.contains(&"path"), "fields were {field_names:?}");
    }
```

- [ ] **Step 2: Run the test to confirm it fails**

Run: `cargo test -p aegis-server --lib transport::http::trace_id::tests::make_span_includes_required_field_names`
Expected: FAIL — `TraceIdMakeSpan` is not defined; the compiler reports an undefined type.

- [ ] **Step 3: Implement `TraceIdMakeSpan` and the `MakeSpan` impl**

In `apps/server/aegis-server/src/transport/http/trace_id.rs`, add this implementation just below `extract_trace_id`:

```rust
/// `MakeSpan` impl that produces an `http_request` span carrying
/// `trace_id`, `method`, and `path`. The `trace_id` is taken from
/// the inbound `X-Trace-ID` header when present and valid; otherwise
/// it is freshly minted via [`TraceIdGenerator::server_side`].
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
```

- [ ] **Step 4: Run the test to confirm it passes**

Run: `cargo test -p aegis-server --lib transport::http::trace_id::tests`
Expected: 13 PASS (12 from earlier tasks + 1 new).

- [ ] **Step 5: Commit**

```bash
git add apps/server/aegis-server/src/transport/http/trace_id.rs
git commit -m "feat(server): add TraceIdMakeSpan MakeSpan impl"
```

---

## Task 5: Wire `TraceIdMakeSpan` into the router and add an integration test

**Files:**
- Modify: `apps/server/aegis-server/src/transport/http/router.rs`

**Interfaces:**
- Consumes: `TraceIdMakeSpan` (defined in Task 4), `TraceIdGenerator` (workspace crate).
- Produces: `pub fn router(state: AppState) -> axum::Router` now wraps the entire router in a `TraceLayer` whose span maker is `TraceIdMakeSpan`. The existing public signature is unchanged.

- [ ] **Step 1: Add the failing integration test**

In `apps/server/aegis-server/src/transport/http/router.rs`, scroll to the `#[cfg(test)] mod tests` block. Append the following test at the end of the module (just before the closing `}` of `mod tests`):

```rust
    /// `GET /healthz` with a valid `X-Trace-ID` header still
    /// returns 200 OK — proves the new `TraceIdMakeSpan` layer
    /// does not break the existing path. The value-level
    /// extraction is covered by the unit tests on
    /// `extract_trace_id` / `is_valid_trace_id` in
    /// `transport::http::trace_id`.
    #[tokio::test]
    async fn request_with_trace_id_header_succeeds() {
        let app = router(test_state());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/healthz")
                    .header("x-trace-id", "C-desktop-01ABCDEF01234567890ABCDEF")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::OK);
    }
```

- [ ] **Step 2: Skip the failure gate for this integration test**

This test is a wiring smoke test, not a behaviour test — it only asserts `200 OK` against `/healthz`, which the default `TraceLayer` would also satisfy. The strict TDD discipline has already been applied to `extract_trace_id` (Task 3) and `TraceIdMakeSpan` (Task 4); the value-level assertion lives there. Proceed directly to wiring the new layer.

- [ ] **Step 3: Wire `TraceIdMakeSpan` into the router**

In `apps/server/aegis-server/src/transport/http/router.rs`, edit the imports at the top of the file. Add the following two lines, kept together near the other `crate::transport::http::...` imports:

```rust
use crate::transport::http::trace_id::TraceIdMakeSpan;
use trace_id::TraceIdGenerator;
```

Then locate the closing block of `pub fn router(state: AppState) -> axum::Router { ... }`:

```rust
    router
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api))
        .layer(tower_http::trace::TraceLayer::new_for_http())
}
```

Replace the trailing `.layer(...)` call:

```rust
    router
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api))
        .layer(
            tower_http::trace::TraceLayer::new_for_http()
                .make_span_with(TraceIdMakeSpan::new(TraceIdGenerator::new(None))),
        )
}
```

The generator has no device prefix because the server is not a specific device.

- [ ] **Step 4: Run the new integration test (and the surrounding tests) to confirm they pass**

Run: `cargo test -p aegis-server --lib transport::http::router::tests`
Expected: all PASS — the new `request_with_trace_id_header_succeeds` test joins the existing router tests; no regression.

- [ ] **Step 5: Run the full `aegis-server` test suite**

Run: `cargo test -p aegis-server`
Expected: all PASS — every existing unit + integration test stays green; the new tests all pass.

- [ ] **Step 6: Commit**

```bash
git add apps/server/aegis-server/src/transport/http/router.rs
git commit -m "feat(server): wire TraceIdMakeSpan into router TraceLayer"
```

---

## Task 6: Final verification gate

**Files:** none (verification only).

- [ ] **Step 1: Run `cargo fmt` check**

Run: `cargo fmt --all -- --check`
Expected: no diff.

- [ ] **Step 2: Run clippy on `aegis-server`**

Run: `cargo clippy -p aegis-server --all-targets --all-features -- -D warnings`
Expected: PASS, no warnings.

- [ ] **Step 3: Run the full `aegis-server` test suite once more**

Run: `cargo test -p aegis-server`
Expected: all PASS.

- [ ] **Step 4: Run a cheap cross-crate sanity check**

Run: `cargo check --workspace`
Expected: PASS — the new `trace-id` dep on `aegis-server` does not break any other crate; the `trace-id` crate is unchanged.

- [ ] **Step 5: Summarise the diff**

Run: `git log --oneline main..HEAD` and `git diff --stat main..HEAD`
Expected: ~6 new commits (Tasks 1, 3, 4, 5, plus the spec commit from the previous step). New file `apps/server/aegis-server/src/transport/http/trace_id.rs`; small edits to `Cargo.toml`, `transport/http.rs`, `transport/http/router.rs`.