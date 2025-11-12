use super::Language;
use super::TTSProviderType;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

/// Specific dialects within each language
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Dialect {
    // Spanish dialects
    #[serde(rename = "spanish_mexican")]
    SpanishMexican,
    #[serde(rename = "spanish_castilian")]
    SpanishCastilian,
    #[serde(rename = "spanish_argentinian")]
    SpanishArgentinian,
    #[serde(rename = "spanish_cuban")]
    SpanishCuban,
    #[serde(rename = "spanish_chilean")]
    SpanishChilean,
    #[serde(rename = "spanish_colombian")]
    SpanishColombian,

    // Arabic dialects
    #[serde(rename = "arabic_egyptian")]
    ArabicEgyptian,
    #[serde(rename = "arabic_levantine")]
    ArabicLevantine,
    #[serde(rename = "arabic_gulf")]
    ArabicGulf,
    #[serde(rename = "arabic_maghrebi")]
    ArabicMaghrebi,
    #[serde(rename = "arabic_iraqi")]
    ArabicIraqi,

    // French dialects
    #[serde(rename = "french_quebecois")]
    FrenchQuebecois,
    #[serde(rename = "french_parisian")]
    FrenchParisian,
    #[serde(rename = "french_swiss")]
    FrenchSwiss,
    #[serde(rename = "french_belgian")]
    FrenchBelgian,
    #[serde(rename = "french_african")]
    FrenchAfrican,
}

impl Dialect {
    /// Get the language this dialect belongs to
    pub fn language(&self) -> Language {
        match self {
            Self::SpanishMexican
            | Self::SpanishCastilian
            | Self::SpanishArgentinian
            | Self::SpanishCuban
            | Self::SpanishChilean
            | Self::SpanishColombian => Language::Spanish,

            Self::ArabicEgyptian
            | Self::ArabicLevantine
            | Self::ArabicGulf
            | Self::ArabicMaghrebi
            | Self::ArabicIraqi => Language::Arabic,

            Self::FrenchQuebecois
            | Self::FrenchParisian
            | Self::FrenchSwiss
            | Self::FrenchBelgian
            | Self::FrenchAfrican => Language::French,
        }
    }

