use serde::{Deserialize, Serialize};
use std::fmt;

/// Supported languages in the dialect coach system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Spanish,
    Arabic,
    French,
    English,
    Japanese,
}

impl Language {
    /// Get all supported languages
    pub fn all() -> Vec<Language> {
        vec![
            Language::Spanish,
            Language::Arabic,
            Language::French,
            Language::English,
            Language::Japanese,
        ]
    }

    /// Get the ISO 639-1 language code
    pub fn code(&self) -> &'static str {
        match self {
            Language::Spanish => "es",
            Language::Arabic => "ar",
            Language::French => "fr",
            Language::English => "en",
            Language::Japanese => "ja",
        }
    }

    /// Get the English name of the language
    pub fn name(&self) -> &'static str {
        match self {
            Language::Spanish => "Spanish",
            Language::Arabic => "Arabic",
            Language::French => "French",
            Language::English => "English",
            Language::Japanese => "Japanese",
        }
    }

    /// Get the language family (returns self as Language represents language families)
    pub fn language_family(&self) -> Language {
        *self
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
        assert_eq!(Language::English.code(), "en");
        assert_eq!(Language::Japanese.code(), "ja");
    }

    #[test]
    fn test_all_languages() {
        let languages = Language::all();
        assert_eq!(languages.len(), 5);
        assert!(languages.contains(&Language::Spanish));
        assert!(languages.contains(&Language::Arabic));
        assert!(languages.contains(&Language::French));
        assert!(languages.contains(&Language::English));
        assert!(languages.contains(&Language::Japanese));
    }

    #[test]
    fn test_language_family() {
        assert_eq!(Language::Spanish.language_family(), Language::Spanish);
        assert_eq!(Language::Arabic.language_family(), Language::Arabic);
        assert_eq!(Language::French.language_family(), Language::French);
        assert_eq!(Language::English.language_family(), Language::English);
        assert_eq!(Language::Japanese.language_family(), Language::Japanese);
    }
}
