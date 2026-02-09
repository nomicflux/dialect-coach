use anyhow::Result;
use dialect_coach_shared::models::needs_pronunciation_text;
use dialect_coach_shared::{AgentUsage, Dialect, Language, LanguageOption};
use serde::Deserialize;
use std::sync::Arc;

use super::provider::{ANTHROPIC_PROVIDER, CompletionAgent, CompletionRequest};
use super::retry::retry_completion_call;
use super::util::{
    JSON_OUTPUT_INSTRUCTION, create_prefilled_assistant_message, normalize_json_response,
};

pub struct PronunciationAgentParams<'a> {
    pub response_text: &'a str,
    pub dialect: Dialect,
    pub language_option: &'a Option<LanguageOption>,
}

pub struct PronunciationAgentOutput {
    pub pronunciation_text: Option<String>,
}

impl PronunciationAgentOutput {
    pub fn empty() -> Self {
        Self {
            pronunciation_text: None,
        }
    }
}

#[derive(Deserialize)]
struct RawPronunciationOutput {
    pronunciation_text: String,
}

pub struct PronunciationAgent {
    agent: Arc<dyn CompletionAgent>,
}

impl PronunciationAgent {
    pub fn new(agent: Arc<dyn CompletionAgent>) -> Self {
        Self { agent }
    }

    pub async fn generate_pronunciation(
        &self,
        params: &PronunciationAgentParams<'_>,
    ) -> (Result<PronunciationAgentOutput>, Vec<AgentUsage>) {
        if !needs_pronunciation_text(params.dialect, params.language_option) {
            return (Ok(PronunciationAgentOutput::empty()), Vec::new());
        }

        let system_content =
            build_pronunciation_system_content(params.dialect, params.language_option);
        let prompt = build_pronunciation_prompt(params.response_text);

        let mut history = Vec::new();
        if self.agent.provider() == ANTHROPIC_PROVIDER {
            history.push(create_prefilled_assistant_message());
        }

        let request = CompletionRequest {
            preamble: &system_content,
            prompt: &prompt,
            history: &history,
            max_tokens: 2048,
            temperature: 0.0,
        };

        let (result, usage) = retry_completion_call(self.agent.as_ref(), &request, 3).await;
        match result {
            Ok(response) => match try_parse_pronunciation_output(&response) {
                Ok(output) => (Ok(output), usage),
                Err(e) => (Err(e), usage),
            },
            Err(e) => (Err(e), usage),
        }
    }
}

fn build_pronunciation_system_content(
    dialect: Dialect,
    language_option: &Option<LanguageOption>,
) -> String {
    let dialect_name = dialect.name();
    let specific_instructions = match language_option {
        Some(LanguageOption::Arabic(_)) => build_arabic_pronunciation_system(dialect_name),
        Some(LanguageOption::Japanese(_)) => build_japanese_pronunciation_system(dialect_name),
        None => build_pronunciation_system_for_language(dialect),
    };

    format!(
        r#"# PRONUNCIATION AGENT
Rewrite the given {dialect_name} text as it would actually be pronounced by a native speaker, for TTS rendering.

{specific_instructions}

# OUTPUT FORMAT
{JSON_OUTPUT_INSTRUCTION}
{{"pronunciation_text": "...text as pronounced in {dialect_name}..."}}"#
    )
}

fn build_pronunciation_system_for_language(dialect: Dialect) -> String {
    match dialect.language() {
        Language::Spanish => build_spanish_pronunciation_system(dialect.name()),
        Language::French => build_french_pronunciation_system(dialect.name()),
        _ => String::new(),
    }
}

fn build_spanish_pronunciation_system(dialect_name: &str) -> String {
    format!(
        r#"# TASK
Rewrite the text to reflect actual {dialect_name} pronunciation for TTS rendering.

# CRITICAL RULES
- Apply {dialect_name}-specific sound changes to the text
- Modify spelling to reflect how words are actually pronounced in this dialect
- Preserve meaning while adjusting pronunciation representation
- The output must sound like natural {dialect_name} when read by a TTS engine"#
    )
}

fn build_french_pronunciation_system(dialect_name: &str) -> String {
    format!(
        r#"# TASK
Rewrite the text to reflect actual {dialect_name} pronunciation for TTS rendering.

# CRITICAL RULES
- Apply {dialect_name}-specific sound changes to the text
- Modify spelling to reflect how words are actually pronounced in this dialect
- Preserve meaning while adjusting pronunciation representation
- The output must sound like natural {dialect_name} when read by a TTS engine"#
    )
}

