//! Tests for [`InMemoryTokenVersionCache`].

use std::sync::Arc;

use crate::domain::TokenVersionCache;

use super::InMemoryTokenVersionCache;

#[tokio::test]
async fn get_returns_none_for_unknown_code() {
    let cache = InMemoryTokenVersionCache::new();
    assert_eq!(cache.get("ghost").await, None);
}

#[tokio::test]
async fn put_then_get_returns_stored_value() {
    let cache = InMemoryTokenVersionCache::new();
    cache.put("u1", 7).await;
    assert_eq!(cache.get("u1").await, Some(7));
}

#[tokio::test]
async fn put_overwrites_prior_value() {
    let cache = InMemoryTokenVersionCache::new();
    cache.put("u1", 7).await;
    cache.put("u1", 8).await;
    assert_eq!(cache.get("u1").await, Some(8));
}

#[tokio::test]
async fn cache_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<InMemoryTokenVersionCache>();
    assert_send_sync::<Arc<dyn TokenVersionCache>>();
}

#[tokio::test]
async fn multiple_caches_do_not_share_state() {
    let a = InMemoryTokenVersionCache::new();
    let b = InMemoryTokenVersionCache::new();
    a.put("u1", 1).await;
    assert_eq!(b.get("u1").await, None);
}