    /// Get the machine-readable identifier (matches serde name)
    /// Used for database storage and filtering
    pub fn id(&self) -> &'static str {
        match self {
            Self::SpanishMexican => "spanish_mexican",
            Self::SpanishCastilian => "spanish_castilian",
            Self::SpanishArgentinian => "spanish_argentinian",
            Self::SpanishCuban => "spanish_cuban",
            Self::SpanishChilean => "spanish_chilean",
            Self::SpanishColombian => "spanish_colombian",

            Self::ArabicEgyptian => "arabic_egyptian",
            Self::ArabicLevantine => "arabic_levantine",
            Self::ArabicGulf => "arabic_gulf",
            Self::ArabicMaghrebi => "arabic_maghrebi",
            Self::ArabicIraqi => "arabic_iraqi",

            Self::FrenchQuebecois => "french_quebecois",
            Self::FrenchParisian => "french_parisian",
            Self::FrenchSwiss => "french_swiss",
            Self::FrenchBelgian => "french_belgian",
            Self::FrenchAfrican => "french_african",
        }
    }

    /// Get the human-readable name of the dialect
    pub fn name(&self) -> &'static str {
        match self {
            Self::SpanishMexican => "Mexican Spanish",
            Self::SpanishCastilian => "Castilian Spanish",
            Self::SpanishArgentinian => "Argentinian Spanish",
            Self::SpanishCuban => "Cuban Spanish",
            Self::SpanishChilean => "Chilean Spanish",
            Self::SpanishColombian => "Colombian Spanish",

            Self::ArabicEgyptian => "Egyptian Arabic",
            Self::ArabicLevantine => "Levantine Arabic",
            Self::ArabicGulf => "Gulf Arabic",
            Self::ArabicMaghrebi => "Maghrebi Arabic",
            Self::ArabicIraqi => "Iraqi Arabic",

            Self::FrenchQuebecois => "Quebec French",
            Self::FrenchParisian => "Parisian French",
            Self::FrenchSwiss => "Swiss French",
            Self::FrenchBelgian => "Belgian French",
            Self::FrenchAfrican => "African French",
        }
    }

    /// Get the BCP 47 language tag for speech APIs
    /// Returns (language-region) format like "es-MX" or "ar-EG"
    pub fn bcp47_tag(&self) -> &'static str {
        match self {
            Self::SpanishMexican => "es-MX",
            Self::SpanishCastilian => "es-ES",
            Self::SpanishArgentinian => "es-AR",
            Self::SpanishCuban => "es-CU",
            Self::SpanishChilean => "es-CL",
            Self::SpanishColombian => "es-CO",

            Self::ArabicEgyptian => "ar-EG",
            Self::ArabicLevantine => "ar-LB", // Lebanese as representative
            Self::ArabicGulf => "ar-SA",      // Saudi as representative
            Self::ArabicMaghrebi => "ar-MA",  // Moroccan as representative
            Self::ArabicIraqi => "ar-IQ",

            Self::FrenchQuebecois => "fr-CA",
            Self::FrenchParisian => "fr-FR",
            Self::FrenchSwiss => "fr-CH",
            Self::FrenchBelgian => "fr-BE",
            Self::FrenchAfrican => "fr-CI", // Ivorian as representative
        }
    }

    /// Parse BCP-47 language tag to Dialect
    /// Examples: "es-AR" → SpanishArgentinian, "es-CO" → SpanishColombian
    pub fn from_bcp47(tag: &str) -> Option<Self> {
        match tag {
            // Spanish dialects
            "es-AR" => Some(Self::SpanishArgentinian),
            "es-CO" => Some(Self::SpanishColombian),
            "es-CU" => Some(Self::SpanishCuban),
            "es-MX" => Some(Self::SpanishMexican),
            "es-ES" => Some(Self::SpanishCastilian),
            "es-CL" => Some(Self::SpanishChilean),

            // Arabic dialects
            "ar-EG" => Some(Self::ArabicEgyptian),
            "ar-LB" | "ar-SY" | "ar-JO" | "ar-PS" => Some(Self::ArabicLevantine),
            "ar-SA" | "ar-AE" | "ar-KW" | "ar-QA" | "ar-BH" | "ar-OM" => Some(Self::ArabicGulf),
            "ar-MA" | "ar-DZ" | "ar-TN" | "ar-LY" => Some(Self::ArabicMaghrebi),
            "ar-IQ" => Some(Self::ArabicIraqi),

            // French dialects
            "fr-CA" => Some(Self::FrenchQuebecois),
            "fr-FR" => Some(Self::FrenchParisian),
            "fr-CH" => Some(Self::FrenchSwiss),
            "fr-BE" => Some(Self::FrenchBelgian),
            "fr-CI" | "fr-SN" | "fr-CM" => Some(Self::FrenchAfrican),

            _ => None,
        }
    }

    fn all_spanish_dialects() -> Vec<Dialect> {
        vec![
            Self::SpanishMexican,
            Self::SpanishCastilian,
            Self::SpanishArgentinian,
            Self::SpanishCuban,
            Self::SpanishChilean,
            Self::SpanishColombian,
        ]
    }

    fn all_arabic_dialects() -> Vec<Dialect> {
        vec![
            Self::ArabicEgyptian,
            Self::ArabicLevantine,
            Self::ArabicGulf,
            Self::ArabicMaghrebi,
            Self::ArabicIraqi,
        ]
    }

    fn all_french_dialects() -> Vec<Dialect> {
        vec![
            Self::FrenchQuebecois,
            Self::FrenchParisian,
            Self::FrenchSwiss,
            Self::FrenchBelgian,
            Self::FrenchAfrican,
        ]
    }

    fn all_dialects_for_language(language: Language) -> Vec<Dialect> {
        match language {
            Language::Spanish => Self::all_spanish_dialects(),
            Language::Arabic => Self::all_arabic_dialects(),
            Language::French => Self::all_french_dialects(),
        }
    }

    /// Get all dialects for a specific language, optionally filtered by feature availability
    pub fn for_language(language: Language, with_tts: bool, with_corpus: bool) -> Vec<Dialect> {
        Self::all_dialects_for_language(language)
            .into_iter()
            .filter(|dialect| {
                let features = dialect_features(*dialect);
                let matches_tts = !with_tts || features.tts_voices.values().any(|voice| voice.is_some());
                let matches_corpus = !with_corpus || features.has_corpus;
                matches_tts && matches_corpus
            })
            .collect()
    }

    /// Get all supported dialects
    pub fn all() -> Vec<Dialect> {
        vec![
            Self::SpanishMexican,
            Self::SpanishCastilian,
            Self::SpanishArgentinian,
            Self::SpanishCuban,
            Self::SpanishChilean,
            Self::SpanishColombian,
            Self::ArabicEgyptian,
            Self::ArabicLevantine,
            Self::ArabicGulf,
            Self::ArabicMaghrebi,
            Self::ArabicIraqi,
            Self::FrenchQuebecois,
            Self::FrenchParisian,
            Self::FrenchSwiss,
            Self::FrenchBelgian,
            Self::FrenchAfrican,
        ]
    }

    /// Parse from serde ID format ("spanish_mexican", "arabic_egyptian", etc.)
    /// This is the canonical string format for database storage and serialization
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "spanish_mexican" => Some(Self::SpanishMexican),
            "spanish_castilian" => Some(Self::SpanishCastilian),
            "spanish_argentinian" => Some(Self::SpanishArgentinian),
            "spanish_cuban" => Some(Self::SpanishCuban),
            "spanish_chilean" => Some(Self::SpanishChilean),
            "spanish_colombian" => Some(Self::SpanishColombian),
            "arabic_egyptian" => Some(Self::ArabicEgyptian),
            "arabic_levantine" => Some(Self::ArabicLevantine),
            "arabic_gulf" => Some(Self::ArabicGulf),
            "arabic_maghrebi" => Some(Self::ArabicMaghrebi),
            "arabic_iraqi" => Some(Self::ArabicIraqi),
            "french_quebecois" => Some(Self::FrenchQuebecois),
            "french_parisian" => Some(Self::FrenchParisian),
            "french_swiss" => Some(Self::FrenchSwiss),
            "french_belgian" => Some(Self::FrenchBelgian),
            "french_african" => Some(Self::FrenchAfrican),
            _ => None,
        }
    }
}

