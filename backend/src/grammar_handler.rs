use anyhow::{Context, Result};
use dialect_coach_shared::models::{Dialect, GrammarExplanation, GrammarRequest, GrammarResponse};

use crate::AppState;
use crate::selection_cache::SelectionCache;
use crate::translation_handler::parse_dialect;

/// Explain the grammar of a selected phrase, using the cache when it was explained before.
pub async fn explain(state: &AppState, request: GrammarRequest) -> GrammarResponse {
    tracing::info!("Grammar request: {} -> {}", request.phrase, request.dialect);
    let outcome = async {
        let dialect = parse_dialect(&request.dialect)?;
        cached_explanations(state, &request.phrase, &request.context, dialect).await
    }
    .await;
    grammar_response(&request.phrase, outcome)
}

async fn cached_explanations(
    state: &AppState,
    phrase: &str,
    context: &str,
    dialect: Dialect,
) -> Result<Vec<GrammarExplanation>, String> {
    let cache = &state.translation_cache;
    let key = SelectionCache::generate_key("grammar", dialect.id(), phrase);
    if let Some(cached) = cache.get_grammar(&key).await {
        tracing::info!("Cache hit for grammar: {}", key);
        return Ok(cached);
    }
    tracing::info!("Cache miss for grammar: {}", key);
    let explained = explain_grammar(state, phrase, context, dialect).await;
    let explanations = explained.map_err(|e| format!("Grammar explanation failed: {}", e))?;
    cache.set_grammar(&key, explanations.clone()).await;
    Ok(explanations)
}

fn grammar_response(
    phrase: &str,
    outcome: Result<Vec<GrammarExplanation>, String>,
) -> GrammarResponse {
    let (explanations, error) = match outcome {
        Ok(explanations) => (explanations, None),
        Err(error) => {
            tracing::error!("{}", error);
            (vec![], Some(error))
        }
    };
    GrammarResponse {
        original_phrase: phrase.to_string(),
        explanations,
        success: error.is_none(),
        error,
    }
}

async fn explain_grammar(
    state: &AppState,
    phrase: &str,
    context: &str,
    dialect: Dialect,
) -> Result<Vec<GrammarExplanation>> {
    let system_preamble = "Return ONLY valid JSON array format. Each element must have 'element' and 'explanation' fields. No other text, no markdown formatting, just the JSON array.";

    let prompt = format!(
        r#"Explain the grammar of "{}" as it appears in this {} sentence: "{}"

For each grammatical element worth explaining (particles, verb conjugations, sentence structures, honorifics, constructions, etc.), provide:
- element: The specific grammatical element or pattern
- explanation: A clear, brief explanation suitable for a language learner

Find the top 1-3 salient grammatical points in the selection "{}". The user chose this specific selection, so focus on what this selection does grammatically in the full context.

Return JSON array: [{{"element": "...", "explanation": "..."}}]"#,
        phrase,
        dialect.name(),
        context,
        phrase,
    );

    let response = state
        .agent
        .generate_simple_response(system_preamble, &prompt)
        .await
        .context("Failed to get AI response")?;

    tracing::debug!("AI grammar response: {}", response.response);
    parse_grammar_explanations(&response.response)
}

fn parse_grammar_explanations(json_str: &str) -> Result<Vec<GrammarExplanation>> {
    serde_json::from_str(json_str).map_err(|e| {
        tracing::error!("JSON parse error: {}. Raw response: {}", e, json_str);
        anyhow::anyhow!(
            "Failed to parse AI response as JSON array of GrammarExplanation: {}",
            e
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grammar_response_reports_success_or_error() {
        let explanations = vec![GrammarExplanation {
            element: "voseo".to_string(),
            explanation: "vos takes hacés".to_string(),
        }];
        let ok = grammar_response("hacés", Ok(explanations.clone()));
        assert!(ok.success && ok.error.is_none());
        assert_eq!(ok.explanations, explanations);
        let failed = grammar_response("hacés", Err("Invalid dialect: x".to_string()));
        assert!(!failed.success && failed.explanations.is_empty());
        assert_eq!(failed.error.as_deref(), Some("Invalid dialect: x"));
        assert_eq!(failed.original_phrase, "hacés");
    }

    #[test]
    fn test_parse_grammar_explanations_valid() {
        let json = r#"[{"element": "subject verb order", "explanation": "The verb comes before the subject in questions"}]"#;
        let result = parse_grammar_explanations(json);
        assert!(result.is_ok());
        let explanations = result.unwrap();
        assert_eq!(explanations.len(), 1);
        assert_eq!(explanations[0].element, "subject verb order");
        assert!(explanations[0].explanation.contains("verb comes before"));
    }

    #[test]
    fn test_parse_grammar_explanations_empty_array() {
        let json = "[]";
        let result = parse_grammar_explanations(json);
        assert!(result.is_ok());
        let explanations = result.unwrap();
        assert_eq!(explanations.len(), 0);
    }

    #[test]
    fn test_parse_grammar_explanations_invalid() {
        let invalid_json = "not valid json";
        let result = parse_grammar_explanations(invalid_json);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_grammar_explanations_multiple() {
        let json = r#"[
            {"element": "particle wa", "explanation": "topic marker"},
            {"element": "verb stem", "explanation": "masu form indicates polite"}
        ]"#;
        let result = parse_grammar_explanations(json);
        assert!(result.is_ok());
        let explanations = result.unwrap();
        assert_eq!(explanations.len(), 2);
        assert_eq!(explanations[0].element, "particle wa");
        assert_eq!(explanations[1].element, "verb stem");
    }
}
