use anyhow::Result;
use dialect_coach_shared::{
    AgentUsage, Dialect, Explained, Exploratory, Formality, LanguageOption, Mistake, TeachingMode,
    Translated,
};
use rig::completion::Message as RigMessage;
use serde::Deserialize;
use std::sync::Arc;

use super::language_instructions::build_language_instruction;
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
    pub learning_goals: &'a [dialect_coach_shared::LearningGoal],
    pub past_mistakes: &'a [Mistake],
    pub past_explained: &'a [Explained],
    pub past_translated: &'a [Translated],
    pub past_exploratory: &'a [Exploratory],
    pub language_option: &'a Option<LanguageOption>,
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

/// Calculate maximum items to request for each learning type
struct LearningItemLimits {
    max_mistakes: u8,
    max_explained: u8,
    max_exploratory: u8,
}

fn calculate_learning_limits(params: &LearningAgentParams<'_>) -> LearningItemLimits {
    let total_items = params.past_mistakes.len()
        + params.past_explained.len()
        + params.past_translated.len()
        + params.past_exploratory.len();

    let calc_limit = |type_count: usize| -> u8 {
        if type_count == 0 && total_items < 5 {
            2
        } else {
            1
        }
    };

    LearningItemLimits {
        max_mistakes: calc_limit(params.past_mistakes.len()),
        max_explained: calc_limit(params.past_explained.len()),
        max_exploratory: calc_limit(params.past_exploratory.len()),
    }
}

fn should_skip_learning_call(params: &LearningAgentParams<'_>) -> bool {
    let total_items = params.past_mistakes.len()
        + params.past_explained.len()
        + params.past_translated.len()
        + params.past_exploratory.len();

    if total_items < 10 {
        return false;
    }

    // Skip for modes that generate limited items
    // Continue for Interleaved (user-controlled)
    matches!(
        params.teaching_mode,
        TeachingMode::Corrective | TeachingMode::Explanatory | TeachingMode::StoryTeller
    )
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
        tracing::info!(
            "Learning agent called: teaching_mode={:?}, total_items={}",
            params.teaching_mode,
            params.past_mistakes.len()
                + params.past_explained.len()
                + params.past_translated.len()
                + params.past_exploratory.len()
        );

        if Self::skip_mode(&params.teaching_mode) {
            return (Ok(LearningAgentOutput::empty()), Vec::new());
        }

        if should_skip_learning_call(params) {
            tracing::info!(
                "Skipping learning agent call - {} total items (≥10)",
                params.past_mistakes.len()
                    + params.past_explained.len()
                    + params.past_translated.len()
                    + params.past_exploratory.len()
            );
            return (Ok(LearningAgentOutput::empty()), Vec::new());
        }

        let limits = calculate_learning_limits(params);
        let system_content = build_learning_system_content(params, &limits);
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

fn build_learning_system_content(
    params: &LearningAgentParams<'_>,
    limits: &LearningItemLimits,
) -> String {
    let language_instr = build_language_instruction(params.language_option);
    let lang_section = if !language_instr.is_empty() {
        format!("\n\nLANGUAGE INSTRUCTION: {}\n", language_instr)
    } else {
        String::new()
    };
    format!(
        "# LEARNING AGENT ROLE\n\
        You evaluate the user's latest message and the assistant's reply for new learning items.\n\n\
        # EVALUATION CONTEXT\n\
        You are a native speaker of {} with {} formality.{}\n\
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
        lang_section,
        learning_mode_context(&params.teaching_mode, &params.dialect, &params.formality),
        JSON_OUTPUT_INSTRUCTION,
        learning_output_format_spec(&params.teaching_mode, limits)
    )
}

fn learning_mode_context(
    teaching_mode: &TeachingMode,
    dialect: &Dialect,
    formality: &Formality,
) -> String {
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
            Returning {\"mistakes\": []} is normal and expected for most messages.\n\n".to_string()
        }
        TeachingMode::Explanatory => {
            "# WHAT TO LOG\n\
            - New vocabulary, idioms, or cultural context introduced in the previous assistanjt message\n\
            - Only noteworthy items worth remembering\n\n".to_string()
        }
        TeachingMode::Interleaved => {
            format!(
                "# WHAT TO LOG\n\
                - English words/phrases from the user's message that needed translation\n\
                - Translations must be in {} at {} formality\n\n",
                dialect.name(),
                formality.name()
            )
        }
        TeachingMode::StoryTeller => {
            r#"# WHAT TO LOG\n\
            - New language patterns or linguistic features user should practice from the previous assistant message\n\
            - Linguistic points that naturally fit the story context\n\
            - All points MUST be SPECIFIC LINGUSTIC FEATURES.
            "#.to_string()
        }
        TeachingMode::Immersive | TeachingMode::Debug => String::new(),
    }
}

