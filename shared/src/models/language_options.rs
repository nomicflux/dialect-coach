use serde::{Deserialize, Serialize};
use std::fmt;

use super::language::Language;

/// Arabic script options for learning
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArabicScript {
    #[default]
    Naskh,
    Ruqa,
    Latin,
}

impl fmt::Display for ArabicScript {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            ArabicScript::Naskh => "Naskh",
            ArabicScript::Ruqa => "Ruq'a",
            ArabicScript::Latin => "Latin (Romanized)",
        };
        write!(f, "{}", label)
    }
}

/// Japanese script options for learning
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JapaneseScript {
    Romaji,
    OnlyKana,
    #[default]
    KanjiWithRuby,
    Kanji,
}

impl fmt::Display for JapaneseScript {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            JapaneseScript::Romaji => "Romaji",
            JapaneseScript::OnlyKana => "Kana Only",
            JapaneseScript::KanjiWithRuby => "Kanji with Furigana",
            JapaneseScript::Kanji => "Kanji",
        };
        write!(f, "{}", label)
    }
}

/// Language-specific options enum for type-safe handling
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LanguageOption {
    Arabic(ArabicScript),
    Japanese(JapaneseScript),
}

/// Options container for all language-specific settings
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LanguageOptions {
    pub arabic_script: Option<ArabicScript>,
    pub japanese_script: Option<JapaneseScript>,
}

impl LanguageOptions {
    /// Get the language option for a specific language
    pub fn for_language(&self, language: Language) -> Option<LanguageOption> {
        match language {
            Language::Arabic => self.arabic_script.map(LanguageOption::Arabic),
            Language::Japanese => self.japanese_script.map(LanguageOption::Japanese),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arabic_script_default() {
        assert_eq!(ArabicScript::default(), ArabicScript::Naskh);
    }

    #[test]
    fn test_arabic_script_display() {
        assert_eq!(ArabicScript::Naskh.to_string(), "Naskh");
        assert_eq!(ArabicScript::Ruqa.to_string(), "Ruq'a");
        assert_eq!(ArabicScript::Latin.to_string(), "Latin (Romanized)");
    }

    #[test]
    fn test_japanese_script_default() {
        assert_eq!(JapaneseScript::default(), JapaneseScript::KanjiWithRuby);
    }

    #[test]
    fn test_japanese_script_display() {
        assert_eq!(JapaneseScript::Romaji.to_string(), "Romaji");
        assert_eq!(JapaneseScript::OnlyKana.to_string(), "Kana Only");
        assert_eq!(
            JapaneseScript::KanjiWithRuby.to_string(),
            "Kanji with Furigana"
        );
        assert_eq!(JapaneseScript::Kanji.to_string(), "Kanji");
    }

    #[test]
    fn test_language_options_default() {
        let opts = LanguageOptions::default();
        assert_eq!(opts.arabic_script, None);
        assert_eq!(opts.japanese_script, None);
    }

    #[test]
    fn test_for_language_arabic() {
        let opts = LanguageOptions {
            arabic_script: Some(ArabicScript::Latin),
            japanese_script: None,
        };
        assert_eq!(
            opts.for_language(Language::Arabic),
            Some(LanguageOption::Arabic(ArabicScript::Latin))
        );
    }

    #[test]
    fn test_for_language_japanese() {
        let opts = LanguageOptions {
            arabic_script: None,
            japanese_script: Some(JapaneseScript::Romaji),
        };
        assert_eq!(
            opts.for_language(Language::Japanese),
            Some(LanguageOption::Japanese(JapaneseScript::Romaji))
        );
    }

    #[test]
    fn test_for_language_none() {
        let opts = LanguageOptions::default();
        assert_eq!(opts.for_language(Language::Arabic), None);
        assert_eq!(opts.for_language(Language::Japanese), None);
        assert_eq!(opts.for_language(Language::Spanish), None);
        assert_eq!(opts.for_language(Language::French), None);
        assert_eq!(opts.for_language(Language::English), None);
    }

    #[test]
    fn test_for_language_unsupported_languages() {
        let opts = LanguageOptions {
            arabic_script: Some(ArabicScript::Naskh),
            japanese_script: Some(JapaneseScript::Kanji),
        };
        assert_eq!(opts.for_language(Language::Spanish), None);
        assert_eq!(opts.for_language(Language::French), None);
        assert_eq!(opts.for_language(Language::English), None);
    }

    #[test]
    fn test_language_option_arabic_variant() {
        let opt = LanguageOption::Arabic(ArabicScript::Ruqa);
        assert_eq!(opt, LanguageOption::Arabic(ArabicScript::Ruqa));
    }

    #[test]
    fn test_language_option_japanese_variant() {
        let opt = LanguageOption::Japanese(JapaneseScript::OnlyKana);
        assert_eq!(opt, LanguageOption::Japanese(JapaneseScript::OnlyKana));
    }
}
