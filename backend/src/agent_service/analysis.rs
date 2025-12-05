use anyhow::Result;
use dialect_coach_shared::{AgentUsage, Dialect, Explained, LanguageOption, Mistake};
use rig::completion::Message as RigMessage;

use super::language_instructions::build_language_instruction;
use super::provider::CompletionRequest;
use super::retry::{RetryContext, build_retry_analysis_preamble, retry_completion_call};
use super::util::{JSON_OUTPUT_INSTRUCTION, normalize_json_response};

pub fn format_mistakes_for_analysis(mistakes: &[Mistake]) -> String {
    if mistakes.is_empty() {
        return "".to_string();
    }
    format!(
        "PAST MISTAKES TO ANALYZE:\n{}\n{}\n\n",
        mistakes
            .iter()
            .map(|m| serde_json::to_string(m).unwrap())
            .collect::<Vec<_>>()
            .join(", "),
        r#"SCORING MISTAKES (-10 to 10): -10=still occurring, 0=no usage/different error, 10=fixed. Give partial points for similar cases to the mistake. Make sure you give positive points for the "correction" being used, negative for "specific_mistake" being used."#
    )
}

pub fn format_explained_for_analysis(explained: &[Explained]) -> String {
    if explained.is_empty() {
        return "".to_string();
    }
    format!(
        "PAST EXPLAINED FEATURES TO ANALYZE:\n{}\n{}\n\n",
        explained
            .iter()
            .map(|e| serde_json::to_string(e).unwrap())
            .collect::<Vec<_>>()
            .join(", "),
        r#"SCORING EXPLAINED (0 to 10): 0=not used, 5=attempted incorrectly, 10=used correctly. Give partial points for similar cases to the explained item."#
    )
}

pub fn format_translated_for_analysis(translated: &[dialect_coach_shared::Translated]) -> String {
    if translated.is_empty() {
        return "".to_string();
    }
    format!(
        "PAST TRANSLATIONS TO ANALYZE:\n{}\n{}\n\n",
        translated
            .iter()
            .map(|t| format!(
                r#"{{"id": "{}", "translated_word": "{}", "translated_to": "{}"}}"#,
                t.id, t.translated_word, t.translated_to
            ))
            .collect::<Vec<_>>()
            .join(", "),
        r#"TRANSLATED (-10 to 10): -10=reverted to untranslated, 0=not used, 10=correctly used. Give full points for different conjugations, declensions, etc. as the translated item."#
    )
}

