use serde::{Deserialize, Serialize};

/// TTS provider type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TTSProviderType {
    #[serde(rename = "azure")]
    Azure,
    #[serde(rename = "elevenlabs")]
    ElevenLabs,
}

/// Gender presentation for TTS voices
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Gender {
    #[serde(rename = "male_presenting")]
    MalePresenting,
    #[serde(rename = "female_presenting")]
    FemalePresenting,
}

/// TTS voice configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TTSVoice {
    pub provider: TTSProviderType,
    pub voice_name: String,
    pub gender: Gender,
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

    #[test]
    fn test_gender_serialization() {
        let male = Gender::MalePresenting;
        let female = Gender::FemalePresenting;

        let male_json = serde_json::to_string(&male).unwrap();
        let female_json = serde_json::to_string(&female).unwrap();

        assert_eq!(male_json, "\"male_presenting\"");
        assert_eq!(female_json, "\"female_presenting\"");

        let male_deserialized: Gender = serde_json::from_str(&male_json).unwrap();
        let female_deserialized: Gender = serde_json::from_str(&female_json).unwrap();

        assert_eq!(male_deserialized, Gender::MalePresenting);
        assert_eq!(female_deserialized, Gender::FemalePresenting);
    }

    #[test]
    fn test_tts_voice_serialization() {
        let voice = TTSVoice {
            provider: TTSProviderType::Azure,
            voice_name: "es-AR-ElenaNeural".to_string(),
            gender: Gender::FemalePresenting,
        };

        let json = serde_json::to_string(&voice).unwrap();
        let deserialized: TTSVoice = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.provider, TTSProviderType::Azure);
        assert_eq!(deserialized.voice_name, "es-AR-ElenaNeural");
        assert_eq!(deserialized.gender, Gender::FemalePresenting);
    }
}
