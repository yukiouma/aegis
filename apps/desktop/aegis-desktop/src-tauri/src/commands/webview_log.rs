//! Tauri command that emits a webview `console.warn` / `console.error`
//! through the global subscriber, with no trace-id wrap and no http
//! call. Other levels are silently dropped — the frontend is the
//! primary gate, this is defense-in-depth.

use serde::Deserialize;

use crate::http::dto::ApiError;

/// Allowed level strings. The frontend pins this union; serde
/// rejects any non-`"warn"` / non-`"error"` string at the wire
/// boundary, so a stale build cannot widen the surface even by
/// mistake.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WebviewLogLevel {
    Warn,
    Error,
}

/// Emit a `console.warn` / `console.error` call through the global
/// `tracing` subscriber under the `aegis_desktop_lib::webview` target.
///
/// The command takes two positional parameters (`level`, `message`)
/// matching how `commands::auth::login` takes `code` / `password` —
/// the `api.forwardWebviewLog` wrapper on the TS side spreads them
/// as `{ level, message }`.
///
/// Intentionally does NOT mint a trace id, NOT open a `info_span!`,
/// and NOT scope a `TRACE_ID` task-local. Webview console output is
/// observability noise, not request-scoped telemetry.
#[tauri::command]
pub fn forward_webview_log(level: WebviewLogLevel, message: String) -> Result<(), ApiError> {
    match level {
        WebviewLogLevel::Warn => {
            tracing::warn!(
                target: "aegis_desktop_lib::webview",
                message = %message,
                "webview console.warn"
            );
        }
        WebviewLogLevel::Error => {
            tracing::error!(
                target: "aegis_desktop_lib::webview",
                message = %message,
                "webview console.error"
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    //! The level → tracing emission is the only behavior that lives
    //! in Rust. The frontend owns the "which console methods to
    //! intercept" decision; this command is a thin sink. Tests
    //! assert the level mapping and that the message is carried as
    //! the `message` field. A custom `Layer` captures every event
    //! fired through the dispatcher's `with_default` scope.

    use super::*;
    use std::sync::{Arc, Mutex};
    use tracing::field::Visit;
    use tracing::Subscriber;
    use tracing_subscriber::layer::Context;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::Layer;

    #[derive(Default, Clone)]
    struct Capture {
        events: Arc<Mutex<Vec<(tracing::Level, String, String)>>>,
    }

    struct MessageVisitor(String);

    impl Visit for MessageVisitor {
        fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
            if field.name() == "message" {
                self.0 = format!("{value:?}");
            }
        }
        fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
            if field.name() == "message" {
                self.0 = value.to_string();
            }
        }
    }

    impl<S: Subscriber> Layer<S> for Capture {
        fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
            let metadata = event.metadata();
            let mut visitor = MessageVisitor(String::new());
            event.record(&mut visitor);
            self.events.lock().unwrap().push((
                *metadata.level(),
                metadata.target().to_string(),
                visitor.0,
            ));
        }
    }

    fn with_capture<F: FnOnce()>(cap: &Capture, f: F) {
        let subscriber = tracing_subscriber::registry().with(cap.clone());
        tracing::subscriber::with_default(subscriber, f);
    }

    #[test]
    fn forward_warn_emits_warn_event_under_webview_target() {
        let cap = Capture::default();
        with_capture(&cap, || {
            forward_webview_log(WebviewLogLevel::Warn, "boom".into()).unwrap();
        });
        let events = cap.events.lock().unwrap();
        assert_eq!(events.len(), 1, "expected exactly one event");
        let (level, target, message) = &events[0];
        assert_eq!(*level, tracing::Level::WARN);
        assert_eq!(target, "aegis_desktop_lib::webview");
        assert_eq!(message, "boom");
    }

    #[test]
    fn forward_error_emits_error_event_under_webview_target() {
        let cap = Capture::default();
        with_capture(&cap, || {
            forward_webview_log(WebviewLogLevel::Error, "kaboom".into()).unwrap();
        });
        let events = cap.events.lock().unwrap();
        assert_eq!(events.len(), 1);
        let (level, target, message) = &events[0];
        assert_eq!(*level, tracing::Level::ERROR);
        assert_eq!(target, "aegis_desktop_lib::webview");
        assert_eq!(message, "kaboom");
    }

    #[test]
    fn forward_returns_ok_for_every_supported_level() {
        // The contract is that the command never fails — the IPC
        // surface is fire-and-forget. Pin the unit variants here so
        // a future addition without a matching match arm is caught
        // at compile time.
        assert!(forward_webview_log(WebviewLogLevel::Warn, "x".into()).is_ok());
        assert!(forward_webview_log(WebviewLogLevel::Error, "x".into()).is_ok());
    }

    #[test]
    fn webview_log_level_deserializes_lowercase_strings() {
        let w: WebviewLogLevel = serde_json::from_str("\"warn\"").unwrap();
        let e: WebviewLogLevel = serde_json::from_str("\"error\"").unwrap();
        assert_eq!(w, WebviewLogLevel::Warn);
        assert_eq!(e, WebviewLogLevel::Error);
    }
}
