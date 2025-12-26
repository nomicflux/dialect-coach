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

    pub past_mistakes: &'a [Mistake],
    pub past_explained: &'a [Explained],
    pub past_translated: &'a [Translated],
    pub past_exploratory: &'a [Exploratory],
    pub language_option: &'a Option<LanguageOption>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LearningAgentOutput {
    pub mistakes: Vec<(Mistake, u8)>,
    pub explained: Vec<(Explained, u8)>,
    pub translated: Vec<(Translated, u8)>,
    pub exploratory: Vec<(Exploratory, u8)>,
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

    if total_items < 100 {
        return false;
    }

    // Skip for modes that generate limited items when total >= 100
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
            mistakes: raw.mistakes.into_iter().map(|m| (m, 0)).collect(),
            explained: raw.explained.into_iter().map(|e| (e, 0)).collect(),
            translated: raw.translated.into_iter().map(|t| (t, 0)).collect(),
            exploratory: raw.exploratory.into_iter().map(|x| (x, 0)).collect(),
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

        // ErrorFinding mode: analyze how user handled intentional errors
        if params.teaching_mode == TeachingMode::ErrorFinding {
            return self.generate_error_finding_items(params).await;
        }

        if should_skip_learning_call(params) {
            tracing::info!(
                "Skipping learning agent call - {} total items (≥100)",
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

    async fn generate_error_finding_items(
        &self,
        params: &LearningAgentParams<'_>,
    ) -> (Result<LearningAgentOutput>, Vec<AgentUsage>) {
        let (result, usage) = self.call_error_finding_agent(params).await;
        match result {
            Ok(mistakes) => (
                Ok(LearningAgentOutput {
                    mistakes,
                    explained: Vec::new(),
                    translated: Vec::new(),
                    exploratory: Vec::new(),
                }),
                usage,
            ),
            Err(e) => (Err(e), usage),
        }
    }

    async fn call_error_finding_agent(
        &self,
        params: &LearningAgentParams<'_>,
    ) -> (Result<Vec<(Mistake, u8)>>, Vec<AgentUsage>) {
        let limits = calculate_learning_limits(params);
        let system_content = build_learning_system_content(params, &limits);
        let prompt = build_error_finding_prompt(params);
        let history = self.build_prefill_history();

        let request = CompletionRequest {
            preamble: &system_content,
            prompt: &prompt,
            history: &history,
            max_tokens: 512,
            temperature: 0.5,
        };

        let (result, usage) = retry_completion_call(self.agent.as_ref(), &request, 3).await;
        match result {
            Ok(response) => {
                tracing::info!("ErrorFinding raw response: {}", response);
                (parse_error_finding_output(&response), usage)
            }
            Err(e) => (Err(e.context("Failed to get error finding items")), usage),
        }
    }

    fn build_prefill_history(&self) -> Vec<RigMessage> {
        if self.agent.provider() == super::provider::ANTHROPIC_PROVIDER {
            vec![create_prefilled_assistant_message()]
        } else {
            Vec::new()
        }
    }

    fn skip_mode(mode: &TeachingMode) -> bool {
        matches!(mode, TeachingMode::Immersive | TeachingMode::Debug)
    }
}

fn parse_error_finding_output(response: &str) -> Result<Vec<(Mistake, u8)>> {
    use super::error_finding::{ErrorFindingLearningOutput, score_for_handling};

    let normalized = normalize_json_response(response);
    let output: ErrorFindingLearningOutput = serde_json::from_str(&normalized).map_err(|e| {
        tracing::warn!("Failed to parse ErrorFinding output: {}", e);
        anyhow::anyhow!("ErrorFinding parse failed: {}", e)
    })?;

    let mistakes = output
        .handled_errors
        .into_iter()
        .map(|discovered| {
            let score = score_for_handling(&discovered.handling);
            tracing::info!(
                "ErrorFinding: '{}' handled as {:?}, score {}",
                discovered.error_form,
                discovered.handling,
                score
            );
            let mistake = Mistake::new(
                discovered.error_form,
                discovered.correct_form,
                discovered.error_category,
            );
            (mistake, score)
        })
        .collect();

    Ok(mistakes)
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
        learning_mode_context(&params.teaching_mode),
        JSON_OUTPUT_INSTRUCTION,
        learning_output_format_spec(&params.teaching_mode, limits)
    )
}

