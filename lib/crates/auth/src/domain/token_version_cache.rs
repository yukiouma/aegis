use async_trait::async_trait;

/// Outbound port for caching per-user JWT token versions.
///
/// Implementations are best-effort: a `get` that returns `None` may
/// indicate either a real cache miss or a transient cache failure
/// (e.g. a Redis timeout), and the caller is expected to fall back
/// to the source of truth (Postgres) on every `None`.
///
/// A `RedisTokenVersionCache` will live at `adapter::cache::redis`
/// and implement the same trait when that backend lands. Today's
/// only backend is `InMemoryTokenVersionCache` at
/// `adapter::cache::in_memory`.
#[async_trait]
pub trait TokenVersionCache: Send + Sync {
    /// Look up the cached `token_version` for `code`. Returns `None`
    /// on miss or transient failure.
    async fn get(&self, code: &str) -> Option<u32>;

    /// Store `version` for `code`. Overwrites any prior value. Best
    /// effort: failures (e.g. a network blip to a remote store) are
    /// swallowed because the cache is an optimization, not the source
    /// of truth.
    async fn put(&self, code: &str, version: u32);
}
