pub mod azure_tts_provider;
pub mod eleven_labs_tts_provider;

use dialect_coach_shared::tts::{
    TextToSpeechProvider, TtsError, TtsRequest, TtsResponse,
};
use std::sync::Arc;
use tracing::info;

/// Wrapper service that includes caching
pub struct TtsService {
    provider: Arc<dyn TextToSpeechProvider + Send + Sync>,
    cache: Arc<tokio::sync::Mutex<lru::LruCache<String, TtsResponse>>>,
}

impl TtsService {
    /// Create a new TTS service with the given provider
    pub fn new(provider: Arc<dyn TextToSpeechProvider + Send + Sync>) -> Self {
        // LRU cache with capacity for 100 responses (roughly 10-50MB depending on audio length)
        let cache = Arc::new(tokio::sync::Mutex::new(lru::LruCache::new(
            std::num::NonZeroUsize::new(100).unwrap(),
        )));

        Self { provider, cache }
    }

    /// Synthesize speech with caching
    pub async fn synthesize(&self, request: TtsRequest) -> Result<TtsResponse, TtsError> {
        let cache_key = dialect_coach_shared::tts::generate_cache_key(&request);

        // Check cache first
        {
            let mut cache = self.cache.lock().await;
            if let Some(cached) = cache.get(&cache_key) {
                info!("Cache hit for TTS request: {}", cache_key);
                return Ok(cached.clone());
            }
        }

        // Cache miss - synthesize
        info!("Cache miss for TTS request: {}", cache_key);
        let response = self.provider.synthesize(request).await?;

        // Store in cache
        {
            let mut cache = self.cache.lock().await;
            cache.put(cache_key, response.clone());
        }

        Ok(response)
    }

    /// Get the provider name
    pub fn provider_name(&self) -> &'static str {
        self.provider.provider_name()
    }

    /// Clear the cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.lock().await;
        cache.clear();
        info!("TTS cache cleared");
    }

    /// Get cache statistics
    pub async fn cache_stats(&self) -> (usize, usize) {
        let cache = self.cache.lock().await;
        (cache.len(), cache.cap().get())
    }
}
