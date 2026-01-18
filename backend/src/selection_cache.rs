use dialect_coach_shared::models::{GrammarExplanation, PhraseTranslation};
use lru::LruCache;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub enum CachedResult {
    Translation(Vec<PhraseTranslation>),
    Grammar(Vec<GrammarExplanation>),
}

pub struct SelectionCache {
    cache: Arc<Mutex<LruCache<String, CachedResult>>>,
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
        match cache.get(key) {
            Some(CachedResult::Translation(t)) => Some(t.clone()),
            _ => None,
        }
    }

    pub async fn set_translation(&self, key: &str, value: Vec<PhraseTranslation>) {
        let mut cache = self.cache.lock().await;
        cache.put(key.to_string(), CachedResult::Translation(value));
    }

    pub async fn get_grammar(&self, key: &str) -> Option<Vec<GrammarExplanation>> {
        let mut cache = self.cache.lock().await;
        match cache.get(key) {
            Some(CachedResult::Grammar(g)) => Some(g.clone()),
            _ => None,
        }
    }

    pub async fn set_grammar(&self, key: &str, value: Vec<GrammarExplanation>) {
        let mut cache = self.cache.lock().await;
        cache.put(key.to_string(), CachedResult::Grammar(value));
    }
}