fn learning_output_format_spec(
    teaching_mode: &TeachingMode,
    limits: &LearningItemLimits,
) -> String {
    match teaching_mode {
        TeachingMode::Corrective => {
            format!(
                r#"{{
  "mistakes": [{{
    "specific_mistake": "<exact erroneous token/phrase>",
    "correction": "<replacement>",
    "mistake_category": {{"type": "<category>", "context": "<≤8 words or empty>"}}
  }}]
}}
Categories: spelling_error, vocabulary_error, grammar_error, dialect_usage_error, other
- Prefer single-token fixes; multi-token only for phrase-level errors
- Context: brief clarification or empty string
- Maximum {} item(s)
- Return {{"mistakes": []}} if no communication errors"#,
                limits.max_mistakes
            )
        }
        TeachingMode::Explanatory => {
            format!(
                r#"{{
  "explained": [{{"new_phrase": "<word/phrase>", "explanation": "<brief usage note>"}}]
}}
- Maximum {} item(s)
- Return {{"explained": []}} if nothing new worth cataloging"#,
                limits.max_explained
            )
        }
        TeachingMode::Interleaved => r#"{
  "translated": [{"translated_word": "<source word>", "translated_to": "<dialect translation>"}]
}
- Return {"translated": []} when nothing required translating"#
            .to_string(),
        TeachingMode::StoryTeller => {
            format!(
                r#"{{
  "exploratory": [{{"point_to_try": "<specific linguistic feature>", "instructions_for_use": "<how to use>"}}]
}}
- Maximum {} item(s)
- Return {{"exploratory": []}} if nothing new introduced"#,
                limits.max_exploratory
            )
        }
        TeachingMode::Immersive | TeachingMode::Debug => r#"{}
No learning items for this mode."#
            .to_string(),
    }
}

