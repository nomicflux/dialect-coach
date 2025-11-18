use anyhow::Result;
use dialect_coach_shared::{
    AgentUsage, Dialect, Explained, Exploratory, Formality, Mistake, TeachingMode, Translated,
};
use rig::completion::Message as RigMessage;
use serde::Deserialize;
use std::sync::Arc;

use super::provider::{CompletionAgent, CompletionRequest};
use super::retry::{RetryContext, build_retry_learning_preamble, retry_completion_call};
use super::util::{
    JSON_OUTPUT_INSTRUCTION, create_prefilled_assistant_message, format_learning_items_context,
    normalize_json_response,
};

#[derive(Debug, Clone)]
pub struct LearningAgentParams<'a> {
    pub user_message: &'a str,
    pub assistant_response: &'a str,
    pub dialect: Dialect,
    pub formality: Formality,
    pub teaching_mode: TeachingMode,
    pub learning_goals: &'a [String],
    pub past_mistakes: &'a [Mistake],
    pub past_explained: &'a [Explained],
    pub past_translated: &'a [Translated],
    pub past_exploratory: &'a [Exploratory],
}

#[derive(Debug, Clone, PartialEq)]
pub struct LearningAgentOutput {
    pub mistakes: Vec<Mistake>,
    pub explained: Vec<Explained>,
    pub translated: Vec<Translated>,
    pub exploratory: Vec<Exploratory>,
}

