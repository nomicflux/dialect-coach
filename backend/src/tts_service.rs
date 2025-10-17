use dialect_coach_shared::tts::{
    AudioFormat, TextToSpeechProvider, TtsError, TtsRequest, TtsResponse, VoiceGender,
    VoiceInfo,
};
use base64::prelude::*;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};

/// Google Cloud Text-to-Speech provider using Neural2 and Wavenet voices
pub struct GoogleTtsProvider {
    client: Client,
    api_key: String,
    endpoint: String,
}

impl GoogleTtsProvider {
    /// Create a new Google TTS provider
    ///
    /// # Arguments
    /// * `api_key` - Google Cloud API key with Text-to-Speech API enabled
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            endpoint: "https://texttospeech.googleapis.com/v1/text:synthesize".to_string(),
        }
    }

    /// Create provider from environment variable
    pub fn from_env() -> Result<Self, TtsError> {
        let api_key = std::env::var("GOOGLE_TTS_API_KEY").map_err(|_| {
            TtsError::AuthenticationFailed
        })?;

        if api_key.is_empty() {
            return Err(TtsError::AuthenticationFailed);
        }

        Ok(Self::new(api_key))
    }

    /// Convert our TtsRequest to Google's API format
    fn build_google_request(&self, request: &TtsRequest) -> GoogleTtsSynthesizeRequest {
        // Determine input type
        let input = if request.ssml {
            GoogleTtsInput {
                ssml: Some(request.text.clone()),
                text: None,
            }
        } else {
            GoogleTtsInput {
                ssml: None,
                text: Some(request.text.clone()),
            }
        };

        // Build voice selection
        let voice = GoogleTtsVoiceSelection {
            language_code: request.language_code.clone(),
            name: request.voice_id.clone(),
            ssml_gender: None, // Let Google choose based on voice name
        };

        // Build audio config
        let audio_config = GoogleTtsAudioConfig {
            audio_encoding: "MP3".to_string(),
            speaking_rate: request.rate,
            pitch: request.pitch,
            volume_gain_db: request.volume_gain_db,
            sample_rate_hertz: None,
            effects_profile_id: None,
        };

        GoogleTtsSynthesizeRequest {
            input,
            voice,
            audio_config,
        }
    }

    /// Parse Google's response and convert to our format
    fn parse_google_response(
        &self,
        response: GoogleTtsSynthesizeResponse,
        request: &TtsRequest,
    ) -> Result<TtsResponse, TtsError> {
        // Decode base64 audio content
        let audio_data = BASE64_STANDARD.decode(&response.audio_content).map_err(|e| {
            error!("Failed to decode base64 audio: {}", e);
            TtsError::Unknown(format!("Failed to decode audio: {}", e))
        })?;

        // Estimate duration (rough approximation: ~150 words per minute, ~5 chars per word)
        let estimated_duration_ms = (request.text.len() as f32 / 5.0 / 150.0 * 60.0 * 1000.0) as u32;

        Ok(TtsResponse {
            audio_data,
            audio_format: AudioFormat::Mp3,
            duration_ms: estimated_duration_ms,
            cache_key: dialect_coach_shared::tts::generate_cache_key(request),
        })
    }
}

#[async_trait::async_trait]
impl TextToSpeechProvider for GoogleTtsProvider {
    async fn synthesize(&self, request: TtsRequest) -> Result<TtsResponse, TtsError> {
        info!(
            "Synthesizing speech: {} chars in {} with voice {:?}",
            request.text.len(),
            request.language_code,
            request.voice_id
        );

        // Build Google API request
        let google_request = self.build_google_request(&request);

        // Make API call
        let url = format!("{}?key={}", self.endpoint, self.api_key);
        let response = self
            .client
            .post(&url)
            .json(&google_request)
            .send()
            .await
            .map_err(|e| {
                error!("Google TTS API request failed: {}", e);
                TtsError::NetworkError(format!("Request failed: {}", e))
            })?;

        // Check status code
        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            error!("Google TTS API error ({}): {}", status, error_body);

            return match status.as_u16() {
                401 | 403 => Err(TtsError::AuthenticationFailed),
                429 => Err(TtsError::QuotaExceeded),
                400 => Err(TtsError::InvalidRequest(error_body)),
                _ => Err(TtsError::NetworkError(format!("HTTP {}: {}", status, error_body))),
            };
        }

