//! Cache adapter implementations for [`TokenVersionCache`].
//!
//! `cache` itself is `pub(crate)` because callers reach concrete caches
//! via the layer boundary (`adapter::InMemoryTokenVersionCache`) and the
//! crate root. Today only the in-memory backend exists; a future Redis
//! backend can be added as a sibling module under `cache/redis.rs`
//! additively.

pub(crate) mod in_memory;
