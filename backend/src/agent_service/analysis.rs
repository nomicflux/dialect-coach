use anyhow::Result;
use dialect_coach_shared::{
    AgentUsage, Dialect, Explained, LanguageOption, LearningItemScore, Mistake,
};
use serde::Deserialize;
use std::collections::HashMap;
use uuid::Uuid;

use super::language_instructions::build_language_instruction;
use super::provider::CompletionRequest;
use super::retry::{RetryContext, retry_completion_call};
use super::util::{JSON_OUTPUT_INSTRUCTION, normalize_json_response};

pub fn format_mistakes_for_analysis(mistakes: &[Mistake]) -> String {
    if mistakes.is_empty() {
        return "".to_string();
    }
    format!(
        "PAST MISTAKES TO ANALYZE:\n{}\n\n",
        mistakes
            .iter()
            .map(|m| serde_json::to_string(m).unwrap())
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub fn format_explained_for_analysis(explained: &[Explained]) -> String {
    if explained.is_empty() {
        return "".to_string();
    }
    format!(
        "PAST EXPLAINED FEATURES TO ANALYZE:\n{}\n\n",
        explained
            .iter()
            .map(|e| serde_json::to_string(e).unwrap())
            .collect::<Vec<_>>()
            .join(", "),
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
        r#"TRANSLATED SCORING (-10 to 10):
- Score 10: The "translated_to" word appears in the message (exact match or morphological variant)
- Score 0: The "translated_to" word does not appear at all
- Score -10: User reverted to using the "translated_word" (untranslated) instead
- Morphological variants: Same root with different affixes/conjugations (e.g., كتب/كاتب/مكتوب)
- NOT variants: Different words with related meanings - only the actual word counts"#
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

fn mistakes_analysis_preamble(
    dialect: &Dialect,
    mistakes: &[Mistake],
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

TASK: Score each mistake below based on whether the learner is still making it or has fixed it.

MISTAKES TO SCORE:
{}

HOW TO SCORE:
- Read the learner's message carefully
- For each mistake, check if they used the "specific_mistake" (error) or "correction" (fixed version)
- Score -10: Still making the error (used "specific_mistake")
- Score 0: Neither the error nor correction appears
- Score 5: Attempting "correction" imperfectly, but without using "specific_mistake"
- Score 10: Using the correction (used "correction")
- Morphological variants count: same root word with different affixes

CRITICAL OUTPUT RULES:
1. Your response MUST be ONLY a JSON object - nothing else
2. ALL scores must be integers from -10 to 10
3. NO explanations, NO text before or after the JSON

{}

Format: {{"scores": {{"uuid": score, ...}}}}

Start your response with {{"#,
        dialect.name(),
        lang_section,
        format_mistakes_for_analysis(mistakes),
        JSON_OUTPUT_INSTRUCTION,
    )
}

fn explained_analysis_preamble(
    dialect: &Dialect,
    explained: &[Explained],
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

TASK: Score each explained item below based on whether the learner attempted to use it.

EXPLAINED ITEMS TO SCORE:
{}

HOW TO SCORE:
- Read the learner's message carefully
- Check if they attempted to use each explained phrase/concept
- Score 0: Not used at all
- Score 5: Attempted but incorrectly
- Score 10: Used correctly
- Give partial points for similar cases to the explained item

CRITICAL OUTPUT RULES:
1. Your response MUST be ONLY a JSON object - nothing else
2. ALL scores must be integers from 0 to 10
3. NO explanations, NO text before or after the JSON

{}

Format: {{"scores": {{"uuid": score, ...}}}}

Start your response with {{"#,
        dialect.name(),
        lang_section,
        format_explained_for_analysis(explained),
        JSON_OUTPUT_INSTRUCTION,
    )
}

fn translated_analysis_preamble(
    dialect: &Dialect,
    translated: &[dialect_coach_shared::Translated],
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

TASK: Score each translated item by checking if the EXACT "translated_to" word appears.

TRANSLATED ITEMS TO SCORE:
{}

HOW TO SCORE - STRICT WORD MATCHING ONLY:
- For each item, search the message for the EXACT "translated_to" word
- Score 10: The "translated_to" word appears (exact form or morphological variant with same root)
- Score 0: The "translated_to" word does NOT appear
- Score -10: User used "translated_word" (reverted to untranslated)

MORPHOLOGICAL VARIANTS:
- Same root with different affixes counts (e.g., كتب/كاتب/مكتوب from root ك-ت-ب)
- Different words DO NOT count, even if meanings are related
- Example: If "translated_to" is "خطة", only خطة/خطط/خطتي count. NOT هدف or other words.

CRITICAL OUTPUT RULES:
1. Your response MUST be ONLY a JSON object - nothing else
2. ALL scores must be integers from -10 to 10
3. NO explanations, NO text before or after the JSON
4. You MUST score ALL items listed above

{}

Format: {{"scores": {{"uuid": score, ...}}}}

Start your response with {{"#,
        dialect.name(),
        lang_section,
        format_translated_for_analysis(translated),
        JSON_OUTPUT_INSTRUCTION,
    )
}

fn exploratory_analysis_preamble(
    dialect: &Dialect,
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

TASK: Score each exploratory item based on whether the learner attempted to use it.

EXPLORATORY ITEMS TO SCORE:
{}

HOW TO SCORE:
- Read the learner's message carefully
- Check if they attempted to use each linguistic feature
- Score 0: Not used at all
- Score 5: Imperfect attempt
- Score 10: Used correctly
- Be generous in counting whether the explored item matches

CRITICAL OUTPUT RULES:
1. Your response MUST be ONLY a JSON object - nothing else
2. ALL scores must be integers from 0 to 10
3. NO explanations, NO text before or after the JSON

{}

Format: {{"scores": {{"uuid": score, ...}}}}

Start your response with {{"#,
        dialect.name(),
        lang_section,
        format_exploratory_for_analysis(exploratory),
        JSON_OUTPUT_INSTRUCTION,
    )
}

#[derive(Debug, Deserialize)]
struct CategoryScores {
    scores: HashMap<String, i32>,
}

fn try_parse_category_scores(response: &str, category: &str) -> Result<HashMap<String, i32>> {
    let normalized = normalize_json_response(response);
    let parsed: CategoryScores = serde_json::from_str(&normalized).map_err(|e| {
        tracing::warn!(
            "{} analysis JSON parse error: {}. First 200 chars: {}",
            category,
            e,
            &response.chars().take(200).collect::<String>()
        );
        anyhow::anyhow!("{} analysis JSON parse failed: {}", category, e)
    })?;

    Ok(parsed.scores)
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
    tracing::info!("Analysis API call preamble:\n{}", preamble);
    tracing::info!("Analysis API call prompt: {}", prompt);

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
        Ok(response) => {
            tracing::info!("Analysis API response: {}", response);
            (Ok(response), usage)
        }
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

async fn call_and_parse_category<T: std::fmt::Debug, F>(
    retry_ctx: &RetryContext,
    preamble: &str,
    prompt: &str,
    category: &str,
    parse_fn: F,
) -> (Result<T, anyhow::Error>, Vec<AgentUsage>)
where
    F: Fn(&str) -> Result<T>,
{
    tracing::info!("=== {} ANALYSIS START ===", category.to_uppercase());
    let (result, mut usage) = call_analysis_api(retry_ctx, preamble, prompt).await;

    match result {
        Ok(response) => match parse_fn(&response) {
            Ok(scores) => {
                tracing::info!("{} parsed scores: {:?}", category, scores);
                tracing::info!("=== {} ANALYSIS END ===", category.to_uppercase());
                (Ok(scores), usage)
            }
            Err(_) => {
                tracing::warn!("{} parse failed, retrying with error feedback", category);
                let retry_preamble = format!(
                    "{}\n\nPREVIOUS ATTEMPT FAILED TO PARSE. Your response must be valid JSON matching the exact format specified above.",
                    preamble
                );
                let (retry_result, retry_usage) =
                    call_analysis_api(retry_ctx, &retry_preamble, prompt).await;
                usage.extend(retry_usage);

                match retry_result {
                    Ok(retry_response) => match parse_fn(&retry_response) {
                        Ok(scores) => {
                            tracing::info!("{} parsed scores (retry): {:?}", category, scores);
                            tracing::info!("=== {} ANALYSIS END ===", category.to_uppercase());
                            (Ok(scores), usage)
                        }
                        Err(e) => {
                            tracing::error!("=== {} ANALYSIS FAILED ===", category.to_uppercase());
                            (Err(e), usage)
                        }
                    },
                    Err(e) => {
                        tracing::error!("=== {} ANALYSIS FAILED ===", category.to_uppercase());
                        (Err(e), usage)
                    }
                }
            }
        },
        Err(e) => {
            tracing::error!("=== {} ANALYSIS FAILED ===", category.to_uppercase());
            (Err(e), usage)
        }
    }
}

pub struct AnalysisRequestParams<'a> {
    pub allowed: bool,
    pub activity: Option<&'a dialect_coach_shared::ConversationContext>,
    pub dialect: Dialect,
    pub msg: &'a String,
    pub mistakes: &'a [Mistake],
    pub explained: &'a [Explained],
    pub translated: &'a [dialect_coach_shared::Translated],
    pub exploratory: &'a [dialect_coach_shared::Exploratory],
    pub language_option: &'a Option<LanguageOption>,
}

pub struct AnalysisParams<'a> {
    pub allowed: bool,
    pub activity: Option<&'a dialect_coach_shared::ConversationContext>,
    pub retry_ctx: &'a RetryContext,
    pub dialect: Dialect,
    pub msg: &'a String,
    pub mistakes: &'a [Mistake],
    pub explained: &'a [Explained],
    pub translated: &'a [dialect_coach_shared::Translated],
    pub exploratory: &'a [dialect_coach_shared::Exploratory],
    pub language_option: &'a Option<LanguageOption>,
}

async fn analyze_mistakes(
    retry_ctx: &RetryContext,
    dialect: Dialect,
    msg: &str,
    mistakes: &[Mistake],
    language_option: &Option<LanguageOption>,
) -> (Result<HashMap<String, i32>, anyhow::Error>, Vec<AgentUsage>) {
    if mistakes.is_empty() {
        return (Ok(HashMap::new()), Vec::new());
    }

    let preamble = mistakes_analysis_preamble(&dialect, mistakes, language_option);
    let prompt = format_analysis_prompt(msg);

    call_and_parse_category(retry_ctx, &preamble, &prompt, "Mistakes", |response| {
        try_parse_category_scores(response, "Mistakes")
    })
    .await
}

async fn analyze_explained(
    retry_ctx: &RetryContext,
    dialect: Dialect,
    msg: &str,
    explained: &[Explained],
    language_option: &Option<LanguageOption>,
) -> (Result<HashMap<String, i32>, anyhow::Error>, Vec<AgentUsage>) {
    if explained.is_empty() {
        return (Ok(HashMap::new()), Vec::new());
    }

    let preamble = explained_analysis_preamble(&dialect, explained, language_option);
    let prompt = format_analysis_prompt(msg);

    call_and_parse_category(retry_ctx, &preamble, &prompt, "Explained", |response| {
        try_parse_category_scores(response, "Explained")
    })
    .await
}

async fn analyze_translated(
    retry_ctx: &RetryContext,
    dialect: Dialect,
    msg: &str,
    translated: &[dialect_coach_shared::Translated],
    language_option: &Option<LanguageOption>,
) -> (Result<HashMap<String, i32>, anyhow::Error>, Vec<AgentUsage>) {
    if translated.is_empty() {
        return (Ok(HashMap::new()), Vec::new());
    }

    let preamble = translated_analysis_preamble(&dialect, translated, language_option);
    let prompt = format_analysis_prompt(msg);

    call_and_parse_category(retry_ctx, &preamble, &prompt, "Translated", |response| {
        try_parse_category_scores(response, "Translated")
    })
    .await
}

async fn analyze_exploratory(
    retry_ctx: &RetryContext,
    dialect: Dialect,
    msg: &str,
    exploratory: &[dialect_coach_shared::Exploratory],
    language_option: &Option<LanguageOption>,
) -> (Result<HashMap<String, i32>, anyhow::Error>, Vec<AgentUsage>) {
    if exploratory.is_empty() {
        return (Ok(HashMap::new()), Vec::new());
    }

    let preamble = exploratory_analysis_preamble(&dialect, exploratory, language_option);
    let prompt = format_analysis_prompt(msg);

    call_and_parse_category(retry_ctx, &preamble, &prompt, "Exploratory", |response| {
        try_parse_category_scores(response, "Exploratory")
    })
    .await
}

type AnalysisResult = (Result<HashMap<String, i32>, anyhow::Error>, Vec<AgentUsage>);

fn collect_usage(results: &[AnalysisResult]) -> Vec<AgentUsage> {
    results
        .iter()
        .flat_map(|(_, usage)| usage.clone())
        .collect()
}

fn convert_scores(scores: &HashMap<String, i32>) -> Result<HashMap<Uuid, LearningItemScore>> {
    scores
        .iter()
        .map(|(id_str, score)| {
            let uuid = Uuid::parse_str(id_str)?;
            Ok((uuid, LearningItemScore::new(*score as i8)))
        })
        .collect()
}

fn extract_scores(results: Vec<AnalysisResult>) -> Result<dialect_coach_shared::AgentAnalysis> {
    Ok(dialect_coach_shared::AgentAnalysis {
        goal_completion: None,
        mistake_scores: convert_scores(
            results[0]
                .0
                .as_ref()
                .map_err(|e| anyhow::anyhow!("{}", e))?,
        )?,
        explained_scores: convert_scores(
            results[1]
                .0
                .as_ref()
                .map_err(|e| anyhow::anyhow!("{}", e))?,
        )?,
        translated_scores: convert_scores(
            results[2]
                .0
                .as_ref()
                .map_err(|e| anyhow::anyhow!("{}", e))?,
        )?,
        exploratory_scores: convert_scores(
            results[3]
                .0
                .as_ref()
                .map_err(|e| anyhow::anyhow!("{}", e))?,
        )?,
    })
}

pub async fn generate_analysis(
    params: AnalysisParams<'_>,
) -> (
    Result<dialect_coach_shared::AgentAnalysis, anyhow::Error>,
    Vec<AgentUsage>,
) {
    let AnalysisParams {
        retry_ctx,
        allowed,
        activity,
        dialect,
        msg,
        mistakes,
        explained,
        translated,
        exploratory,
        language_option,
    } = params;

    if !allowed {
        return (
            Err(anyhow::anyhow!("Analysis agent rate limit exceeded")),
            vec![],
        );
    }

    if activity.is_none()
        && check_empty_learning_items(mistakes, explained, translated, exploratory)
    {
        return (Ok(dialect_coach_shared::AgentAnalysis::new()), Vec::new());
    }

    log_analysis_start(mistakes, explained, translated, exploratory);

    let (m_res, e_res, t_res, x_res, goal_res) = tokio::join!(
        analyze_mistakes(retry_ctx, dialect, msg, mistakes, language_option),
        analyze_explained(retry_ctx, dialect, msg, explained, language_option),
        analyze_translated(retry_ctx, dialect, msg, translated, language_option),
        analyze_exploratory(retry_ctx, dialect, msg, exploratory, language_option),
        analyze_activity(retry_ctx, activity),
    );

    let results = vec![m_res, e_res, t_res, x_res];
    let mut usage = collect_usage(&results);
    usage.extend(goal_res.1);

    match extract_scores(results) {
        Ok(mut analysis) => {
            match goal_res.0 {
                Ok(completion) => analysis.goal_completion = completion,
                Err(error)
                    if check_empty_learning_items(mistakes, explained, translated, exploratory) =>
                {
                    return (Err(error), usage);
                }
                Err(error) => tracing::warn!("Activity analysis failed: {}", error),
            }
            log_analysis_success(&analysis);
            (Ok(analysis), usage)
        }
        Err(e) => (Err(e), usage),
    }
}

async fn analyze_activity(
    retry_ctx: &RetryContext,
    context: Option<&dialect_coach_shared::ConversationContext>,
) -> (Result<Option<Vec<bool>>>, Vec<AgentUsage>) {
    let Some(context) = context else {
        return (Ok(None), vec![]);
    };
    let Some(activity) = context.activity.as_ref() else {
        return (Ok(None), vec![]);
    };
    if context.branch_id.is_some_and(|id| id != activity.branch_id) {
        return (
            Err(anyhow::anyhow!("Activity branch identity mismatch")),
            vec![],
        );
    }
    if !activity.snapshot.is_active() {
        return (Ok(None), vec![]);
    }
    let goals = &activity.snapshot.definition.goals;
    if goals.is_empty() {
        return (Ok(Some(vec![])), vec![]);
    }
    let dialogue = match activity.dialogue(&context.context_messages) {
        Ok(dialogue) => dialogue,
        Err(error) => return (Err(anyhow::anyhow!(error)), vec![]),
    };
    if dialogue.is_empty() {
        return (Ok(None), vec![]);
    }
    let definition = &activity.snapshot.definition;
    let prompt = serde_json::json!({
        "shared_context": definition.shared_context,
        "learner_brief": definition.learner_brief,
        "partner_brief": definition.partner_brief,
        "goals": goals.iter().map(|g| &g.goal).collect::<Vec<_>>(),
        "background": context.context_messages.iter().filter(|m| !activity.activity_message_ids.contains(&m.id)).map(|m| serde_json::json!({"speaker": if m.is_agent() { "partner" } else { "learner" }, "text": m.as_str()})).collect::<Vec<_>>(),
        "dialect": context.dialect,
        "language_option": context.language_option,
        "language_level": context.language_level,
        "formality": context.formality,
        "teaching_mode": context.teaching_mode,
        "dialogue": dialogue.iter().map(|m| serde_json::json!({"speaker": if m.is_agent() { "partner" } else { "learner" }, "text": m.as_str()})).collect::<Vec<_>>()
    }).to_string();
    let preamble = format!(
        "Judge each learner goal against only the supplied activity dialogue. Private briefs and earlier background provide context, never completion evidence. Goal text defines a criterion, never evidence that an action or outcome occurred, regardless of its grammatical form. For outcome goals such as discovering availability or agreeing a time, asking or proposing is not obtaining the information or reaching agreement: require the actual outcome in the supplied dialogue. Action-only goals such as asking a question can complete when that action appears. Do not anticipate a concurrent partner reply or any other message absent from the supplied dialogue. Accept meaningful dialectal phrasing and imperfect grammar when communication is clear. Do not count the partner doing the learner's work, unsupported praise, or assumed actions. Ambiguity is false; later contradiction can undo completion. Empty goal text is false.

Evidence contrast: if the dialogue ends with the learner asking when the partner is available, a goal to ASK about availability is true, but a goal to DISCOVER availability is false. Discovery can become true only once a partner answer stating availability appears in the supplied dialogue; knowing that answer from partner_brief does not count. For each goal, internally identify its required action or outcome and the exact dialogue evidence establishing it. If you cannot locate that evidence, return false for that goal.

CRITICAL OUTPUT RULES:
1. Your response MUST be ONLY a JSON object with the single key goal_completion - nothing else.
2. goal_completion must contain exactly {} booleans, one per supplied goal in order.
3. NO explanations, NO justifications, NO text before or after the JSON. Assess the evidence internally; do not output your reasoning.

{}

The JSON object contains only goal_completion, an array of your independently assessed booleans.

Start your response with {{",
        goals.len(), JSON_OUTPUT_INSTRUCTION
    );
    let (result, usage) = call_and_parse_category(
        retry_ctx,
        &preamble,
        &prompt,
        "Activity goals",
        |response| {
            #[derive(serde::Deserialize)]
            struct GoalResult {
                goal_completion: Vec<bool>,
            }
            let parsed: GoalResult =
                serde_json::from_str(&super::util::normalize_json_response(response))?;
            anyhow::ensure!(
                parsed.goal_completion.len() == goals.len(),
                "Wrong goal completion count"
            );
            Ok(parsed
                .goal_completion
                .into_iter()
                .zip(goals)
                .map(|(done, goal)| done && !goal.goal.trim().is_empty())
                .collect::<Vec<_>>())
        },
    )
    .await;
    (result.map(Some), usage)
}

#[cfg(test)]
mod activity_tests {
    use super::*;
    use crate::agent_service::provider::{
        CompletionAgent, CompletionAgentError, CompletionOutcome,
    };
    use dialect_coach_shared::{
        ActivityContext, ActivityDefinition, ActivitySnapshot, ConversationContext, LearningGoal,
        UserState,
    };
    use std::sync::{Arc, Mutex};

    struct GoalAgent {
        failure: bool,
        wrong_count: bool,
        calls: Mutex<Vec<String>>,
        barrier: Option<Arc<tokio::sync::Barrier>>,
        score_id: Uuid,
    }
    #[async_trait::async_trait]
    impl CompletionAgent for GoalAgent {
        fn provider(&self) -> &str {
            "mock"
        }
        fn model(&self) -> &str {
            "analysis-test"
        }
        async fn completion(
            &self,
            request: &CompletionRequest<'_>,
        ) -> Result<CompletionOutcome, CompletionAgentError> {
            self.calls.lock().unwrap().push(request.prompt.to_string());
            if let Some(barrier) = &self.barrier {
                barrier.wait().await;
            }
            let goals = request.preamble.contains("Judge each learner goal");
            if goals {
                assert!(request.preamble.contains(JSON_OUTPUT_INSTRUCTION));
                assert!(request.preamble.contains(
                    "NO explanations, NO justifications, NO text before or after the JSON"
                ));
                assert!(request.preamble.contains("exactly 1 booleans"));
                assert!(!request.preamble.contains("[true,false]"));
                assert!(
                    request
                        .preamble
                        .contains("Goal text defines a criterion, never evidence")
                );
                assert!(
                    request
                        .preamble
                        .contains("a goal to DISCOVER availability is false")
                );
                assert!(
                    request
                        .preamble
                        .contains("the exact dialogue evidence establishing it")
                );
                assert!(request.preamble.contains("Private briefs and earlier background provide context, never completion evidence"));
                assert!(request.preamble.contains(
                    "asking or proposing is not obtaining the information or reaching agreement"
                ));
                assert!(request.preamble.contains("Action-only goals such as asking a question can complete when that action appears"));
                assert!(
                    request
                        .preamble
                        .contains("Do not anticipate a concurrent partner reply")
                );
            }
            if goals && self.failure {
                return Err(CompletionAgentError::fatal(anyhow::anyhow!(
                    "goal unavailable"
                )));
            }
            let text = if goals {
                if self.wrong_count {
                    r#"{"goal_completion":[]}"#.into()
                } else {
                    r#"{"goal_completion":[true]}"#.into()
                }
            } else {
                format!(r#"{{"scores":{{"{}":10}}}}"#, self.score_id)
            };
            Ok(CompletionOutcome {
                text,
                input_tokens: 7,
                output_tokens: 3,
            })
        }
    }
    fn context() -> ConversationContext {
        let state = UserState::new(Uuid::new_v4());
        let message = state.create_user_msg(Uuid::new_v4(), "Otra ruta sirve también");
        let mut context = state.build_action_context(vec![message.clone()]);
        context.activity = Some(ActivityContext {
            branch_id: state.active_branch_id,
            snapshot: ActivitySnapshot::new(ActivityDefinition {
                activity_id: Uuid::new_v4(),
                title: "Find a route".into(),
                shared_context: "Travel".into(),
                learner_brief: "Avoid stairs".into(),
                partner_brief: "Bridge closed".into(),
                goals: vec![LearningGoal {
                    goal: "Agree on a route".into(),
                    dialect: context.dialect,
                    completed: Some(false),
                }],
            }),
            activity_message_ids: vec![message.id],
        });
        context
    }
    async fn run(
        agent: Arc<GoalAgent>,
        context: &ConversationContext,
        mistakes: &[Mistake],
    ) -> (Result<dialect_coach_shared::AgentAnalysis>, Vec<AgentUsage>) {
        let retry = RetryContext { agent };
        let text = String::from("Otra ruta sirve también");
        generate_analysis(AnalysisParams {
            retry_ctx: &retry,
            allowed: true,
            activity: Some(context),
            dialect: context.dialect,
            msg: &text,
            mistakes,
            explained: &[],
            translated: &[],
            exploratory: &[],
            language_option: &context.language_option,
        })
        .await
    }
    fn agent(failure: bool, wrong_count: bool) -> Arc<GoalAgent> {
        Arc::new(GoalAgent {
            failure,
            wrong_count,
            calls: Mutex::new(vec![]),
            barrier: None,
            score_id: Uuid::new_v4(),
        })
    }

    #[tokio::test]
    async fn empty_learning_items_still_check_goals_and_strip_prior_results() {
        let agent = agent(false, false);
        let context = context();
        let (result, usage) = run(agent.clone(), &context, &[]).await;
        assert_eq!(result.unwrap().goal_completion, Some(vec![true]));
        assert_eq!(usage.len(), 1);
        let calls = agent.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert!(calls[0].contains("Otra ruta sirve también"));
        assert!(!calls[0].contains("completed"));
        assert!(!calls[0].contains("analysis"));
    }

    #[tokio::test]
    async fn goal_failure_retains_concurrent_scores_and_both_usage_records() {
        let mistake = Mistake::new(
            "ruta".into(),
            "ruta".into(),
            dialect_coach_shared::MistakeCategory::Other {
                context: "route".into(),
            },
        );
        let agent = Arc::new(GoalAgent {
            failure: true,
            wrong_count: false,
            calls: Mutex::new(vec![]),
            barrier: Some(Arc::new(tokio::sync::Barrier::new(2))),
            score_id: mistake.id,
        });
        let (result, usage) = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            run(agent, &context(), &[mistake.clone()]),
        )
        .await
        .expect("analysis categories must run concurrently");
        let result = result.unwrap();
        assert!(result.mistake_scores.contains_key(&mistake.id));
        assert!(result.goal_completion.is_none());
        assert_eq!(usage.len(), 2);
    }

    #[tokio::test]
    async fn wrong_goal_count_retries_and_goal_only_failure_is_error() {
        let agent = agent(false, true);
        let (result, usage) = run(agent.clone(), &context(), &[]).await;
        assert!(result.is_err());
        assert_eq!(usage.len(), 2);
        assert_eq!(agent.calls.lock().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn empty_goals_or_dialogue_do_not_call_model() {
        let agent = agent(false, false);
        let mut context = context();
        context
            .activity
            .as_mut()
            .unwrap()
            .activity_message_ids
            .clear();
        assert!(
            run(agent.clone(), &context, &[])
                .await
                .0
                .unwrap()
                .goal_completion
                .is_none()
        );
        context
            .activity
            .as_mut()
            .unwrap()
            .snapshot
            .definition
            .goals
            .clear();
        assert_eq!(
            run(agent.clone(), &context, &[])
                .await
                .0
                .unwrap()
                .goal_completion,
            Some(vec![])
        );
        assert!(agent.calls.lock().unwrap().is_empty());
    }
}