        // Parse response
        let google_response: GoogleTtsSynthesizeResponse = response.json().await.map_err(|e| {
            error!("Failed to parse Google TTS response: {}", e);
            TtsError::Unknown(format!("Failed to parse response: {}", e))
        })?;

        // Convert to our format
        let tts_response = self.parse_google_response(google_response, &request)?;

        info!(
            "Successfully synthesized {} bytes of audio",
            tts_response.audio_data.len()
        );

        Ok(tts_response)
    }

    async fn get_voices(&self, language_code: &str) -> Result<Vec<VoiceInfo>, TtsError> {
        info!("Fetching available voices for language: {}", language_code);

        let url = format!(
            "https://texttospeech.googleapis.com/v1/voices?key={}&languageCode={}",
            self.api_key, language_code
        );

        let response = self.client.get(&url).send().await.map_err(|e| {
            error!("Google TTS voices API request failed: {}", e);
            TtsError::NetworkError(format!("Request failed: {}", e))
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            error!("Google TTS voices API error ({}): {}", status, error_body);

            return match status.as_u16() {
                401 | 403 => Err(TtsError::AuthenticationFailed),
                _ => Err(TtsError::NetworkError(format!("HTTP {}: {}", status, error_body))),
            };
        }

        let voices_response: GoogleTtsVoicesResponse = response.json().await.map_err(|e| {
            error!("Failed to parse Google TTS voices response: {}", e);
            TtsError::Unknown(format!("Failed to parse response: {}", e))
        })?;

        // Convert to our format
        let voices: Vec<VoiceInfo> = voices_response
            .voices
            .into_iter()
            .map(|v| {
                let is_neural = v.name.contains("Neural2");
                let gender = match v.ssml_gender.as_deref() {
                    Some("MALE") => VoiceGender::Male,
                    Some("FEMALE") => VoiceGender::Female,
                    _ => VoiceGender::Neutral,
                };

                VoiceInfo {
                    id: v.name.clone(),
                    name: v.name,
                    language_codes: v.language_codes,
                    gender,
                    is_neural,
                }
            })
            .collect();

        info!("Found {} voices for {}", voices.len(), language_code);
        Ok(voices)
    }

    fn provider_name(&self) -> &'static str {
        "Google Neural2"
    }
}

// Google Cloud TTS API types

#[derive(Debug, Serialize)]
struct GoogleTtsSynthesizeRequest {
    input: GoogleTtsInput,
    voice: GoogleTtsVoiceSelection,
    #[serde(rename = "audioConfig")]
    audio_config: GoogleTtsAudioConfig,
}

#[derive(Debug, Serialize)]
struct GoogleTtsInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ssml: Option<String>,
}

#[derive(Debug, Serialize)]
struct GoogleTtsVoiceSelection {
    #[serde(rename = "languageCode")]
    language_code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "ssmlGender")]
    ssml_gender: Option<String>,
}

#[derive(Debug, Serialize)]
struct GoogleTtsAudioConfig {
    #[serde(rename = "audioEncoding")]
    audio_encoding: String,
    #[serde(skip_serializing_if = "Option::is_none", rename = "speakingRate")]
    speaking_rate: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pitch: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "volumeGainDb")]
    volume_gain_db: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "sampleRateHertz")]
    sample_rate_hertz: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "effectsProfileId")]
    effects_profile_id: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct GoogleTtsSynthesizeResponse {
    #[serde(rename = "audioContent")]
    audio_content: String, // Base64-encoded audio
}

#[derive(Debug, Deserialize)]
struct GoogleTtsVoicesResponse {
    voices: Vec<GoogleVoice>,
}

