#![allow(dead_code)]

use dialect_coach_shared::models::TTSProviderType;
use dialect_coach_shared::models::dialect::dialect_features;
use dialect_coach_shared::tts::{
    AudioFormat, TextToSpeechProvider, TtsError, TtsRequest, TtsResponse,
};
use reqwest::Client;

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

    fn get_voice_id(
        dialect_features: &dialect_coach_shared::models::DialectWithFeatures,
    ) -> Result<String, TtsError> {
        dialect_features
            .tts_voices
            .get(&TTSProviderType::Azure)
            .and_then(|voice| voice.clone())
            .ok_or_else(|| {
                TtsError::VoiceNotFound(format!(
                    "No Azure voice available for dialect {}",
                    dialect_features.dialect.name()
                ))
            })
    }

    fn build_ssml(&self, request: &TtsRequest, voice_name: &str) -> String {
        let rate_str = request
            .rate
            .map(|r| format!(" rate='{}'", r))
            .unwrap_or_default();
        let lang_code = request.dialect.language().code();

        format!(
            "<speak version='1.0' xmlns='http://www.w3.org/2001/10/synthesis' xml:lang='{}'>\n\
                <voice name='{}'>\n\
                    <prosody{}{}>{}</prosody>\n\
                </voice>\n\
            </speak>",
            lang_code, voice_name, rate_str, "0.0Hz", request.text
        )
    }
}

#[async_trait::async_trait]
impl TextToSpeechProvider for AzureTtsProvider {
    async fn synthesize(&self, request: TtsRequest) -> Result<TtsResponse, TtsError> {
        let features = dialect_features(request.dialect);
        let voice_name = Self::get_voice_id(&features)?;
        let ssml = self.build_ssml(&request, &voice_name);

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

    fn provider_name(&self) -> &'static str {
        "Azure Neural"
    }

    fn provider_type(&self) -> TTSProviderType {
        TTSProviderType::Azure
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dialect_coach_shared::models::Dialect;
    use std::env;

    #[tokio::test]
    #[ignore] // Run with: cargo test -p dialect-coach-backend -- --ignored
    async fn test_verify_voices_against_azure_api() {
        // This test verifies all mapped voices exist in Azure's live API
        // Requires AZURE_SPEECH_REGION and AZURE_SPEECH_KEY environment variables
        let region = env::var("AZURE_SPEECH_REGION")
            .expect("AZURE_SPEECH_REGION must be set to run this test");
        let key =
            env::var("AZURE_SPEECH_KEY").expect("AZURE_SPEECH_KEY must be set to run this test");

        let client = reqwest::Client::new();
        let url = format!(
            "https://{}.tts.speech.microsoft.com/cognitiveservices/voices/list",
            region
        );

        let response = client
            .get(&url)
            .header("Ocp-Apim-Subscription-Key", &key)
            .send()
            .await
            .expect("Failed to call Azure voices API");

        assert!(
            response.status().is_success(),
            "Azure API call failed: {}",
            response.status()
        );

        let voices: Vec<serde_json::Value> = response
            .json()
            .await
            .expect("Failed to parse Azure voices JSON");

        // Extract all voice short names for lookup
        let voice_names: std::collections::HashSet<String> = voices
            .iter()
            .filter_map(|v| v["ShortName"].as_str())
            .map(|s| s.to_string())
            .collect();

        // Test all voices in our mapping
        let test_cases = [
            (Dialect::SpanishMexican, "es-MX-DaliaNeural"),
            (Dialect::SpanishCastilian, "es-ES-ElviraNeural"),
            (Dialect::SpanishArgentinian, "es-AR-ElenaNeural"),
            (Dialect::SpanishCuban, "es-CU-BelkysNeural"),
            (Dialect::SpanishChilean, "es-CL-CatalinaNeural"),
            (Dialect::SpanishColombian, "es-CO-SalomeNeural"),
            (Dialect::ArabicEgyptian, "ar-EG-SalmaNeural"),
            (Dialect::ArabicLevantine, "ar-LB-LaylaNeural"),
            (Dialect::ArabicGulf, "ar-SA-ZariyahNeural"),
            (Dialect::ArabicMaghrebi, "ar-MA-MounaNeural"),
            (Dialect::ArabicIraqi, "ar-IQ-RanaNeural"),
            (Dialect::FrenchQuebecois, "fr-CA-SylvieNeural"),
            (Dialect::FrenchParisian, "fr-FR-DeniseNeural"),
            (Dialect::FrenchSwiss, "fr-CH-ArianeNeural"),
            (Dialect::FrenchBelgian, "fr-BE-CharlineNeural"),
            // Note: FrenchAfrican is deliberately excluded as it doesn't exist in Azure
        ];

        for (dialect, expected_voice) in test_cases {
            let features = dialect_features(dialect);
            let mapped_voice = AzureTtsProvider::get_voice_id(&features)
                .unwrap_or_else(|_| panic!("Voice should exist for {:?}", dialect));
            assert_eq!(
                mapped_voice, expected_voice,
                "Voice mapping mismatch for dialect {:?}",
                dialect
            );

            assert!(
                voice_names.contains(expected_voice),
                "Voice '{}' for dialect '{:?}' not found in Azure API. Available voices: {:?}",
                expected_voice,
                dialect,
                voice_names.iter().take(5).collect::<Vec<_>>()
            );
        }

        println!(
            "✅ All {} voice mappings verified against Azure API",
            test_cases.len()
        );
    }

    #[test]
    fn test_azure_provider_type() {
        use dialect_coach_shared::tts::TextToSpeechProvider;
        let provider = AzureTtsProvider::new("test_key".to_string(), "eastus".to_string());
        assert_eq!(provider.provider_type(), TTSProviderType::Azure);
    }
}
