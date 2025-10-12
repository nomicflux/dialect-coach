use serde::{Deserialize, Serialize};
use std::fmt;

/// Supported languages in the dialect coach system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Spanish,
    Arabic,
    French,
}

impl Language {
    /// Get all supported languages
    pub fn all() -> Vec<Language> {
        vec![Language::Spanish, Language::Arabic, Language::French]
    }

    /// Get the ISO 639-1 language code
    pub fn code(&self) -> &'static str {
        match self {
            Language::Spanish => "es",
            Language::Arabic => "ar",
            Language::French => "fr",
        }
    }

    /// Get the English name of the language
    pub fn name(&self) -> &'static str {
        match self {
            Language::Spanish => "Spanish",
            Language::Arabic => "Arabic",
            Language::French => "French",
        }
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_code() {
        assert_eq!(Language::Spanish.code(), "es");
        assert_eq!(Language::Arabic.code(), "ar");
        assert_eq!(Language::French.code(), "fr");
    }

    #[test]
    fn test_all_languages() {
        let languages = Language::all();
        assert_eq!(languages.len(), 3);
        assert!(languages.contains(&Language::Spanish));
        assert!(languages.contains(&Language::Arabic));
        assert!(languages.contains(&Language::French));
    }
}
