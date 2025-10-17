use super::Language;
use serde::{Deserialize, Serialize};
use std::fmt;

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
    #[serde(rename = "spanish_caribbean")]
    SpanishCaribbean,
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
            | Self::SpanishCaribbean
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
            Self::SpanishCaribbean => "spanish_caribbean",
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
            Self::SpanishCaribbean => "Caribbean Spanish",
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
            Self::SpanishCaribbean => "es-CU", // Cuban as representative
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
            "es-PR" | "es-CU" | "es-DO" => Some(Self::SpanishCaribbean),
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

    /// Get all dialects for a specific language
    pub fn for_language(language: Language) -> Vec<Dialect> {
        match language {
            Language::Spanish => vec![
                Self::SpanishMexican,
                Self::SpanishCastilian,
                Self::SpanishArgentinian,
                Self::SpanishCaribbean,
                Self::SpanishChilean,
                Self::SpanishColombian,
            ],
            Language::Arabic => vec![
                Self::ArabicEgyptian,
                Self::ArabicLevantine,
                Self::ArabicGulf,
                Self::ArabicMaghrebi,
                Self::ArabicIraqi,
            ],
            Language::French => vec![
                Self::FrenchQuebecois,
                Self::FrenchParisian,
                Self::FrenchSwiss,
                Self::FrenchBelgian,
                Self::FrenchAfrican,
            ],
        }
    }
}

impl fmt::Display for Dialect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Formality level of language use
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Formality {
    Formal,
    Casual,
    #[serde(rename = "dialect_rich")]
    DialectRich,
    Slang,
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
    /// Immersive: Only speak in target dialect, no corrections
    Immersive,
    /// Corrective: Point out mistakes and provide corrections
    Corrective,
    /// Explanatory: Explain grammar, usage, and cultural context
    Explanatory,
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
        let spanish_dialects = Dialect::for_language(Language::Spanish);
        assert!(spanish_dialects.len() >= 3);
        assert!(spanish_dialects.contains(&Dialect::SpanishMexican));
    }
}