pub fn format_exploratory_for_analysis(
    exploratory: &[dialect_coach_shared::Exploratory],
) -> String {
    if exploratory.is_empty() {
        return "".to_string();
    }
    format!(
        "PAST EXPLORATORY POINTS TO ANALYZE:\n{}\n{}\n\n",
        exploratory
            .iter()
            .map(|e| format!(r#"{{"id": "{}", "text": "{}"}}"#, e.id, e.point_to_try))
            .collect::<Vec<_>>()
            .join(", "),
        r#"EXPLORATORY (0 to 10): 0=not used, 5=imperfect attempt, 10=correctly used. Be generous in counting whether the explored item matches."#
    )
}

pub fn analysis_agent_preamble(
    dialect: &Dialect,
    mistakes: &[Mistake],
    explained: &[Explained],
    translated: &[dialect_coach_shared::Translated],
    exploratory: &[dialect_coach_shared::Exploratory],
    language_option: &Option<LanguageOption>,
) -> String {
    let language_instr = build_language_instruction(language_option);
    let lang_section = if !language_instr.is_empty() {
        format!("\n\nLANGUAGE INSTRUCTION: {}\n", language_instr)
    } else {
        String::new()
    };
    format!(
        r#"You are analyzing a language learner's progress in {}.{}

TASK: You will receive a message from the learner. Your job is to check that message and score each learning item below based on whether and how the learner used it.

LEARNING ITEMS TO SCORE:
{}{}{}{}

HOW TO SCORE:
- Read the learner's message carefully
- For EACH item listed above, check if it appears in the message IN ANY FORM
- "Present" means: the word/concept appears in ANY conjugation, declension, or related form
- "Absent" means: completely absent - the word/concept does not appear even in alternative forms
- Be generous with scoring - if you see the item used in any form, give points
- Translated items: Look for the "translated_to" word in any conjugation/declension/form
- Mistakes: Negative if still making the error, positive if using the correction
- Explained/Exploratory: Positive if attempting to use the concept
- Only score 0 if the item is completely absent (not present in any form)

CRITICAL OUTPUT RULES:
1. Your response MUST be ONLY a JSON object - nothing else
2. ALL scores must be integers from -10 to 10
3. NO explanations, NO text before or after the JSON
4. You MUST score ALL items that are listed in the categories above
5. Empty categories (with no items listed) get {{}}, but if items ARE listed, you MUST check and score them

{}

Format: {{"mistake_scores": {{"uuid": score}}, "explained_scores": {{"uuid": score}}, "translated_scores": {{"uuid": score}}, "exploratory_scores": {{"uuid": score}}}}

Start your response with {{"#,
        dialect.name(),
        lang_section,
        format_mistakes_for_analysis(mistakes),
        format_explained_for_analysis(explained),
        format_translated_for_analysis(translated),
        format_exploratory_for_analysis(exploratory),
        JSON_OUTPUT_INSTRUCTION,
    )
}

pub fn try_parse_analysis(response: &str) -> Result<dialect_coach_shared::AgentAnalysis> {
    let normalized = normalize_json_response(response);
    let parsed: dialect_coach_shared::AgentAnalysis =
        serde_json::from_str(&normalized).map_err(|e| {
            tracing::warn!(
                "JSON parse error: {}. First 200 chars: {}",
                e,
                &response.chars().take(200).collect::<String>()
            );
            anyhow::anyhow!("JSON parse failed: {}", e)
        })?;

    Ok(parsed)
}

pub fn log_analysis_success(parsed_analysis: &dialect_coach_shared::AgentAnalysis) {
    tracing::info!(
        "Generated analysis ({} mistake scores, {} explained scores, {} translated scores, {} exploratory scores)",
        parsed_analysis.mistake_scores.len(),
        parsed_analysis.explained_scores.len(),
        parsed_analysis.translated_scores.len(),
        parsed_analysis.exploratory_scores.len()
    );
}

fn check_empty_learning_items(
    mistakes: &[Mistake],
    explained: &[Explained],
    translated: &[dialect_coach_shared::Translated],
    exploratory: &[dialect_coach_shared::Exploratory],
) -> bool {
    mistakes.is_empty() && explained.is_empty() && translated.is_empty() && exploratory.is_empty()
}

fn log_analysis_start(
    mistakes: &[Mistake],
    explained: &[Explained],
    translated: &[dialect_coach_shared::Translated],
    exploratory: &[dialect_coach_shared::Exploratory],
) {
    tracing::info!(
        "Starting analysis for {} mistakes, {} explained, {} translated, {} exploratory items",
        mistakes.len(),
        explained.len(),
        translated.len(),
        exploratory.len()
    );
}

fn format_analysis_prompt(msg: &str) -> String {
    format!("USER MESSAGE TO ANALYZE: {msg}")
}

async fn call_analysis_api(
    retry_ctx: &RetryContext,
    preamble: &str,
    prompt: &str,
) -> (Result<String, anyhow::Error>, Vec<AgentUsage>) {
    let mut history_with_prefill = Vec::new();
    if retry_ctx.agent.provider() == super::provider::ANTHROPIC_PROVIDER {
        history_with_prefill.push(super::util::create_prefilled_assistant_message());
    }

    let request = CompletionRequest {
        preamble,
        prompt,
        history: &history_with_prefill,
        max_tokens: 1024,
        temperature: 0.2,
    };
    let (result, usage) = retry_completion_call(retry_ctx.agent.as_ref(), &request, 3).await;
    match result {
        Ok(response) => (Ok(response), usage),
        Err(e) => (
            Err(e.context(format!(
                "Failed to get analysis from provider {} model {}",
                retry_ctx.agent.provider(),
                retry_ctx.agent.model()
            ))),
            usage,
        ),
    }
}

async fn handle_successful_analysis(
    retry_ctx: &RetryContext,
    response: String,
    usage: Vec<AgentUsage>,
    preamble: &str,
    msg: &String,
) -> (
    Result<dialect_coach_shared::AgentAnalysis, anyhow::Error>,
    Vec<AgentUsage>,
) {
    tracing::info!("Raw analysis response from Claude: {}", response);
    match handle_analysis_response(retry_ctx, &response, usage.clone(), preamble, msg).await {
        Ok((analysis, final_usage)) => (Ok(analysis), final_usage),
        Err(e) => (Err(e), usage),
    }
}

async fn handle_analysis_response(
    retry_ctx: &RetryContext,
    response: &str,
    initial_usage: Vec<AgentUsage>,
    preamble: &str,
    msg: &String,
) -> Result<(dialect_coach_shared::AgentAnalysis, Vec<AgentUsage>)> {
    match try_parse_analysis(response) {
        Ok(analysis) => {
            log_analysis_success(&analysis);
            Ok((analysis, initial_usage))
        }
        Err(_) => {
            tracing::warn!("Initial analysis parse failed, attempting retries");
            let mut history_with_prefill = Vec::new();
            if retry_ctx.agent.provider() == super::provider::ANTHROPIC_PROVIDER {
                history_with_prefill.push(super::util::create_prefilled_assistant_message());
            }

            let (analysis, retry_usage) = retry_analysis_with_error_feedback_tracked(
                retry_ctx,
                preamble,
                response,
                msg,
                &history_with_prefill,
            )
            .await?;
            let mut all_usage = initial_usage;
            all_usage.extend(retry_usage);
            Ok((analysis, all_usage))
        }
    }
}

pub struct AnalysisRequestParams<'a> {
    pub dialect: Dialect,
    pub msg: &'a String,
    pub mistakes: &'a [Mistake],
    pub explained: &'a [Explained],
    pub translated: &'a [dialect_coach_shared::Translated],
    pub exploratory: &'a [dialect_coach_shared::Exploratory],
    pub language_option: &'a Option<LanguageOption>,
}

pub struct AnalysisParams<'a> {
    pub retry_ctx: &'a RetryContext,
    pub dialect: Dialect,
    pub msg: &'a String,
    pub mistakes: &'a [Mistake],
    pub explained: &'a [Explained],
    pub translated: &'a [dialect_coach_shared::Translated],
    pub exploratory: &'a [dialect_coach_shared::Exploratory],
    pub language_option: &'a Option<LanguageOption>,
}

pub async fn generate_analysis(
    params: AnalysisParams<'_>,
) -> (
    Result<dialect_coach_shared::AgentAnalysis, anyhow::Error>,
    Vec<AgentUsage>,
) {
    let AnalysisParams {
        retry_ctx,
        dialect,
        msg,
        mistakes,
        explained,
        translated,
        exploratory,
        language_option,
    } = params;
    if check_empty_learning_items(mistakes, explained, translated, exploratory) {
        tracing::debug!("Skipping analysis - no learning items");
        return (Ok(dialect_coach_shared::AgentAnalysis::new()), Vec::new());
    }

    log_analysis_start(mistakes, explained, translated, exploratory);

    let preamble = analysis_agent_preamble(
        &dialect,
        mistakes,
        explained,
        translated,
        exploratory,
        language_option,
    );

    let prompt = format_analysis_prompt(msg);

    tracing::info!(
        "Analysis request:\n=== SYSTEM PREAMBLE ===\n{}\n=== USER MESSAGE ===\n{}",
        preamble,
        prompt
    );

    tracing::info!("Calling Claude API for analysis...");
    let (result, usage) = call_analysis_api(retry_ctx, &preamble, &prompt).await;

    match result {
        Ok(response) => {
            handle_successful_analysis(retry_ctx, response, usage, &preamble, msg).await
        }
        Err(e) => (Err(e.context("Failed to get analysis from Claude")), usage),
    }
}

async fn retry_analysis_with_error_feedback_tracked(
    retry_ctx: &RetryContext,
    original_preamble: &str,
    failed_response: &str,
    msg: &String,
    conversation_history: &[RigMessage],
) -> Result<(dialect_coach_shared::AgentAnalysis, Vec<AgentUsage>)> {
    let prompt = format!("USER MESSAGE TO ANALYZE: {msg}");
    let parse_fn = move |response: &str| -> Result<dialect_coach_shared::AgentAnalysis> {
        try_parse_analysis(response)
    };
    let log_success = move |parsed: &dialect_coach_shared::AgentAnalysis| {
        log_analysis_success(parsed);
    };
    let preamble_builder = move |preamble: &str, failed: &str| -> String {
        build_retry_analysis_preamble(preamble, failed)
    };
    let prompt_params = super::retry::RetryPromptParams {
        original_preamble,
        failed_response,
        prompt: &prompt,
        preamble_builder: &preamble_builder,
    };
    let config = super::util::GenerationConfig {
        max_tokens: 1024,
        temperature: 0.2,
    };
    retry_ctx
        .retry_with_error_feedback_tracked(
            &prompt_params,
            conversation_history,
            &config,
            &parse_fn,
            &log_success,
        )
        .await
}
