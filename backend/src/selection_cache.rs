use dialect_coach_shared::models::PhraseTranslation;
use lru::LruCache;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct SelectionCache {
    cache: Arc<Mutex<LruCache<String, Vec<PhraseTranslation>>>>,
}

impl SelectionCache {
    pub fn new(capacity: usize) -> Self {
        let cache = Arc::new(Mutex::new(LruCache::new(
            NonZeroUsize::new(capacity).unwrap(),
        )));
        Self { cache }
    }

    pub fn generate_key(operation: &str, dialect_id: &str, text: &str) -> String {
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        format!("{}:{}:{:x}", operation, dialect_id, hasher.finish())
    }

    pub async fn get_translation(&self, key: &str) -> Option<Vec<PhraseTranslation>> {
        let mut cache = self.cache.lock().await;
        cache.get(key).cloned()
    }

    pub async fn set_translation(&self, key: &str, value: Vec<PhraseTranslation>) {
        let mut cache = self.cache.lock().await;
        cache.put(key.to_string(), value);
    }
}
