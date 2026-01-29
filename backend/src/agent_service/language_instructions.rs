use dialect_coach_shared::models::{ArabicScript, JapaneseScript, LanguageOption};

pub fn build_language_instruction(language_option: &Option<LanguageOption>) -> String {
    match language_option {
        None => String::new(),
        Some(LanguageOption::Arabic(ArabicScript::Naskh)) => {
            "You must respond using Arabic script in Naskh style.".to_string()
        }
        Some(LanguageOption::Arabic(ArabicScript::Ruqa)) => {
            "You must respond using Arabic script in Ruq'a style.".to_string()
        }
        Some(LanguageOption::Arabic(ArabicScript::Latin)) => {
            "You must respond using romanized Arabic script. Use the appropriate romanization system for the user's dialect.".to_string()
        }
        Some(LanguageOption::Arabic(ArabicScript::FullyVoweled)) => {
            "You must respond using fully-voweled Arabic script with complete harakat (tashkeel) on all letters.".to_string()
        }
        Some(LanguageOption::Japanese(JapaneseScript::Romaji)) => {
            "You must respond using romanized Japanese (romaji).".to_string()
        }
        Some(LanguageOption::Japanese(JapaneseScript::OnlyKana)) => {
            "You must respond using only hiragana and katakana. Do not use kanji.".to_string()
        }
        Some(LanguageOption::Japanese(JapaneseScript::KanjiWithRuby)) => {
            "You must respond using kanji with furigana. ONLY wrap specific Kanji characters in ruby tags. NEVER wrap Hiragana, Katakana, or Okurigana (trailing hiragana). Examples: \n- Good: <ruby>私<rt>わたし</rt></ruby>は<ruby>食<rt>た</rt></ruby>べます\n- Bad (Wrapping Hiragana): <ruby>さむい<rt>さむい</rt></ruby>\n- Bad (Wrapping Okurigana): <ruby>飲んでいる<rt>のんでいる</rt></ruby>\n- Correct Okurigana handling: <ruby>飲<rt>の</rt></ruby>んでいる (only wrap the Kanji '飲')".to_string()
        }
        Some(LanguageOption::Japanese(JapaneseScript::Kanji)) => {
            "You must respond using standard kanji without annotations.".to_string()
        }
    }
}

