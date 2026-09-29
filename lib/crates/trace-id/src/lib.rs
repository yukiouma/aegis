//! `trace-id` workspace crate.
//!
//! Generates trace ids that identify a single unit of work as it
//! crosses a client ↔ server boundary. A trace id is composed of
//! three optional segments joined by `-`:
//!
//! 1. **Side identifier** — `C` for client, `S` for server.
//! 2. **Device prefix** — caller-supplied free-form label
//!    (e.g. `"desktop"`). Omitted entirely when not provided.
//! 3. **Log id** — a time-sortable ULID.
//!
//! Examples:
//!
//! ```text
//! C-01H9XQ8Z6VK3FJ4P5N2W7Y0T8CB
//! S-desktop-01H9XQ9A2DF4GJ7M5P1R3V6X9BC
//! ```

use ulid::Ulid;

/// Which side of the wire a trace id was minted on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    /// Mints ids prefixed with `C`.
    Client,
    /// Mints ids prefixed with `S`.
    Server,
}

/// Mints trace ids composed of a side identifier, optional device
/// prefix, and a fresh ULID on every call.
///
/// Construct once per process (or per request-handling task) and
/// reuse — the generator itself is stateless; only `device_prefix`
/// is captured at construction time.
#[derive(Debug, Clone)]
pub struct TraceIdGenerator {
    device_prefix: Option<String>,
}

impl TraceIdGenerator {
    /// Build a generator. `device_prefix` is embedded between the
    /// side identifier and the ULID; pass `None` to omit it.
    pub fn new(device_prefix: Option<String>) -> Self {
        Self { device_prefix }
    }

    /// Mint a client-side trace id (`C-…`).
    pub fn client_side(&self) -> String {
        self.build(Side::Client)
    }

    /// Mint a server-side trace id (`S-…`).
    pub fn server_side(&self) -> String {
        self.build(Side::Server)
    }

    fn build(&self, side: Side) -> String {
        let prefix = match side {
            Side::Client => "C",
            Side::Server => "S",
        };
        let ulid = Ulid::generate().to_string();
        match &self.device_prefix {
            Some(device) => format!("{prefix}-{device}-{ulid}"),
            None => format!("{prefix}-{ulid}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_side_starts_with_c() {
        let generator = TraceIdGenerator::new(None);
        let id = generator.client_side();
        assert!(
            id.starts_with('C'),
            "expected to start with 'C', got {id:?}"
        );
        assert_eq!(
            id.chars().nth(1),
            Some('-'),
            "expected '-' after 'C', got {id:?}"
        );
    }

    #[test]
    fn server_side_starts_with_s() {
        let generator = TraceIdGenerator::new(None);
        let id = generator.server_side();
        assert!(
            id.starts_with('S'),
            "expected to start with 'S', got {id:?}"
        );
        assert_eq!(
            id.chars().nth(1),
            Some('-'),
            "expected '-' after 'S', got {id:?}"
        );
    }

    #[test]
    fn no_device_prefix_omits_middle_segment() {
        let generator = TraceIdGenerator::new(None);
        let id = generator.client_side();
        let parts: Vec<&str> = id.split('-').collect();
        assert_eq!(parts.len(), 2, "expected 2 segments, got {id:?}");
        assert_eq!(parts[0], "C");
    }

    #[test]
    fn device_prefix_appears_in_middle() {
        let generator = TraceIdGenerator::new(Some("desktop".to_string()));
        let id = generator.server_side();
        let parts: Vec<&str> = id.split('-').collect();
        assert_eq!(parts.len(), 3, "expected 3 segments, got {id:?}");
        assert_eq!(parts[0], "S");
        assert_eq!(parts[1], "desktop");
    }

    #[test]
    fn each_call_yields_a_fresh_ulid() {
        let generator = TraceIdGenerator::new(None);
        let a = generator.client_side();
        let b = generator.client_side();
        assert_ne!(a, b);
    }

    #[test]
    fn side_enum_variants_are_distinct() {
        assert_ne!(Side::Client, Side::Server);
    }
}