impl LearningAgentOutput {
    pub fn empty() -> Self {
        Self {
            mistakes: Vec::new(),
            explained: Vec::new(),
            translated: Vec::new(),
            exploratory: Vec::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct RawLearningAgentOutput {
    #[serde(default)]
    mistakes: Vec<Mistake>,
    #[serde(default)]
    explained: Vec<Explained>,
    #[serde(default)]
    translated: Vec<Translated>,
    #[serde(default)]
    exploratory: Vec<Exploratory>,
}

impl From<RawLearningAgentOutput> for LearningAgentOutput {
    fn from(raw: RawLearningAgentOutput) -> Self {
        Self {
            mistakes: raw.mistakes,
            explained: raw.explained,
            translated: raw.translated,
            exploratory: raw.exploratory,
        }
    }
}

pub struct LearningAgent {
    agent: Arc<dyn CompletionAgent>,
}

impl LearningAgent {
    pub fn new(agent: Arc<dyn CompletionAgent>) -> Self {
        Self { agent }
    }

    pub async fn generate_learning_items(
        &self,
        params: &LearningAgentParams<'_>,
    ) -> (Result<LearningAgentOutput>, Vec<AgentUsage>) {
        if Self::skip_mode(&params.teaching_mode) {
            return (Ok(LearningAgentOutput::empty()), Vec::new());
        }

        let system_content = build_learning_system_content(params);
        tracing::debug!(
            "Learning system content sent to Claude:\n{}",
            system_content
        );
        let prompt = build_learning_prompt(params);
        tracing::debug!("Learning prompt sent to Claude:\n{}", prompt);

        let mut history_with_prefill = Vec::new();
        if self.agent.provider() == super::provider::ANTHROPIC_PROVIDER {
            history_with_prefill.push(create_prefilled_assistant_message());
        }

        let request = CompletionRequest {
            preamble: &system_content,
            prompt: &prompt,
            history: &history_with_prefill,
            max_tokens: 512,
            temperature: 0.0,
        };

        let (result, usage) = retry_completion_call(self.agent.as_ref(), &request, 3).await;
        match result {
            Ok(response) => {
                self.handle_successful_completion(
                    response,
                    usage,
                    params,
                    &system_content,
                    &prompt,
                    &history_with_prefill,
                )
                .await
            }
            Err(e) => (
                Err(e.context(format!(
                    "Failed to get learning items from provider {} model {}",
                    self.agent.provider(),
                    self.agent.model()
                ))),
                usage,
            ),
        }
    }

    fn skip_mode(mode: &TeachingMode) -> bool {
        matches!(mode, TeachingMode::Immersive | TeachingMode::Debug)
    }
}

fn build_learning_system_content(params: &LearningAgentParams<'_>) -> String {
    format!(
        "# LEARNING AGENT ROLE\n\
        You evaluate the user's latest message and the assistant's reply for new learning items.\n\n\
        # EVALUATION CONTEXT\n\
        You are a native speaker of {} with {} formality.\n\n\
        {}\
        # RULES\n\
        - Log learning items only, not conversational responses\n\
        - Do not duplicate previously logged items\n\
        - No meta-commentary or extra text\n\n\
        # OUTPUT FORMAT\n\
        {}\n\
        {}\n",
        params.dialect.name(),
        params.formality.name(),
        learning_mode_context(&params.teaching_mode),
        JSON_OUTPUT_INSTRUCTION,
        learning_output_format_spec(&params.teaching_mode)
    )
}

fn learning_mode_context(teaching_mode: &TeachingMode) -> &'static str {
    match teaching_mode {
        TeachingMode::Corrective => {
            "# COMMUNICATION ERROR DETECTION\n\
            Default: The user's text is CORRECT. Most messages have zero errors.\n\
            Only flag errors in the previous user message that a native speaker would actually notice and reject.\n\n\
            BEFORE flagging anything, verify:\n\
            1. Is this actually wrong in THIS dialect? (not just different from standard/formal)\n\
            2. Would natives notice or care in casual chat?\n\
            3. Is this only less common instead of actually wrong?\n\
            4. Is your correction meaningfully different, not just orthographic variation?\n\
            5. Is it in the previous user message (not assistent message, not dialect examples, not further back in the conversation)?\n\
            If ANY answer is NO → do not flag. When unsure → do not flag.\n\n\
            NOT errors (never flag):\n\
            - Arabic without short vowel marks (tashkeel: ◌َ ◌ُ ◌ِ ◌ّ ◌ْ) - standard in chat\n\
            - Spanish without ¿ or ¡ - common in casual typing\n\
            - In general, missing or incorrect punctuation marks are not errors (common to leave out in casual typing)\n\
            - Capitalization differences - casual chat norm\n\
            - Valid dialectal spellings and forms\n\n\
            Returning {\"mistakes\": []} is normal and expected for most messages.\n\n"
        }
        TeachingMode::Explanatory => {
            "# WHAT TO LOG\n\
            - New vocabulary, idioms, or cultural context introduced in the previous assistant message\n\
            - Only noteworthy items worth remembering\n\n"
        }
        TeachingMode::Interleaved => {
            "# WHAT TO LOG\n\
            - Words/phrases user needed translated from source language to target dialect in the previous user message\n\
            - Focus on translations, not errors\n\n"
        }
        TeachingMode::StoryTeller => {
            "# WHAT TO LOG\n\
            - New language patterns or features user should practice from the previous assistant message\n\
            - Points that naturally fit the story context\n\n"
        }
        TeachingMode::Immersive | TeachingMode::Debug => "",
    }
}

fn learning_output_format_spec(teaching_mode: &TeachingMode) -> &'static str {
    match teaching_mode {
        TeachingMode::Corrective => {
            r#"{
  "mistakes": [{
    "specific_mistake": "<exact erroneous token/phrase>",
    "correction": "<replacement>",
    "mistake_category": {"type": "<category>", "context": "<≤8 words or empty>"}
  }]
}
Categories: spelling_error, vocabulary_error, grammar_error, dialect_usage_error, other
- Prefer single-token fixes; multi-token only for phrase-level errors
- Context: brief clarification or empty string
- Maximum 3 entries
- Return {"mistakes": []} if no communication errors"#
        }
        TeachingMode::Explanatory => {
            r#"{
  "explained": [{"new_phrase": "<word/phrase>", "explanation": "<brief usage note>"}]
}
- Keep to 1-2 essential items
- Return {"explained": []} if nothing new worth cataloging"#
        }
        TeachingMode::Interleaved => {
            r#"{
  "translated": [{"translated_word": "<source word>", "translated_to": "<dialect translation>"}]
}
- Return {"translated": []} when nothing required translating"#
        }
        TeachingMode::StoryTeller => {
            r#"{
  "exploratory": [{"point_to_try": "<language feature>", "instructions_for_use": "<how to use>"}]
}
- Keep to 1-2 brief points
- Return {"exploratory": []} if nothing new introduced"#
        }
        TeachingMode::Immersive | TeachingMode::Debug => {
            r#"{}
No learning items for this mode."#
        }
    }
}

