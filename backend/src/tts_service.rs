use dialect_coach_shared::tts::{
    AudioFormat, TextToSpeechProvider, TtsError, TtsRequest, TtsResponse, VoiceInfo,
};
use reqwest::Client;
use std::sync::Arc;
use tracing::info;

/// Azure Speech Service Text-to-Speech provider
pub struct AzureTtsProvider {
    client: Client,
    subscription_key: String,
    endpoint: String,
}

impl AzureTtsProvider {
    pub fn new(subscription_key: String, region: String) -> Self {
        let endpoint = format!(
            "https://{}.tts.speech.microsoft.com/cognitiveservices/v1",
            region
        );
        Self {
            client: Client::new(),
            subscription_key,
            endpoint,
        }
    }

    pub fn from_env() -> Result<Self, TtsError> {
        let subscription_key =
            std::env::var("AZURE_SPEECH_KEY").map_err(|_| TtsError::AuthenticationFailed)?;
        let region =
            std::env::var("AZURE_SPEECH_REGION").map_err(|_| TtsError::AuthenticationFailed)?;

        if subscription_key.is_empty() || region.is_empty() {
            return Err(TtsError::AuthenticationFailed);
        }

        Ok(Self::new(subscription_key, region))
    }

    /// Map BCP-47 language code to Azure Neural voice ID
    fn map_language_to_voice(language_code: &str) -> &'static str {
        match language_code {
            // Spanish dialects
            "es-MX" => "es-MX-DaliaNeural",
            "es-ES" => "es-ES-ElviraNeural",
            "es-AR" => "es-AR-ElenaNeural",
            "es-CU" => "es-CU-BelkysNeural",
            "es-CL" => "es-CL-CatalinaNeural",
            "es-CO" => "es-CO-SalomeNeural",

            // Arabic dialects
            "ar-EG" => "ar-EG-SalmaNeural",
            "ar-LB" => "ar-LB-LaylaNeural",
            "ar-SA" => "ar-SA-ZariyahNeural",
            "ar-MA" => "ar-MA-MounaNeural",
            "ar-IQ" => "ar-IQ-RanaNeural",

            // French dialects
            "fr-CA" => "fr-CA-SylvieNeural",
            "fr-FR" => "fr-FR-DeniseNeural",
            "fr-CH" => "fr-CH-ArianeNeural",
            "fr-BE" => "fr-BE-CharlineNeural",
            "fr-CI" => "fr-CI-AkanNeural",

            // Default fallback
            _ => "en-US-AriaNeural",
        }
    }

    fn build_ssml(&self, request: &TtsRequest) -> String {
        let rate_str = request
            .rate
            .map(|r| format!(" rate='{}'", r))
            .unwrap_or_default();
        let pitch_str = request
            .pitch
            .map(|p| format!(" pitch='{}Hz'", p))
            .unwrap_or_default();

        // Use provided voice_id if available, otherwise map from language_code
        let voice_name = request
            .voice_id
            .as_deref()
            .unwrap_or_else(|| Self::map_language_to_voice(&request.language_code));

        format!(
            "<speak version='1.0' xmlns='http://www.w3.org/2001/10/synthesis' xml:lang='{}'>\n\
                <voice name='{}'>\n\
                    <prosody{}{}>{}</prosody>\n\
                </voice>\n\
            </speak>",
            request.language_code, voice_name, rate_str, pitch_str, request.text
        )
    }
}

#[async_trait::async_trait]
impl TextToSpeechProvider for AzureTtsProvider {
    async fn synthesize(&self, request: TtsRequest) -> Result<TtsResponse, TtsError> {
        let ssml = if request.ssml {
            request.text.clone()
        } else {
            self.build_ssml(&request)
        };

        let response = self
            .client
            .post(&self.endpoint)
            .header("Ocp-Apim-Subscription-Key", &self.subscription_key)
            .header("Content-Type", "application/ssml+xml")
            .header(
                "X-Microsoft-OutputFormat",
                "audio-24khz-48kbitrate-mono-mp3",
            )
            .header("User-Agent", "dialect-coach")
            .body(ssml)
            .send()
            .await
            .map_err(|e| TtsError::NetworkError(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return match status.as_u16() {
                401 | 403 => Err(TtsError::AuthenticationFailed),
                429 => Err(TtsError::QuotaExceeded),
                400 => Err(TtsError::InvalidRequest(error_body)),
                _ => Err(TtsError::NetworkError(format!(
                    "HTTP {}: {}",
                    status, error_body
                ))),
            };
        }

        let audio_data = response
            .bytes()
            .await
            .map_err(|e| TtsError::Unknown(format!("Failed to read response: {}", e)))?
            .to_vec();

        let estimated_duration_ms =
            (request.text.len() as f32 / 5.0 / 150.0 * 60.0 * 1000.0) as u32;

        Ok(TtsResponse {
            audio_data,
            audio_format: AudioFormat::Mp3,
            duration_ms: estimated_duration_ms,
            cache_key: dialect_coach_shared::tts::generate_cache_key(&request),
        })
    }

    async fn get_voices(&self, _language_code: &str) -> Result<Vec<VoiceInfo>, TtsError> {
        // Simplified implementation - return empty for now
        Ok(Vec::new())
    }

    fn provider_name(&self) -> &'static str {
        "Azure Neural"
    }
}

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

    /// Get available voices (not cached)
    pub async fn get_voices(&self, language_code: &str) -> Result<Vec<VoiceInfo>, TtsError> {
        self.provider.get_voices(language_code).await
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