#[derive(Debug, Deserialize)]
struct GoogleVoice {
    #[serde(rename = "languageCodes")]
    language_codes: Vec<String>,
    name: String,
    #[serde(rename = "ssmlGender")]
    ssml_gender: Option<String>,
}

/// Azure Speech Service Text-to-Speech provider
pub struct AzureTtsProvider {
    client: Client,
    subscription_key: String,
    region: String,
    endpoint: String,
}

impl AzureTtsProvider {
    pub fn new(subscription_key: String, region: String) -> Self {
        let endpoint = format!("https://{}.tts.speech.microsoft.com/cognitiveservices/v1", region);
        Self {
            client: Client::new(),
            subscription_key,
            region,
            endpoint,
        }
    }

    pub fn from_env() -> Result<Self, TtsError> {
        let subscription_key = std::env::var("AZURE_SPEECH_KEY")
            .map_err(|_| TtsError::AuthenticationFailed)?;
        let region = std::env::var("AZURE_SPEECH_REGION")
            .map_err(|_| TtsError::AuthenticationFailed)?;

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
        let rate_str = request.rate.map(|r| format!(" rate='{}'", r)).unwrap_or_default();
        let pitch_str = request.pitch.map(|p| format!(" pitch='{}Hz'", p)).unwrap_or_default();

        // Use provided voice_id if available, otherwise map from language_code
        let voice_name = request.voice_id.as_deref()
            .unwrap_or_else(|| Self::map_language_to_voice(&request.language_code));

        format!(
            "<speak version='1.0' xmlns='http://www.w3.org/2001/10/synthesis' xml:lang='{}'>\n\
                <voice name='{}'>\n\
                    <prosody{}{}>{}</prosody>\n\
                </voice>\n\
            </speak>",
            request.language_code,
            voice_name,
            rate_str,
            pitch_str,
            request.text
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

        let response = self.client
            .post(&self.endpoint)
            .header("Ocp-Apim-Subscription-Key", &self.subscription_key)
            .header("Content-Type", "application/ssml+xml")
            .header("X-Microsoft-OutputFormat", "audio-24khz-48kbitrate-mono-mp3")
            .header("User-Agent", "dialect-coach")
            .body(ssml)
            .send()
            .await
            .map_err(|e| TtsError::NetworkError(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return match status.as_u16() {
                401 | 403 => Err(TtsError::AuthenticationFailed),
                429 => Err(TtsError::QuotaExceeded),
                400 => Err(TtsError::InvalidRequest(error_body)),
                _ => Err(TtsError::NetworkError(format!("HTTP {}: {}", status, error_body))),
            };
        }

        let audio_data = response.bytes().await
            .map_err(|e| TtsError::Unknown(format!("Failed to read response: {}", e)))?
            .to_vec();

        let estimated_duration_ms = (request.text.len() as f32 / 5.0 / 150.0 * 60.0 * 1000.0) as u32;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_google_request_building() {
        let provider = GoogleTtsProvider::new("test_key".to_string());

        let request = TtsRequest::new("Hello world".to_string(), "en-US".to_string())
            .with_voice("en-US-Neural2-A".to_string())
            .with_rate(0.9);

        let google_request = provider.build_google_request(&request);

        assert_eq!(google_request.input.text, Some("Hello world".to_string()));
        assert_eq!(google_request.voice.language_code, "en-US");
        assert_eq!(
            google_request.voice.name,
            Some("en-US-Neural2-A".to_string())
        );
        assert_eq!(google_request.audio_config.speaking_rate, Some(0.9));
    }

    #[test]
    fn test_ssml_request() {
        let provider = GoogleTtsProvider::new("test_key".to_string());

        let request = TtsRequest::new("<speak>Hello</speak>".to_string(), "en-US".to_string())
            .with_ssml();

        let google_request = provider.build_google_request(&request);

        assert_eq!(google_request.input.ssml, Some("<speak>Hello</speak>".to_string()));
        assert_eq!(google_request.input.text, None);
    }
}
