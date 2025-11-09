use anyhow::{Context, Result};
use dialect_coach_shared::{
    AgentUsage, Dialect, Explained, Exploratory, Formality, Mistake, TeachingMode, Translated,
};
use rig::completion::{Completion, Message as RigMessage};
use serde::Deserialize;

use super::retry::{
    RetryContext, build_retry_learning_preamble, estimate_input_tokens, retry_completion_call,
};
use super::util::{
    create_prefilled_assistant_message, format_learning_items_context,
    learning_output_format_spec, learning_teaching_desc, normalize_json_response,
    tokens_per_mode, JSON_OUTPUT_INSTRUCTION,
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
    client: rig::providers::anthropic::Client,
    model_name: String,
}

impl LearningAgent {
    pub fn new(client: rig::providers::anthropic::Client, model_name: String) -> Self {
        Self { client, model_name }
    }

    pub async fn generate_learning_items(
        &self,
        params: &LearningAgentParams<'_>,
    ) -> (Result<LearningAgentOutput>, Vec<AgentUsage>) {
        if Self::skip_mode(&params.teaching_mode) {
            return (Ok(LearningAgentOutput::empty()), Vec::new());
        }

        let system_content = build_learning_system_content(params);
        let prompt = build_learning_prompt(params);
        let history_with_prefill = vec![create_prefilled_assistant_message()];
        let agent = self
            .client
            .agent(&self.model_name)
            .preamble(&system_content)
            .max_tokens(tokens_per_mode(&params.teaching_mode))
            .temperature(0.2)
            .build();

        let estimate_fn =
            || estimate_input_tokens(&system_content, &history_with_prefill, &prompt);
        let (result, usage) = retry_completion_call(
            || {
                let prompt_clone = prompt.clone();
                let history_clone = history_with_prefill.clone();
                async {
                    agent
                        .completion(prompt_clone, history_clone)
                        .await?
                        .send()
                        .await
                }
            },
            estimate_fn,
            3,
        )
        .await;
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
                Err(e.context("Failed to get learning items from Claude")),
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
        You evaluate the assistant's latest reply for new learning items.\n\n\
        # CRITICAL RULES\n\
        1. Focus ONLY on logging learning items, never the conversational response.\n\
        2. Do not duplicate previously logged items unless the learner repeated the same issue.\n\
        3. Prioritize items tied directly to the learning goals and the assistant's reply.\n\
        {}\n\n\
        # REQUIRED OUTPUT\n\
        {}\n\
        {}\n",
        learning_teaching_desc(&params.teaching_mode),
        JSON_OUTPUT_INSTRUCTION,
        learning_output_format_spec(&params.teaching_mode)
    )
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
        params: &LearningAgentParams<'_>,
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
                    client: &self.client,
                    model_name: &self.model_name,
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
                    max_tokens: tokens_per_mode(&params.teaching_mode),
                    temperature: 0.2,
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
        let exploratory =
            vec![Exploratory::new("Prueba pretérito".to_string(), "Cuenta algo breve".to_string())];

        let params = LearningAgentParams {
            user_message: "¿Cómo estás?",
            assistant_response: "Todo bien, ¿y tú?",
            dialect: Dialect::SpanishMexican,
            formality: Formality::Casual,
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
        assert!(content.contains("Focus ONLY on logging learning items"));
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
        let exploratory =
            vec![Exploratory::new("Prueba pretérito".to_string(), "Cuenta algo breve".to_string())];

        let params = LearningAgentParams {
            user_message: "¿Cómo estás?",
            assistant_response: "Todo bien, ¿y tú?",
            dialect: Dialect::SpanishMexican,
            formality: Formality::Casual,
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