fn format_learning_goals(goals: &[dialect_coach_shared::LearningGoal]) -> String {
    if goals.is_empty() {
        "None.".to_string()
    } else {
        goals
            .iter()
            .enumerate()
            .map(|(i, learning_goal)| format!("{}. {}", i + 1, learning_goal.goal))
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
    let (user_section, asst_section) = match params.teaching_mode {
        TeachingMode::Corrective | TeachingMode::Interleaved => (
            format!("LATEST USER MESSAGE:\n{}\n\n", params.user_message),
            String::new(),
        ),
        TeachingMode::Explanatory | TeachingMode::StoryTeller => (
            String::new(),
            format!("ASSISTANT RESPONSE:\n{}\n\n", params.assistant_response),
        ),
        _ => (String::new(), String::new()),
    };

    format!(
        "{}{}\
        LEARNING GOALS:\n{}\n\n\
        PREVIOUS LEARNING ITEMS:\n{}\n",
        user_section,
        asst_section,
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
            Err(e) => {
                let retry_ctx = RetryContext {
                    agent: self.agent.clone(),
                };
                let parse_fn = |resp: &str| try_parse_learning_output(resp);
                let log_success = |parsed: &LearningAgentOutput| log_learning_success(parsed);
                let preamble_builder =
                    |preamble: &str, failed: &str, error: &str| build_retry_learning_preamble(preamble, failed, error);
                let initial_error = format!("{}", e);
                let prompt_params = super::retry::RetryPromptParams {
                    original_preamble: system_content,
                    failed_response: &response,
                    error_message: &initial_error,
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
        let goals = vec![dialect_coach_shared::LearningGoal {
            goal: "Goal 1".to_string(),
            dialect: Dialect::SpanishMexican,
        }];
        let mistakes = vec![Mistake::new(
            "hablar".to_string(),
            "habla".to_string(),
            MistakeCategory::SpellingError {
                context: "habla".to_string(),
            },
        )];
        let explained = vec![Explained::new("órale".to_string(), "Slang".to_string())];
        let translated = vec![Translated::new(
            "house".to_string(),
            "casa".to_string(),
            None,
        )];
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
            language_option: &None,
        };
        let limits = calculate_learning_limits(&params);
        let content = build_learning_system_content(&params, &limits);
        assert!(content.contains("LEARNING AGENT ROLE"));
        assert!(content.contains(JSON_OUTPUT_INSTRUCTION));
        assert!(content.contains("Log learning items only"));
    }

    #[test]
    fn test_build_learning_prompt_includes_sections() {
        let goals = vec![dialect_coach_shared::LearningGoal {
            goal: "Goal 1".to_string(),
            dialect: Dialect::SpanishMexican,
        }];
        let mistakes = vec![Mistake::new(
            "hablar".to_string(),
            "habla".to_string(),
            MistakeCategory::SpellingError {
                context: "habla".to_string(),
            },
        )];
        let explained = vec![Explained::new("órale".to_string(), "Slang".to_string())];
        let translated = vec![Translated::new(
            "house".to_string(),
            "casa".to_string(),
            None,
        )];
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
            language_option: &None,
        };
        let prompt = build_learning_prompt(&params);
        assert!(prompt.contains("LATEST USER MESSAGE"));
        assert!(prompt.contains("¿Cómo estás?"));
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

    #[test]
    fn test_calculate_learning_limits_empty_under_5() {
        let params = LearningAgentParams {
            user_message: "test",
            assistant_response: "test",
            dialect: Dialect::SpanishMexican,
            formality: Formality::Informal,
            teaching_mode: TeachingMode::Explanatory,
            learning_goals: &[],
            past_mistakes: &[],
            past_explained: &[],
            past_translated: &[],
            past_exploratory: &[],
            language_option: &None,
        };
        let limits = calculate_learning_limits(&params);
        assert_eq!(limits.max_explained, 2);
        assert_eq!(limits.max_mistakes, 2);
        assert_eq!(limits.max_exploratory, 2);
    }

    #[test]
    fn test_calculate_learning_limits_has_type_under_5() {
        let explained = vec![Explained::new("test".to_string(), "test".to_string())];
        let params = LearningAgentParams {
            user_message: "test",
            assistant_response: "test",
            dialect: Dialect::SpanishMexican,
            formality: Formality::Informal,
            teaching_mode: TeachingMode::Explanatory,
            learning_goals: &[],
            past_mistakes: &[],
            past_explained: &explained,
            past_translated: &[],
            past_exploratory: &[],
            language_option: &None,
        };
        let limits = calculate_learning_limits(&params);
        assert_eq!(limits.max_explained, 1); // Has items
        assert_eq!(limits.max_mistakes, 2); // No items, total < 5
    }

    #[test]
    fn test_should_skip_learning_call_under_10() {
        let params = LearningAgentParams {
            user_message: "test",
            assistant_response: "test",
            dialect: Dialect::SpanishMexican,
            formality: Formality::Informal,
            teaching_mode: TeachingMode::Corrective,
            learning_goals: &[],
            past_mistakes: &[],
            past_explained: &[],
            past_translated: &[],
            past_exploratory: &[],
            language_option: &None,
        };
        assert!(!should_skip_learning_call(&params));
    }

    #[test]
    fn test_should_skip_learning_call_at_10_corrective() {
        let mut explained = Vec::new();
        for i in 0..10 {
            explained.push(Explained::new(format!("test{}", i), "test".to_string()));
        }
        let params = LearningAgentParams {
            user_message: "test",
            assistant_response: "test",
            dialect: Dialect::SpanishMexican,
            formality: Formality::Informal,
            teaching_mode: TeachingMode::Corrective,
            learning_goals: &[],
            past_mistakes: &[],
            past_explained: &explained,
            past_translated: &[],
            past_exploratory: &[],
            language_option: &None,
        };
        assert!(should_skip_learning_call(&params));
    }

    #[test]
    fn test_should_not_skip_learning_call_at_10_interleaved() {
        let mut explained = Vec::new();
        for i in 0..10 {
            explained.push(Explained::new(format!("test{}", i), "test".to_string()));
        }
        let params = LearningAgentParams {
            user_message: "test",
            assistant_response: "test",
            dialect: Dialect::SpanishMexican,
            formality: Formality::Informal,
            teaching_mode: TeachingMode::Interleaved,
            learning_goals: &[],
            past_mistakes: &[],
            past_explained: &explained,
            past_translated: &[],
            past_exploratory: &[],
            language_option: &None,
        };
        assert!(!should_skip_learning_call(&params)); // Interleaved continues
    }
}
