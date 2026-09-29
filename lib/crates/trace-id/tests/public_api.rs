//! Public-API compile test for the `trace-id` crate.
//!
//! Locks the documented import path and the public surface so a
//! regression in `src/lib.rs` is caught at `cargo test -p
//! trace-id` time.

use trace_id::{Side, TraceIdGenerator};

/// Every public type in `trace_id` is nameable from the test.
#[test]
fn public_types_are_nameable() {
    fn assert_gen(_: TraceIdGenerator) {}
    fn assert_side(_: Side) {}

    // `TraceIdGenerator` is constructible with and without a prefix.
    assert_gen(TraceIdGenerator::new(None));
    assert_gen(TraceIdGenerator::new(Some("desktop".to_string())));

    // Touch both `Side` variants.
    let _: Side = Side::Client;
    let _: Side = Side::Server;
    let _ = assert_gen;
    let _ = assert_side;
}

/// Calling both methods on the same instance returns distinct
/// ids prefixed with the expected side identifier.
#[test]
fn both_sides_produce_prefixed_ids() {
    let generator = TraceIdGenerator::new(Some("host".to_string()));
    assert!(generator.client_side().starts_with("C-host-"));
    assert!(generator.server_side().starts_with("S-host-"));
}