fn learning_mode_context(teaching_mode: &TeachingMode) -> String {
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
        TeachingMode::StoryTeller => {
            r#"# WHAT TO LOG\n\
            - New language patterns or linguistic features user should practice from the previous assistant message\n\
            - Linguistic points that naturally fit the story context\n\
            - All points MUST be SPECIFIC LINGUSTIC FEATURES.
            "#.to_string()
        }
        TeachingMode::Immersive | TeachingMode::Debug => String::new(),
        TeachingMode::ErrorFinding => {
            "# ERROR FINDING ANALYSIS\n\
            1. Analyze the PREVIOUS ASSISTANT MESSAGE for grammatical/vocabulary/spelling errors.\n\
            2. For each error you find, check if the user EXPLICITLY addressed it in their response.\n\
            3. DEFINITION OF 'ADDRESSED' (Include ALL of these):\n\
               - Explicit Correction: User points out the error.\n\
               - Contextual Correction: User uses the correct form of the word or phrase (verb conjugation, etc).\n\
               - Repeat Error: User repeats the EXACT error. (This counts as addressed, score 0).\n\
               - Different Error: User tries to use the word but makes a DIFFERENT error. (This counts as addressed, score 5).\n\
            4. IGNORE errors that the user COMPLETELY IGNORED. If the user continued the conversation without using the word/concept at all, it is IGNORED.\n\
            5. Classify handling for ADDRESSED errors only:\n\
               - 'corrected': User fixed the error (Explicit or Contextual) -> Score 10\n\
               - 'same_error': User repeated the error exactly -> Score 0\n\
               - 'different_error': User tried to use the word but failed differently -> Score 5\n\
            6. STRICTLY FILTER OUT IGNORED ERRORS. Do not include them in the output.\n\n".to_string()
        }
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
        TeachingMode::ErrorFinding => r#"{
  "handled_errors": [
    {
      "error_form": "<what assistant said wrong>",
      "correct_form": "<correct version>",
      "handling": "<corrected|same_error|different_error>",
      "error_category": {"type": "<category>", "context": "<brief>"}  
    }
  ]
}
Categories: spelling_error, vocabulary_error, grammar_error, dialect_usage_error, other
- INCLUDE ONLY errors where the user made an explicit attempt to correct or use the word.
- EXCLUDE errors the user ignored."#
            .to_string(),
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
        TeachingMode::Corrective => (
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
        PREVIOUS LEARNING ITEMS:\n{}\n",
        user_section,
        asst_section,
        build_existing_items_section(params)
    )
}

fn build_error_finding_prompt(params: &LearningAgentParams<'_>) -> String {
    format!(
        "PREVIOUS ASSISTANT MESSAGE:\n{}\n\n\
        LATEST USER MESSAGE:\n{}\n\n\
        Analyze the assistant's message for errors. REPORT ONLY ERRORS THAT THE USER EXPLICITLY ADDRESSED (corrected or repeated).",
        params.assistant_response, params.user_message
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
                let preamble_builder = |preamble: &str, failed: &str, error: &str| {
                    build_retry_learning_preamble(preamble, failed, error)
                };
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
            past_mistakes: &mistakes,
            past_explained: &explained,
            past_translated: &translated,
            past_exploratory: &exploratory,
            language_option: &None,
        };
        let prompt = build_learning_prompt(&params);
        assert!(prompt.contains("LATEST USER MESSAGE"));
        assert!(prompt.contains("¿Cómo estás?"));

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
            past_mistakes: &[],
            past_explained: &[],
            past_translated: &[],
            past_exploratory: &[],
            language_option: &None,
        };
        assert!(!should_skip_learning_call(&params));
    }

    #[test]
    fn test_should_skip_learning_call_at_100_corrective() {
        let mut explained = Vec::new();
        for i in 0..100 {
            explained.push(Explained::new(format!("test{}", i), "test".to_string()));
        }
        let params = LearningAgentParams {
            user_message: "test",
            assistant_response: "test",
            dialect: Dialect::SpanishMexican,
            formality: Formality::Informal,
            teaching_mode: TeachingMode::Corrective,
            past_mistakes: &[],
            past_explained: &explained,
            past_translated: &[],
            past_exploratory: &[],
            language_option: &None,
        };
        assert!(should_skip_learning_call(&params));
    }
}