impl fmt::Display for Dialect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// FromStr implementation for Dialect - ONLY accepts serde ID format
/// This enforces that string parsing uses the canonical database format
impl FromStr for Dialect {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_id(s).ok_or_else(|| {
            format!(
                "Invalid dialect ID: '{}'. Use serde ID format like 'spanish_mexican'.",
                s
            )
        })
    }
}

/// User feedback score for a dialect
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Feedback {
    pub score: Option<f64>,
}

/// Dialect with feature availability information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialectWithFeatures {
    pub dialect: Dialect,
    pub tts_voices: HashMap<TTSProviderType, Option<String>>,
    pub has_corpus: bool,
    pub feedback: Feedback,
}

impl From<Dialect> for DialectWithFeatures {
    fn from(dialect: Dialect) -> Self {
        dialect_features(dialect)
    }
}

fn build_voice_map(elevenlabs: Option<&str>, azure: Option<&str>) -> HashMap<TTSProviderType, Option<String>> {
    let mut voices = HashMap::new();
    voices.insert(TTSProviderType::ElevenLabs, elevenlabs.map(|s| s.to_string()));
    voices.insert(TTSProviderType::Azure, azure.map(|s| s.to_string()));
    voices
}

fn get_tts_voices(dialect: Dialect) -> HashMap<TTSProviderType, Option<String>> {
    match dialect {
        Dialect::SpanishArgentinian => build_voice_map(Some("XmoCtjPCefjeLDu0eMSl"), Some("es-AR-ElenaNeural")),
        Dialect::SpanishCuban => build_voice_map(Some("1hB7zCGWj11SeMuBseeI"), Some("es-CU-BelkysNeural")),
        Dialect::SpanishColombian => build_voice_map(Some("86V9x9hrQds83qf7zaGn"), Some("es-CO-SalomeNeural")),
        Dialect::ArabicEgyptian => build_voice_map(Some("LXrTqFIgiubkrMkwvOUr"), Some("ar-EG-SalmaNeural")),
        Dialect::ArabicLevantine => build_voice_map(Some("4wf10lgibMnboGJGCLrP"), Some("ar-LB-LaylaNeural")),
        Dialect::ArabicGulf => build_voice_map(Some("DANw8bnAVbjDEHwZIoYa"), Some("ar-SA-ZariyahNeural")),
        Dialect::FrenchQuebecois => build_voice_map(Some("j9RedbMRSNQ74PyikQwD"), Some("fr-CA-SylvieNeural")),
        Dialect::FrenchAfrican => build_voice_map(Some("FgHDn7bpgpKqz7QttoyC"), None),
        _ => build_voice_map(None, None),
    }
}

