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
use logging_utils::TraceIdGenerator;
use tower_http::trace::MakeSpan;
use tracing::{Span, info_span};

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
    !s.is_empty() && s.len() <= MAX_TRACE_ID_LEN && s.chars().all(|c| c.is_ascii_graphic())
}

/// Extract `X-Trace-ID` from the request headers, falling back to a
/// freshly-minted server-side id when the header is missing,
/// malformed UTF-8, empty, too long, or contains non-graphic bytes.
pub(crate) fn extract_trace_id_from(headers: &HeaderMap, generator: &TraceIdGenerator) -> String {
    let raw = headers.get(X_TRACE_ID_HEADER).and_then(|v| v.to_str().ok());
    extract_trace_id(raw, generator)
}

/// Pure validation + fallback. `raw` is the already-decoded header
/// value (`None` if absent or not valid UTF-8). Splitting the
/// `HeaderMap` lookup out of this helper keeps it trivially
/// unit-testable without building a `HeaderValue`.
pub(crate) fn extract_trace_id(raw: Option<&str>, generator: &TraceIdGenerator) -> String {
    raw.filter(|s| is_valid_trace_id(s))
        .map(str::to_owned)
        .unwrap_or_else(|| generator.server_side())
}

/// `MakeSpan` impl that produces an `http_request` span carrying
/// `trace_id`, `method`, and `path`. The `trace_id` is taken from
/// the inbound `X-Trace-ID` header when present and valid; otherwise
/// it is freshly minted via [`TraceIdGenerator::server_side`].
///
/// `Clone` is required by `tower_http::trace::TraceLayer`, which
/// holds onto the span maker on every request clone.
#[derive(Clone)]
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
        let trace_id = extract_trace_id_from(request.headers(), &self.generator);
        info_span!(
            SPAN_NAME,
            trace_id = %trace_id,
            method   = %request.method(),
            path     = %request.uri().path(),
        )
    }
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

    #[test]
    fn extract_trace_id_uses_header_when_present_and_valid() {
        let raw = Some("C-desktop-01H9XQ8Z6VK3FJ4P5N2W7Y0T8CB");
        let generator = TraceIdGenerator::new(None);
        assert_eq!(
            extract_trace_id(raw, &generator),
            "C-desktop-01H9XQ8Z6VK3FJ4P5N2W7Y0T8CB"
        );
    }

    #[test]
    fn extract_trace_id_generates_when_header_missing() {
        let generator = TraceIdGenerator::new(None);
        let id = extract_trace_id(None, &generator);
        assert!(
            id.starts_with("S-"),
            "expected server-side fallback, got {id:?}"
        );
    }

    #[test]
    fn extract_trace_id_generates_when_header_empty() {
        let generator = TraceIdGenerator::new(None);
        let id = extract_trace_id(Some(""), &generator);
        assert!(
            id.starts_with("S-"),
            "expected server-side fallback, got {id:?}"
        );
    }

    #[test]
    fn extract_trace_id_generates_when_header_too_long() {
        let huge = "x".repeat(MAX_TRACE_ID_LEN + 1);
        let generator = TraceIdGenerator::new(None);
        let id = extract_trace_id(Some(&huge), &generator);
        assert!(
            id.starts_with("S-"),
            "expected server-side fallback, got {id:?}"
        );
    }

    #[test]
    fn extract_trace_id_generates_when_header_non_graphic() {
        // `extract_trace_id` takes the already-decoded &str so we
        // can drive the non-graphic path without building a
        // `HeaderValue` (the `http` crate has no constructor that
        // accepts control bytes).
        let generator = TraceIdGenerator::new(None);
        let id = extract_trace_id(Some("bad\nvalue"), &generator);
        assert!(
            id.starts_with("S-"),
            "expected server-side fallback, got {id:?}"
        );
    }

    #[test]
    fn extract_trace_id_generates_distinct_ids_across_calls() {
        // Sanity check that we are falling back to the generator and
        // not returning a constant.
        let generator = TraceIdGenerator::new(None);
        let a = extract_trace_id(None, &generator);
        let b = extract_trace_id(None, &generator);
        assert_ne!(a, b);
    }

    #[test]
    fn extract_trace_id_from_uses_header_when_present_and_valid() {
        let mut headers = HeaderMap::new();
        headers.insert(
            X_TRACE_ID_HEADER,
            "C-desktop-01H9XQ8Z6VK3FJ4P5N2W7Y0T8CB"
                .parse()
                .expect("valid HeaderValue"),
        );
        let generator = TraceIdGenerator::new(None);
        assert_eq!(
            extract_trace_id_from(&headers, &generator),
            "C-desktop-01H9XQ8Z6VK3FJ4P5N2W7Y0T8CB"
        );
    }

    #[test]
    fn extract_trace_id_from_generates_when_header_absent() {
        let headers = HeaderMap::new();
        let generator = TraceIdGenerator::new(None);
        let id = extract_trace_id_from(&headers, &generator);
        assert!(
            id.starts_with("S-"),
            "expected server-side fallback, got {id:?}"
        );
    }

    #[test]
    fn make_span_includes_required_field_names() {
        let request = Request::builder()
            .method("GET")
            .uri("/healthz")
            .header(X_TRACE_ID_HEADER, "C-desktop-01H9XQ8Z6VK3FJ4P5N2W7Y0T8CB")
            .body(Body::empty())
            .expect("valid request");

        let mut maker = TraceIdMakeSpan::new(TraceIdGenerator::new(None));
        let span = maker.make_span(&request);

        let metadata = span.metadata().expect("span has metadata");
        let field_names: Vec<&'static str> = metadata.fields().iter().map(|f| f.name()).collect();
        assert!(
            field_names.contains(&"trace_id"),
            "fields were {field_names:?}"
        );
        assert!(
            field_names.contains(&"method"),
            "fields were {field_names:?}"
        );
        assert!(field_names.contains(&"path"), "fields were {field_names:?}");
    }
}