fn format_learning_goals(goals: &[String]) -> String {
    if goals.is_empty() {
        "None.".to_string()
    } else {
        goals
            .iter()
            .enumerate()
            .map(|(i, goal)| format!("{}. {}", i + 1, goal))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn build_existing_items_section(params: &LearningAgentParams<'_>) -> String {
    let summary = format_learning_items_context(
        params.past_mistakes,
        params.past_explained,
        params.past_translated,
        params.past_exploratory,
    );
    if summary.is_empty() {
        "No previous learning items recorded.".to_string()
    } else {
        summary
    }
}

fn build_learning_prompt(params: &LearningAgentParams<'_>) -> String {
    format!(
        "LATEST USER MESSAGE:\n{}\n\n\
        ASSISTANT RESPONSE:\n{}\n\n\
        LEARNING GOALS:\n{}\n\n\
        PREVIOUS LEARNING ITEMS:\n{}\n",
        params.user_message,
        params.assistant_response,
        format_learning_goals(params.learning_goals),
        build_existing_items_section(params)
    )
}

fn log_learning_success(output: &LearningAgentOutput) {
    tracing::info!(
        "Generated learning items (mistakes: {}, explained: {}, translated: {}, exploratory: {})",
        output.mistakes.len(),
        output.explained.len(),
        output.translated.len(),
        output.exploratory.len()
    );
}

fn try_parse_learning_output(response: &str) -> Result<LearningAgentOutput> {
    let normalized = normalize_json_response(response);
    let parsed: RawLearningAgentOutput = serde_json::from_str(&normalized).map_err(|e| {
        tracing::warn!(
            "Learning agent JSON parse error: {}. First 200 chars: {}",
            e,
            &response.chars().take(200).collect::<String>()
        );
        anyhow::anyhow!("Learning agent JSON parse failed: {}", e)
    })?;
    Ok(parsed.into())
}

impl LearningAgent {
    async fn handle_successful_completion(
        &self,
        response: String,
        usage: Vec<AgentUsage>,
        params: &LearningAgentParams<'_>,
        system_content: &str,
        prompt: &str,
        history_with_prefill: &[RigMessage],
    ) -> (Result<LearningAgentOutput>, Vec<AgentUsage>) {
        match self
            .handle_learning_response(
                response,
                usage.clone(),
                params,
                system_content,
                prompt,
                history_with_prefill,
            )
            .await
        {
            Ok((output, final_usage)) => (Ok(output), final_usage),
            Err(e) => (Err(e), usage),
        }
    }

    async fn handle_learning_response(
        &self,
        response: String,
        initial_usage: Vec<AgentUsage>,
        _params: &LearningAgentParams<'_>,
        system_content: &str,
        prompt: &str,
        history_with_prefill: &[RigMessage],
    ) -> Result<(LearningAgentOutput, Vec<AgentUsage>)> {
        match try_parse_learning_output(&response) {
            Ok(output) => {
                log_learning_success(&output);
                Ok((output, initial_usage))
            }
            Err(_) => {
                let retry_ctx = RetryContext {
                    agent: self.agent.clone(),
                };
                let parse_fn = |resp: &str| try_parse_learning_output(resp);
                let log_success = |parsed: &LearningAgentOutput| log_learning_success(parsed);
                let preamble_builder =
                    |preamble: &str, failed: &str| build_retry_learning_preamble(preamble, failed);
                let prompt_params = super::retry::RetryPromptParams {
                    original_preamble: system_content,
                    failed_response: &response,
                    prompt,
                    preamble_builder: &preamble_builder,
                };
                let config = super::util::GenerationConfig {
                    max_tokens: 512,
                    temperature: 0.0,
                };
                match retry_ctx
                    .retry_with_error_feedback_tracked(
                        &prompt_params,
                        history_with_prefill,
                        &config,
                        &parse_fn,
                        &log_success,
                    )
                    .await
                {
                    Ok((output, retry_usage)) => {
                        let mut all_usage = initial_usage;
                        all_usage.extend(retry_usage);
                        Ok((output, all_usage))
                    }
                    Err(e) => Err(e),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dialect_coach_shared::{
        Dialect, Explained, Exploratory, Formality, Mistake, MistakeCategory, TeachingMode,
        Translated,
    };

    #[test]
    fn test_build_learning_system_content_contains_directives() {
        let goals = vec!["Goal 1".to_string()];
        let mistakes = vec![Mistake::new(
            "hablar".to_string(),
            "habla".to_string(),
            MistakeCategory::SpellingError {
                context: "habla".to_string(),
            },
        )];
        let explained = vec![Explained::new("órale".to_string(), "Slang".to_string())];
        let translated = vec![Translated::new("house".to_string(), "casa".to_string())];
        let exploratory = vec![Exploratory::new(
            "Prueba pretérito".to_string(),
            "Cuenta algo breve".to_string(),
        )];

        let params = LearningAgentParams {
            user_message: "¿Cómo estás?",
            assistant_response: "Todo bien, ¿y tú?",
            dialect: Dialect::SpanishMexican,
            formality: Formality::Informal,
            teaching_mode: TeachingMode::Corrective,
            learning_goals: &goals,
            past_mistakes: &mistakes,
            past_explained: &explained,
            past_translated: &translated,
            past_exploratory: &exploratory,
        };
        let content = build_learning_system_content(&params);
        assert!(content.contains("LEARNING AGENT ROLE"));
        assert!(content.contains(JSON_OUTPUT_INSTRUCTION));
        assert!(content.contains("Log learning items only"));
    }

    #[test]
    fn test_build_learning_prompt_includes_sections() {
        let goals = vec!["Goal 1".to_string()];
        let mistakes = vec![Mistake::new(
            "hablar".to_string(),
            "habla".to_string(),
            MistakeCategory::SpellingError {
                context: "habla".to_string(),
            },
        )];
        let explained = vec![Explained::new("órale".to_string(), "Slang".to_string())];
        let translated = vec![Translated::new("house".to_string(), "casa".to_string())];
        let exploratory = vec![Exploratory::new(
            "Prueba pretérito".to_string(),
            "Cuenta algo breve".to_string(),
        )];

        let params = LearningAgentParams {
            user_message: "¿Cómo estás?",
            assistant_response: "Todo bien, ¿y tú?",
            dialect: Dialect::SpanishMexican,
            formality: Formality::Informal,
            teaching_mode: TeachingMode::Corrective,
            learning_goals: &goals,
            past_mistakes: &mistakes,
            past_explained: &explained,
            past_translated: &translated,
            past_exploratory: &exploratory,
        };
        let prompt = build_learning_prompt(&params);
        assert!(prompt.contains("LATEST USER MESSAGE"));
        assert!(prompt.contains("¿Cómo estás?"));
        assert!(prompt.contains("ASSISTANT RESPONSE"));
        assert!(prompt.contains("Goal 1"));
        assert!(prompt.contains("hablar -> habla"));
    }

    #[test]
    fn test_try_parse_learning_output() {
        let json = r#"{
            "mistakes": [{
                "specific_mistake": "hablar",
                "correction": "habla",
                "mistake_category": {"type": "spelling_error", "context": "habla"}
            }],
            "explained": [{"new_phrase": "órale", "explanation": "Slang"}],
            "translated": [{"translated_word": "house", "translated_to": "casa"}],
            "exploratory": [{"point_to_try": "Prueba pretérito", "instructions_for_use": "Cuenta algo breve"}]
        }"#;
        let output = try_parse_learning_output(json).unwrap();
        assert_eq!(output.mistakes.len(), 1);
        assert_eq!(output.explained.len(), 1);
        assert_eq!(output.translated.len(), 1);
        assert_eq!(output.exploratory.len(), 1);
    }

    #[test]
    fn test_skip_mode() {
        assert!(LearningAgent::skip_mode(&TeachingMode::Immersive));
        assert!(LearningAgent::skip_mode(&TeachingMode::Debug));
        assert!(!LearningAgent::skip_mode(&TeachingMode::Corrective));
    }
}
