use serde::{Deserialize, Serialize};
use std::fmt;

/// Trait for text-to-speech providers (Google, Azure, Browser, etc.)
#[async_trait::async_trait]
pub trait TextToSpeechProvider: Send + Sync {
    /// Synthesize speech from text
    async fn synthesize(&self, request: TtsRequest) -> Result<TtsResponse, TtsError>;

    /// Get the provider name (e.g., "Google Neural2", "Browser", "Azure")
    fn provider_name(&self) -> &'static str;
}

/// Request for text-to-speech synthesis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsRequest {
    /// User ID for usage tracking
    pub user_id: uuid::Uuid,

    /// Text to synthesize
    pub text: String,

    /// BCP-47 language code (e.g., "es-MX", "ar-EG", "fr-CA")
    pub language_code: String,

    /// Speech rate (0.25-4.0, default 1.0)
    pub rate: Option<f32>,
}

impl TtsRequest {
    /// Create a simple TTS request with just text and language
    pub fn new(user_id: uuid::Uuid, text: String, language_code: String) -> Self {
        Self {
            user_id,
            text,
            language_code,
            rate: None,
        }
    }

    /// Set speech rate
    pub fn with_rate(mut self, rate: f32) -> Self {
        self.rate = Some(rate);
        self
    }
}

/// Response from text-to-speech synthesis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsResponse {
    /// Audio data (MP3, WAV, or OGG bytes)
    pub audio_data: Vec<u8>,

    /// Audio format
    pub audio_format: AudioFormat,

    /// Duration in milliseconds
    pub duration_ms: u32,

    /// Cache key for this audio (hash of request parameters)
    pub cache_key: String,
}

/// Audio format for synthesized speech
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioFormat {
    Mp3,
    Wav,
    Ogg,
}

impl fmt::Display for AudioFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mp3 => write!(f, "audio/mpeg"),
            Self::Wav => write!(f, "audio/wav"),
            Self::Ogg => write!(f, "audio/ogg"),
        }
    }
}

/// Information about an available voice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceInfo {
    /// Voice identifier (provider-specific)
    pub id: String,

    /// Human-readable voice name
    pub name: String,

    /// Language codes this voice supports
    pub language_codes: Vec<String>,

    /// Gender of the voice
    pub gender: VoiceGender,

    /// Whether this is a neural/premium voice
    pub is_neural: bool,
}

/// Gender of a voice
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VoiceGender {
    Male,
    Female,
    Neutral,
}

/// Errors that can occur during TTS operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TtsError {
    /// Network or API error
    NetworkError(String),

    /// Invalid request parameters
    InvalidRequest(String),

    /// Audio format not supported
    UnsupportedFormat(String),

    /// Voice not found
    VoiceNotFound(String),

    /// API quota exceeded
    QuotaExceeded,

    /// Authentication failed
    AuthenticationFailed,

    /// Unknown error
    Unknown(String),
}

impl fmt::Display for TtsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NetworkError(msg) => write!(f, "Network error: {}", msg),
            Self::InvalidRequest(msg) => write!(f, "Invalid request: {}", msg),
            Self::UnsupportedFormat(msg) => write!(f, "Unsupported format: {}", msg),
            Self::VoiceNotFound(msg) => write!(f, "Voice not found: {}", msg),
            Self::QuotaExceeded => write!(f, "API quota exceeded"),
            Self::AuthenticationFailed => write!(f, "Authentication failed"),
            Self::Unknown(msg) => write!(f, "Unknown error: {}", msg),
        }
    }
}

impl std::error::Error for TtsError {}

/// Generate a cache key from a TTS request
pub fn generate_cache_key(request: &TtsRequest) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    request.text.hash(&mut hasher);
    request.language_code.hash(&mut hasher);

    // Convert rate/pitch to deterministic strings
    let rate_str = request
        .rate
        .map(|r| format!("{:.2}", r))
        .unwrap_or_default();
    rate_str.hash(&mut hasher);

    format!("tts:{:x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_generation() {
        let user_id = uuid::Uuid::new_v4();
        let request1 = TtsRequest::new(user_id, "Hello".to_string(), "en-US".to_string());
        let request2 = TtsRequest::new(user_id, "Hello".to_string(), "en-US".to_string());

        assert_eq!(generate_cache_key(&request1), generate_cache_key(&request2));
    }

    #[test]
    fn test_different_text_different_key() {
        let user_id = uuid::Uuid::new_v4();
        let request1 = TtsRequest::new(user_id, "Hello".to_string(), "en-US".to_string());
        let request2 = TtsRequest::new(user_id, "Goodbye".to_string(), "en-US".to_string());

        assert_ne!(generate_cache_key(&request1), generate_cache_key(&request2));
    }
}