/// Returns instructions for generating pronunciation text for TTS.
/// The dialect_name is used to ensure pronunciation matches the specific dialect.
pub fn build_pronunciation_instruction(
    language_option: &Option<LanguageOption>,
    dialect_name: &str,
) -> String {
    match language_option {
        Some(LanguageOption::Arabic(ArabicScript::Naskh | ArabicScript::Ruqa)) => {
            format!(
                r#"PRONUNCIATION FOR TTS: Also provide a "pronunciation_text" field containing the same response but with full harakat (tashkeel vowel marks) on all letters so TTS can pronounce it correctly.

CRITICAL: The harakat MUST reflect {dialect_name} pronunciation, NOT Modern Standard Arabic (Fus7a). Place vowel marks to show how a native {dialect_name} speaker actually pronounces each word. For example:
- Use dialect-specific vowel patterns (e.g., "i" sounds that would be "a" in MSA)
- Reflect dropped or altered vowels common in {dialect_name}
- Show sukun where consonant clusters occur in the dialect
The pronunciation_text must sound like natural {dialect_name} when read aloud, not like formal Arabic."#,
                dialect_name = dialect_name
            )
        }
        Some(LanguageOption::Japanese(JapaneseScript::Kanji)) => {
            format!(
                r#"PRONUNCIATION FOR TTS: Also provide a "pronunciation_text" field containing the same response but with furigana using ruby tags (e.g., <ruby>漢字<rt>かんじ</rt></ruby>) so TTS can pronounce kanji correctly.

CRITICAL: The furigana MUST reflect {dialect_name} pronunciation. Use readings that match how a native {dialect_name} speaker actually pronounces each word, including:
- Dialect-specific readings for common words
- Regional pronunciation variations
- Colloquial contractions and sound changes typical of {dialect_name}
The pronunciation_text must sound like natural {dialect_name} when read aloud."#,
                dialect_name = dialect_name
            )
        }
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_none_returns_empty() {
        assert_eq!(build_language_instruction(&None), "");
    }

    #[test]
    fn test_arabic_naskh() {
        let option = Some(LanguageOption::Arabic(ArabicScript::Naskh));
        assert_eq!(
            build_language_instruction(&option),
            "You must respond using Arabic script in Naskh style."
        );
    }

    #[test]
    fn test_arabic_ruqa() {
        let option = Some(LanguageOption::Arabic(ArabicScript::Ruqa));
        assert_eq!(
            build_language_instruction(&option),
            "You must respond using Arabic script in Ruq'a style."
        );
    }

    #[test]
    fn test_arabic_latin() {
        let option = Some(LanguageOption::Arabic(ArabicScript::Latin));
        assert_eq!(
            build_language_instruction(&option),
            "You must respond using romanized Arabic script. Use the appropriate romanization system for the user's dialect."
        );
    }

    #[test]
    fn test_arabic_fully_voweled() {
        let option = Some(LanguageOption::Arabic(ArabicScript::FullyVoweled));
        assert_eq!(
            build_language_instruction(&option),
            "You must respond using fully-voweled Arabic script with complete harakat (tashkeel) on all letters."
        );
    }

    #[test]
    fn test_japanese_romaji() {
        let option = Some(LanguageOption::Japanese(JapaneseScript::Romaji));
        assert_eq!(
            build_language_instruction(&option),
            "You must respond using romanized Japanese (romaji)."
        );
    }

    #[test]
    fn test_japanese_only_kana() {
        let option = Some(LanguageOption::Japanese(JapaneseScript::OnlyKana));
        assert_eq!(
            build_language_instruction(&option),
            "You must respond using only hiragana and katakana. Do not use kanji."
        );
    }

    #[test]
    fn test_japanese_kanji_with_ruby() {
        let option = Some(LanguageOption::Japanese(JapaneseScript::KanjiWithRuby));
        assert_eq!(
            build_language_instruction(&option),
            "You must respond using kanji with furigana. ONLY wrap specific Kanji characters in ruby tags. NEVER wrap Hiragana, Katakana, or Okurigana (trailing hiragana). Examples: \n- Good: <ruby>私<rt>わたし</rt></ruby>は<ruby>食<rt>た</rt></ruby>べます\n- Bad (Wrapping Hiragana): <ruby>さむい<rt>さむい</rt></ruby>\n- Bad (Wrapping Okurigana): <ruby>飲んでいる<rt>のんでいる</rt></ruby>\n- Correct Okurigana handling: <ruby>飲<rt>の</rt></ruby>んでいる (only wrap the Kanji '飲')"
        );
    }

    #[test]
    fn test_japanese_kanji() {
        let option = Some(LanguageOption::Japanese(JapaneseScript::Kanji));
        assert_eq!(
            build_language_instruction(&option),
            "You must respond using standard kanji without annotations."
        );
    }

    #[test]
    fn test_pronunciation_instruction_arabic_naskh() {
        let option = Some(LanguageOption::Arabic(ArabicScript::Naskh));
        let result = build_pronunciation_instruction(&option, "Levantine Arabic");
        assert!(result.contains("pronunciation_text"));
        assert!(result.contains("harakat"));
        assert!(result.contains("Levantine Arabic"));
        assert!(result.contains("NOT Modern Standard Arabic"));
    }

    #[test]
    fn test_pronunciation_instruction_arabic_ruqa() {
        let option = Some(LanguageOption::Arabic(ArabicScript::Ruqa));
        let result = build_pronunciation_instruction(&option, "Egyptian Arabic");
        assert!(result.contains("pronunciation_text"));
        assert!(result.contains("harakat"));
        assert!(result.contains("Egyptian Arabic"));
    }

    #[test]
    fn test_pronunciation_instruction_arabic_latin() {
        let option = Some(LanguageOption::Arabic(ArabicScript::Latin));
        assert!(build_pronunciation_instruction(&option, "Levantine Arabic").is_empty());
    }

    #[test]
    fn test_pronunciation_instruction_arabic_fully_voweled() {
        let option = Some(LanguageOption::Arabic(ArabicScript::FullyVoweled));
        assert!(build_pronunciation_instruction(&option, "Levantine Arabic").is_empty());
    }

    #[test]
    fn test_pronunciation_instruction_japanese_kanji() {
        let option = Some(LanguageOption::Japanese(JapaneseScript::Kanji));
        let result = build_pronunciation_instruction(&option, "Kansai Japanese");
        assert!(result.contains("pronunciation_text"));
        assert!(result.contains("furigana"));
        assert!(result.contains("Kansai Japanese"));
    }

    #[test]
    fn test_pronunciation_instruction_japanese_kanji_with_ruby() {
        let option = Some(LanguageOption::Japanese(JapaneseScript::KanjiWithRuby));
        assert!(build_pronunciation_instruction(&option, "Tokyo Japanese").is_empty());
    }

    #[test]
    fn test_pronunciation_instruction_none() {
        assert!(build_pronunciation_instruction(&None, "Spanish").is_empty());
    }
}