fn has_corpus(dialect: Dialect) -> bool {
    matches!(
        dialect,
        Dialect::SpanishArgentinian
            | Dialect::SpanishCuban
            | Dialect::SpanishColombian
            | Dialect::ArabicEgyptian
            | Dialect::ArabicLevantine
            | Dialect::ArabicGulf
            | Dialect::FrenchQuebecois
            | Dialect::FrenchAfrican
    )
}

/// Get feature availability for a dialect
pub fn dialect_features(dialect: Dialect) -> DialectWithFeatures {
    DialectWithFeatures {
        dialect,
        tts_voices: get_tts_voices(dialect),
        has_corpus: has_corpus(dialect),
        feedback: Feedback::default(),
    }
}

/// Formality level of language use
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Formality {
    #[serde(rename = "formal")]
    Formal,
    #[serde(rename = "casual")]
    Casual,
    #[serde(rename = "dialect_rich")]
    DialectRich,
    #[serde(rename = "slang")]
    Slang,
}

impl Formality {
    /// Get the serde ID for this formality level
    /// This is the canonical string format for database storage and serialization
    pub fn id(&self) -> &'static str {
        match self {
            Self::Formal => "formal",
            Self::Casual => "casual",
            Self::DialectRich => "dialect_rich",
            Self::Slang => "slang",
        }
    }

    /// Get the human-readable name of the formality level
    pub fn name(&self) -> &'static str {
        match self {
            Self::Formal => "Formal",
            Self::Casual => "Casual",
            Self::DialectRich => "Dialect-Rich",
            Self::Slang => "Slang",
        }
    }

    /// Parse from serde ID format ("formal", "casual", "dialect_rich", "slang")
    /// This is the canonical string format for database storage and serialization
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "formal" => Some(Self::Formal),
            "casual" => Some(Self::Casual),
            "dialect_rich" => Some(Self::DialectRich),
            "slang" => Some(Self::Slang),
            _ => None,
        }
    }
}

impl FromStr for Formality {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_id(s).ok_or_else(|| {
            format!(
                "Invalid formality ID: '{}'. Use serde ID format like 'casual' or 'dialect_rich'.",
                s
            )
        })
    }
}

impl fmt::Display for Formality {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Register (style) of language
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Register {
    Written,
    Spoken,
    SocialMedia,
}

/// Configuration for a dialect coach agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialectConfig {
    pub language: Language,
    pub dialect: Dialect,
    pub formality: Formality,
    pub teaching_mode: TeachingMode,
    pub personality_traits: Vec<String>,
}

/// Teaching mode for the dialect coach
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TeachingMode {
    #[serde(rename = "immersive")]
    Immersive,
    #[serde(rename = "corrective")]
    Corrective,
    #[serde(rename = "explanatory")]
    Explanatory,
    #[serde(rename = "interleaved")]
    Interleaved,
    #[serde(rename = "storyteller")]
    StoryTeller,
    #[serde(rename = "debug")]
    Debug,
}

