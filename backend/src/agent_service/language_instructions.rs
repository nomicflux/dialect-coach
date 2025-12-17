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
}