fn build_arabic_pronunciation_system(dialect_name: &str) -> String {
    format!(
        r#"# TASK
Rewrite the text as it is actually pronounced in {dialect_name}, with full harakat (tashkeel) on all letters.

# CRITICAL RULES
- Replace consonants with their {dialect_name} equivalents (e.g., ق→أ in Egyptian Arabic)
- The harakat MUST reflect {dialect_name} pronunciation, NOT Modern Standard Arabic (Fus7a)
- Use dialect-specific vowel patterns (e.g., "i" sounds that would be "a" in MSA)
- Reflect dropped or altered vowels common in {dialect_name}
- Show sukun where consonant clusters occur in the dialect
- The output must sound like natural {dialect_name} when spoken aloud, not like formal Arabic"#
    )
}

fn build_japanese_pronunciation_system(dialect_name: &str) -> String {
    format!(
        r#"# TASK
Add furigana ruby tags to kanji (e.g., <ruby>漢字<rt>かんじ</rt></ruby>) so TTS can pronounce them correctly.

# CRITICAL RULES
- The furigana MUST reflect {dialect_name} pronunciation
- Use readings that match how a native {dialect_name} speaker actually pronounces each word
- Include dialect-specific readings for common words
- The pronunciation_text must sound like natural {dialect_name} when read aloud"#
    )
}

fn build_pronunciation_prompt(response_text: &str) -> String {
    format!("Write the pronunciation version of the following text:\n\n{response_text}")
}

fn try_parse_pronunciation_output(response: &str) -> Result<PronunciationAgentOutput> {
    let normalized = normalize_json_response(response);
    let parsed: RawPronunciationOutput = serde_json::from_str(&normalized)?;
    Ok(PronunciationAgentOutput {
        pronunciation_text: Some(parsed.pronunciation_text),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use dialect_coach_shared::models::{ArabicScript, JapaneseScript};

    #[test]
    fn test_pronunciation_output_empty() {
        let output = PronunciationAgentOutput::empty();
        assert!(output.pronunciation_text.is_none());
    }

    #[test]
    fn test_parse_valid_json() {
        let json = r#"{"pronunciation_text": "test annotated"}"#;
        let output = try_parse_pronunciation_output(json).unwrap();
        assert_eq!(output.pronunciation_text.unwrap(), "test annotated");
    }

    #[test]
    fn test_parse_invalid_json() {
        let json = r#"not valid json"#;
        assert!(try_parse_pronunciation_output(json).is_err());
    }

    #[test]
    fn test_system_content_arabic_naskh() {
        let opt = Some(LanguageOption::Arabic(ArabicScript::Naskh));
        let content = build_pronunciation_system_content(Dialect::ArabicLevantine, &opt);
        assert!(content.contains("harakat"));
        assert!(content.contains("Levantine Arabic"));
    }

    #[test]
    fn test_system_content_arabic_ruqa() {
        let opt = Some(LanguageOption::Arabic(ArabicScript::Ruqa));
        let content = build_pronunciation_system_content(Dialect::ArabicEgyptian, &opt);
        assert!(content.contains("harakat"));
        assert!(content.contains("Egyptian Arabic"));
    }

    #[test]
    fn test_system_content_japanese_kanji() {
        let opt = Some(LanguageOption::Japanese(JapaneseScript::Kanji));
        let content = build_pronunciation_system_content(Dialect::JapaneseKansai, &opt);
        assert!(content.contains("furigana"));
        assert!(content.contains("Kansai Japanese"));
    }

    #[test]
    fn test_prompt_includes_response_text() {
        let prompt = build_pronunciation_prompt("Hello world");
        assert!(prompt.contains("Hello world"));
    }

    #[test]
    fn test_system_content_spanish_mexican() {
        let content = build_pronunciation_system_content(Dialect::SpanishMexican, &None);
        assert!(content.contains("pronunciation"));
        assert!(content.contains("Mexican Spanish"));
    }

    #[test]
    fn test_system_content_french_quebecois() {
        let content = build_pronunciation_system_content(Dialect::FrenchQuebecois, &None);
        assert!(content.contains("pronunciation"));
        assert!(content.contains("Quebec French"));
    }

    #[test]
    fn test_pronunciation_system_for_language_spanish() {
        let result = build_pronunciation_system_for_language(Dialect::SpanishMexican);
        assert!(!result.is_empty());
    }

    #[test]
    fn test_pronunciation_system_for_language_english() {
        let result = build_pronunciation_system_for_language(Dialect::EnglishGeneralAmerican);
        assert!(result.is_empty());
    }
}
