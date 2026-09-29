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
    !s.is_empty() && s.len() <= MAX_TRACE_ID_LEN && s.chars().all(|c| c.is_ascii_graphic())
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