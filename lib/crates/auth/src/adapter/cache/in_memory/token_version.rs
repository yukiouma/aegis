use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;

use crate::domain::TokenVersionCache;

pub struct InMemoryTokenVersionCache {
    inner: Arc<RwLock<HashMap<String, u32>>>,
}

impl Default for InMemoryTokenVersionCache {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryTokenVersionCache {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl TokenVersionCache for InMemoryTokenVersionCache {
    async fn get(&self, code: &str) -> Option<u32> {
        self.inner.read().unwrap().get(code).copied()
    }

    async fn put(&self, code: &str, version: u32) {
        self.inner
            .write()
            .unwrap()
            .insert(code.to_string(), version);
    }
}
