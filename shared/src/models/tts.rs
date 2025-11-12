use serde::{Deserialize, Serialize};

/// TTS provider type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TTSProviderType {
    #[serde(rename = "azure")]
    Azure,
    #[serde(rename = "elevenlabs")]
    ElevenLabs,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tts_provider_type_serialization() {
        let azure = TTSProviderType::Azure;
        let elevenlabs = TTSProviderType::ElevenLabs;

        let azure_json = serde_json::to_string(&azure).unwrap();
        let elevenlabs_json = serde_json::to_string(&elevenlabs).unwrap();

        assert_eq!(azure_json, "\"azure\"");
        assert_eq!(elevenlabs_json, "\"elevenlabs\"");

        let azure_deserialized: TTSProviderType = serde_json::from_str(&azure_json).unwrap();
        let elevenlabs_deserialized: TTSProviderType =
            serde_json::from_str(&elevenlabs_json).unwrap();

        assert_eq!(azure_deserialized, TTSProviderType::Azure);
        assert_eq!(elevenlabs_deserialized, TTSProviderType::ElevenLabs);
    }
}