impl Default for DialectConfig {
    fn default() -> Self {
        Self {
            language: Language::Spanish,
            dialect: Dialect::SpanishMexican,
            formality: Formality::Casual,
            teaching_mode: TeachingMode::Immersive,
            personality_traits: vec!["friendly".to_string(), "patient".to_string()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dialect_language() {
        assert_eq!(Dialect::SpanishMexican.language(), Language::Spanish);
        assert_eq!(Dialect::ArabicEgyptian.language(), Language::Arabic);
        assert_eq!(Dialect::FrenchQuebecois.language(), Language::French);
    }

    #[test]
    fn test_bcp47_tags() {
        assert_eq!(Dialect::SpanishMexican.bcp47_tag(), "es-MX");
        assert_eq!(Dialect::ArabicEgyptian.bcp47_tag(), "ar-EG");
        assert_eq!(Dialect::FrenchQuebecois.bcp47_tag(), "fr-CA");
    }

    #[test]
    fn test_dialects_for_language() {
        let spanish_dialects = Dialect::for_language(Language::Spanish, true, true);
        assert_eq!(spanish_dialects.len(), 3);
        assert!(spanish_dialects.contains(&Dialect::SpanishCuban));
        assert!(spanish_dialects.contains(&Dialect::SpanishArgentinian));
        assert!(spanish_dialects.contains(&Dialect::SpanishColombian));
    }

    #[test]
    fn test_dialects_for_language_no_filters() {
        let spanish_dialects = Dialect::for_language(Language::Spanish, false, false);
        assert_eq!(spanish_dialects.len(), 6);
    }

    #[test]
    fn test_dialects_for_language_with_tts_only() {
        let spanish_dialects = Dialect::for_language(Language::Spanish, true, false);
        assert!(spanish_dialects.len() >= 3);
        assert!(spanish_dialects.contains(&Dialect::SpanishCuban));
        assert!(spanish_dialects.contains(&Dialect::SpanishArgentinian));
        assert!(spanish_dialects.contains(&Dialect::SpanishColombian));
    }

    #[test]
    fn test_dialects_for_language_with_corpus_only() {
        let spanish_dialects = Dialect::for_language(Language::Spanish, false, true);
        assert!(spanish_dialects.len() >= 3);
        assert!(spanish_dialects.contains(&Dialect::SpanishCuban));
        assert!(spanish_dialects.contains(&Dialect::SpanishArgentinian));
        assert!(spanish_dialects.contains(&Dialect::SpanishColombian));
    }

    #[test]
    fn test_feedback_default() {
        let feedback = Feedback::default();
        assert_eq!(feedback.score, None);
    }

    #[test]
    fn test_dialect_with_features_from_dialect() {
        let features: DialectWithFeatures = Dialect::SpanishArgentinian.into();
        assert_eq!(features.dialect, Dialect::SpanishArgentinian);
        assert!(features.has_corpus);
        assert_eq!(features.feedback.score, None);
    }

    #[test]
    fn test_dialect_features_active_dialect() {
        let features = dialect_features(Dialect::SpanishCuban);
        assert_eq!(features.dialect, Dialect::SpanishCuban);
        assert!(features.has_corpus);
        assert!(features.tts_voices.get(&TTSProviderType::ElevenLabs).is_some());
        assert!(features.tts_voices.get(&TTSProviderType::ElevenLabs).unwrap().is_some());
        assert!(features.tts_voices.get(&TTSProviderType::Azure).is_some());
        assert!(features.tts_voices.get(&TTSProviderType::Azure).unwrap().is_some());
    }

    #[test]
    fn test_dialect_features_inactive_dialect() {
        let features = dialect_features(Dialect::SpanishMexican);
        assert_eq!(features.dialect, Dialect::SpanishMexican);
        assert!(!features.has_corpus);
        assert!(features.tts_voices.get(&TTSProviderType::ElevenLabs).is_some());
        assert!(features.tts_voices.get(&TTSProviderType::ElevenLabs).unwrap().is_none());
        assert!(features.tts_voices.get(&TTSProviderType::Azure).is_some());
        assert!(features.tts_voices.get(&TTSProviderType::Azure).unwrap().is_none());
    }

    #[test]
    fn test_dialect_features_french_african_no_azure() {
        let features = dialect_features(Dialect::FrenchAfrican);
        assert_eq!(features.dialect, Dialect::FrenchAfrican);
        assert!(features.has_corpus);
        assert!(features.tts_voices.get(&TTSProviderType::ElevenLabs).is_some());
        assert!(features.tts_voices.get(&TTSProviderType::ElevenLabs).unwrap().is_some());
        assert!(features.tts_voices.get(&TTSProviderType::Azure).is_some());
        assert!(features.tts_voices.get(&TTSProviderType::Azure).unwrap().is_none());
    }
}
