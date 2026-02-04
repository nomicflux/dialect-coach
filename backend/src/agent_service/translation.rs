use anyhow::Result;
use dialect_coach_shared::{AgentUsage, Dialect, Formality, LanguageOption, Translated};
use serde::Deserialize;
use std::sync::Arc;

use super::language_instructions::build_language_instruction;
use super::provider::{ANTHROPIC_PROVIDER, CompletionAgent, CompletionRequest};
use super::retry::retry_completion_call;
use super::util::{
    JSON_OUTPUT_INSTRUCTION, create_prefilled_assistant_message, normalize_json_response,
};

#[derive(Debug, Clone)]
pub struct TranslationAgentParams<'a> {
    pub user_message: &'a str,
    pub dialect: Dialect,
    pub formality: Formality,
    pub past_translated: &'a [Translated],
    pub language_option: &'a Option<LanguageOption>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TranslationAgentOutput {
    pub translated: Vec<(Translated, u8)>,
}

impl TranslationAgentOutput {
    pub fn empty() -> Self {
        Self {
            translated: Vec::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct RawTranslationOutput {
    #[serde(default)]
    translated: Vec<Translated>,
}

pub struct TranslationAgent {
    agent: Arc<dyn CompletionAgent>,
}

impl TranslationAgent {
    pub fn new(agent: Arc<dyn CompletionAgent>) -> Self {
        Self { agent }
    }

    pub async fn generate_translations(
        &self,
        params: &TranslationAgentParams<'_>,
    ) -> (Result<TranslationAgentOutput>, Vec<AgentUsage>) {
        tracing::debug!(
            "TranslationAgent: generate_translations called for dialect: {}",
            params.dialect
        );

        let system_content = build_translation_system_content(params);
        tracing::debug!("Translation Agent System Content:\n{}", system_content);
        let prompt = build_translation_prompt(params);
        tracing::debug!("Translation Agent Prompt:\n{}", prompt);

        let mut history = Vec::new();
        if self.agent.provider() == ANTHROPIC_PROVIDER {
            history.push(create_prefilled_assistant_message());
        }

        let request = CompletionRequest {
            preamble: &system_content,
            prompt: &prompt,
            history: &history,
            max_tokens: 256,
            temperature: 0.0,
        };

        let (result, usage) = retry_completion_call(self.agent.as_ref(), &request, 3).await;
        match result {
            Ok(response) => {
                tracing::debug!("Translation Agent Raw Response:\n{}", response);
                match try_parse_translation_output(&response) {
                    Ok(output) => {
                        log_translation_success(&output);
                        (Ok(output), usage)
                    }
                    Err(e) => (Err(e), usage),
                }
            }
            Err(e) => (Err(e), usage),
        }
    }
}

fn build_translation_system_content(params: &TranslationAgentParams<'_>) -> String {
    let language_instr = build_language_instruction(params.language_option);
    let lang_section = if !language_instr.is_empty() {
        format!("\n\nLANGUAGE INSTRUCTION: {}\n", language_instr)
    } else {
        String::new()
    };

    format!(
        r#"# TRANSLATION AGENT ROLE
You identify English words/phrases in the user's message that should be translated to the target dialect.

# CONTEXT
Target dialect: {} at {} formality.{}

# CRITICAL: FOREIGN IMPORTS WARNING
Do NOT translate English words that are commonly used as loanwords in this dialect.
Examples of words to KEEP in English:
- Brand names and proper nouns
- Words that have been adopted into the dialect with no native equivalent
- Words the dialect commonly uses in English form

Only translate when the user is clearly code-switching back to English, NOT using established loanwords.

# RULES
- Log translations only, not conversational responses
- Do not duplicate previously logged translations
- Translate the full phrases that the user requests

# OUTPUT FORMAT
{}
{{
  "translated": [{{"translated_word": "<English word>", "translated_to": "<dialect translation>"}}]
}}
Return {{"translated": []}} when nothing requires translation."#,
        params.dialect.name(),
        params.formality.name(),
        lang_section,
        JSON_OUTPUT_INSTRUCTION,
    )
}

fn build_translation_prompt(params: &TranslationAgentParams<'_>) -> String {
    let past_section = if params.past_translated.is_empty() {
        "No previous translations.".to_string()
    } else {
        params
            .past_translated
            .iter()
            .map(|t| format!("{} -> {}", t.translated_word, t.translated_to))
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        "USER MESSAGE:\n{}\n\nPREVIOUS TRANSLATIONS:\n{}",
        params.user_message, past_section
    )
}

fn try_parse_translation_output(response: &str) -> Result<TranslationAgentOutput> {
    let normalized = normalize_json_response(response);
    let parsed: RawTranslationOutput = serde_json::from_str(&normalized)?;
    Ok(TranslationAgentOutput {
        translated: parsed.translated.into_iter().map(|t| (t, 0)).collect(),
    })
}

fn log_translation_success(output: &TranslationAgentOutput) {
    tracing::info!("Generated {} translation items", output.translated.len());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translation_output_empty() {
        let output = TranslationAgentOutput::empty();
        assert!(output.translated.is_empty());
    }
}
