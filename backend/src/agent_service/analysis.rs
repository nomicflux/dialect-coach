use anyhow::{Context, Result};
use dialect_coach_shared::{Dialect, Explained, Mistake};
use rig::completion::{Chat, Message as RigMessage};

use super::retry::{RetryContext, build_retry_analysis_preamble};
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
            .map(|t| serde_json::to_string(t).unwrap())
            .collect::<Vec<_>>()
            .join(", "),
        r#"TRANSLATED (-10 to 10): -10=reverted to untranslated, 0=not used, 10=correctly used. Give full points for different conjugations, declensions, etc. as the translated item."#
    )
}

pub fn format_exploratory_for_analysis(exploratory: &[dialect_coach_shared::Exploratory]) -> String {
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
) -> String {
    format!(
        r#"You are analyzing a language learner's progress in {}. Here are the learning items, with scoring directions for each category.

{}{}{}{}

Score each item (be generous; prefer to give points when in doubt. If you see the item in the user response, do not give a score of 0.).

CRITICAL RULES:
1. ALL SCORES MUST BE NUMBERS (integers from -10 to 10). NO TEXT IN THE SCORE FIELDS.
2. DO NOT WRITE ANY EXPLANATIONS OR ANALYSIS. NO TEXT BEFORE OR AFTER THE JSON.
3. Your response starts with {{ (already provided). Continue directly with the JSON fields - NO explanatory text after the {{.
4. Your response must contain ONLY valid JSON from {{ to }}.
5. If you have no scoring instructions for a category, return an empty object for that category.

Example correct format:
{{"mistake_scores": {{"id1": 8}}, "explained_scores": {{"id2": 5}}, "translated_scores": {{"id3": 10}}, "exploratory_scores": {{"id4": 7}}}}

{}

Return ONLY the JSON object, starting with {{:
{{"mistake_scores": {{}}, "explained_scores": {{}}, "translated_scores": {{}}, "exploratory_scores": {{}}}}"#,
        dialect.name(),
        format_mistakes_for_analysis(mistakes),
        format_explained_for_analysis(explained),
        format_translated_for_analysis(translated),
        format_exploratory_for_analysis(exploratory),
        JSON_OUTPUT_INSTRUCTION,
    )
}

pub fn try_parse_analysis(response: &str) -> Result<dialect_coach_shared::AgentAnalysis> {
    let normalized = normalize_json_response(response);
    let parsed: dialect_coach_shared::AgentAnalysis = serde_json::from_str(&normalized)
        .map_err(|e| {
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
    mistakes.is_empty()
        && explained.is_empty()
        && translated.is_empty()
        && exploratory.is_empty()
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

async fn call_analysis_api(retry_ctx: &RetryContext<'_>, preamble: &str, prompt: &str) -> Result<String> {
    let agent = retry_ctx
        .client
        .agent(retry_ctx.model_name)
        .max_tokens(1024)
        .temperature(0.2)
        .preamble(preamble)
        .build();
    let mut history_with_prefill = Vec::new();
    history_with_prefill.push(super::util::create_prefilled_assistant_message());
    agent
        .chat(prompt, history_with_prefill)
        .await
        .context("Failed to get analysis from Claude")
}

async fn handle_analysis_response(
    retry_ctx: &RetryContext<'_>,
    response: &str,
    preamble: &str,
    msg: &String,
) -> Result<(dialect_coach_shared::AgentAnalysis, u32)> {
    match try_parse_analysis(response) {
        Ok(analysis) => {
            log_analysis_success(&analysis);
            Ok((analysis, 0))
        }
        Err(_) => {
            tracing::warn!("Initial analysis parse failed, attempting retries");
            let mut history_with_prefill = Vec::new();
            history_with_prefill.push(super::util::create_prefilled_assistant_message());
            retry_analysis_with_error_feedback_tracked(
                retry_ctx,
                preamble,
                response,
                msg,
                &history_with_prefill,
            )
            .await
        }
    }
}

pub async fn generate_analysis(
    retry_ctx: &RetryContext<'_>,
    dialect: Dialect,
    msg: &String,
    mistakes: &[Mistake],
    explained: &[Explained],
    translated: &[dialect_coach_shared::Translated],
    exploratory: &[dialect_coach_shared::Exploratory],
) -> Result<(dialect_coach_shared::AgentAnalysis, u32)> {
    if check_empty_learning_items(mistakes, explained, translated, exploratory) {
        tracing::debug!("Skipping analysis - no learning items");
        return Ok((dialect_coach_shared::AgentAnalysis::new(), 0));
    }

    log_analysis_start(mistakes, explained, translated, exploratory);

    let preamble =
        analysis_agent_preamble(&dialect, mistakes, explained, translated, exploratory);
    tracing::info!("Analysis preamble sent to Claude:\n{}", preamble);

    let prompt = format_analysis_prompt(msg);
    tracing::info!("Analysis prompt sent to Claude:\n{}", prompt);

    tracing::info!("Calling Claude API for analysis...");
    let response = call_analysis_api(retry_ctx, &preamble, &prompt).await?;

    tracing::info!("Received analysis response from Claude API");
    tracing::info!("Raw analysis response from Claude: {}", response);

    handle_analysis_response(retry_ctx, &response, &preamble, msg).await
}

async fn retry_analysis_with_error_feedback_tracked(
    retry_ctx: &RetryContext<'_>,
    original_preamble: &str,
    failed_response: &str,
    msg: &String,
    conversation_history: &[RigMessage],
) -> Result<(dialect_coach_shared::AgentAnalysis, u32)> {
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

