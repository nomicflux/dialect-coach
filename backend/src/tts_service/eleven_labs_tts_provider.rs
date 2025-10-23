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
}

fn map_language_to_voice(language_code: &str) -> &'static str {
    tracing::info!("Mapping language code {} to voice", language_code);
    match language_code {
        "es-MX" => "spPXlKT5a4JMfbhPRAzA",
        "es-ES" => "zRUArUmK0DWSP7K6mmLW",
        "es-AR" => "XmoCtjPCefjeLDu0eMSl",
        "es-CU" => "1hB7zCGWj11SeMuBseeI",
        "es-CL" => "nNS8uylvF9GBWVSiIt5h",
        "es-CO" => "86V9x9hrQds83qf7zaGn",

        "ar-EG" => "LXrTqFIgiubkrMkwvOUr",
        "ar-LB" => "4wf10lgibMnboGJGCLrP",
        //"ar-SA" => "ar-SA-ZariyahNeural",
        //"ar-MA" => "ar-MA-MounaNeural",
        //"ar-IQ" => "ar-IQ-RanaNeural",
        "fr-CA" => "j9RedbMRSNQ74PyikQwD",
        //"fr-FR" => "fr-FR-DeniseNeural",
        //"fr-CH" => "fr-CH-ArianeNeural",
        //"fr-BE" => "fr-BE-CharlineNeural",
        //"fr-CI" => "fr-CI-AkanNeural",

        // Fallback to known working voice
        _ => "en-US-AriaNeural", // This is confirmed to exist
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
        let voice_params = VoiceParams {
            stability: 0.3,
            style: 3.0,
            similarity_boost: 0.5,
        };
        let request_body = ElevenLabsTtsRequest {
            text: request.text.clone(),
            model_id: self.model.clone(),
            voice_params: Some(voice_params)
        };
        let mut params = HashMap::new();
        params.insert("output_format", "mp3_22050_32");

        let uri = format!(
            "{}text-to-speech/{}",
            self.endpoint,
            map_language_to_voice(&request.language_code)
        );
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
}
