use dialect_coach_shared::models::TTSProviderType;
use dialect_coach_shared::models::dialect::dialect_features;
use dialect_coach_shared::tts::{
    AudioFormat, TextToSpeechProvider, TtsError, TtsRequest, TtsResponse,
};
use reqwest::Client;
use serde::Serialize;
use std::collections::HashMap;

pub struct ElevenLabsTtsProvider {
    client: Client,
    api_key: String,
    endpoint: String,
    model: String,
}

impl ElevenLabsTtsProvider {
    pub fn new(api_key: String, endpoint: String, model: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            endpoint,
            model,
        }
    }

    pub fn from_env() -> Result<Self, TtsError> {
        let subscription_key =
            std::env::var("ELEVEN_LABS_API_KEY").map_err(|_| TtsError::AuthenticationFailed)?;
        let endpoint =
            std::env::var("ELEVEN_LABS_URL").map_err(|_| TtsError::AuthenticationFailed)?;
        let model =
            std::env::var("ELEVEN_LABS_MODEL").map_err(|_| TtsError::AuthenticationFailed)?;

        if subscription_key.is_empty() || endpoint.is_empty() || model.is_empty() {
            return Err(TtsError::AuthenticationFailed);
        }

        Ok(Self::new(subscription_key, endpoint, model))
    }

    fn get_voice_id(
        dialect_features: &dialect_coach_shared::models::DialectWithFeatures,
    ) -> Result<String, TtsError> {
        dialect_features
            .tts_voices
            .get(&TTSProviderType::ElevenLabs)
            .and_then(|voice| voice.as_ref().map(|v| v.voice_name.clone()))
            .ok_or_else(|| {
                TtsError::VoiceNotFound(format!(
                    "No ElevenLabs voice available for dialect {}",
                    dialect_features.dialect.name()
                ))
            })
    }
}

#[derive(Serialize, Debug)]
struct VoiceParams {
    stability: f32,
    style: f32,
    similarity_boost: f32,
}

#[derive(Serialize, Debug)]
struct ElevenLabsTtsRequest {
    text: String,
    model_id: String,
    voice_params: Option<VoiceParams>,
}

#[async_trait::async_trait]
impl TextToSpeechProvider for ElevenLabsTtsProvider {
    async fn synthesize(&self, request: TtsRequest) -> Result<TtsResponse, TtsError> {
        tracing::info!("Synthesizing voice for request: {:?}", request);
        let features = dialect_features(request.dialect);
        let voice_id = Self::get_voice_id(&features)?;

        let voice_params = VoiceParams {
            stability: 0.3,
            style: 3.0,
            similarity_boost: 0.5,
        };
        let request_body = ElevenLabsTtsRequest {
            text: request.text.clone(),
            model_id: self.model.clone(),
            voice_params: Some(voice_params),
        };
        let mut params = HashMap::new();
        params.insert("output_format", "mp3_22050_32");

        let uri = format!("{}text-to-speech/{}", self.endpoint, voice_id.as_str());
        let response = self
            .client
            .post(&uri)
            .header("xi-api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .query(&params)
            .body(serde_json::to_string(&request_body).unwrap())
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

    fn provider_name(&self) -> &'static str {
        "Elevel Labs"
    }

    fn provider_type(&self) -> TTSProviderType {
        TTSProviderType::ElevenLabs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elevenlabs_provider_type() {
        use dialect_coach_shared::tts::TextToSpeechProvider;
        let provider = ElevenLabsTtsProvider::new(
            "test_key".to_string(),
            "https://api.elevenlabs.io/v1/".to_string(),
            "test_model".to_string(),
        );
        assert_eq!(provider.provider_type(), TTSProviderType::ElevenLabs);
    }
}
