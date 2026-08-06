//! In-memory [`TokenVersionCache`] backed by an `Arc<RwLock<HashMap>>`.
//!
//! Default cache backend. A future Redis backend can be added as a
//! sibling module `adapter::cache::redis` additively.

pub mod token_version;

#[cfg(test)]
mod tests;
