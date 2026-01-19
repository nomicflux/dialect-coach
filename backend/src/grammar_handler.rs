use anyhow::{Context, Result};
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use dialect_coach_shared::models::{Dialect, GrammarExplanation, GrammarRequest, GrammarResponse};

use crate::AppState;
use crate::selection_cache::SelectionCache;

pub async fn grammar_handler(
    State(state): State<AppState>,
    Json(request): Json<GrammarRequest>,
) -> impl IntoResponse {
    tracing::info!("Grammar request: {} -> {}", request.phrase, request.dialect);

    let dialect = match request.dialect.parse::<Dialect>() {
        Ok(d) => d,
        Err(e) => {
            tracing::error!("Invalid dialect '{}': {}", request.dialect, e);
            return (
                StatusCode::BAD_REQUEST,
                Json(GrammarResponse {
                    original_phrase: request.phrase.clone(),
                    explanations: vec![],
                    success: false,
                    error: Some(format!("Invalid dialect: {}", request.dialect)),
                }),
            );
        }
    };

    let cache_key = SelectionCache::generate_key("grammar", dialect.id(), &request.phrase);
    if let Some(cached) = state.translation_cache.get_grammar(&cache_key).await {
        tracing::info!("Cache hit for grammar: {}", cache_key);
        return (
            StatusCode::OK,
            Json(GrammarResponse {
                original_phrase: request.phrase.clone(),
                explanations: cached,
                success: true,
                error: None,
            }),
        );
    }

    match explain_grammar(&state, &request.phrase, &request.context, dialect).await {
        Ok(explanations) => {
            tracing::info!("Cache miss for grammar: {}", cache_key);
            state
                .translation_cache
                .set_grammar(&cache_key, explanations.clone())
                .await;
            (
                StatusCode::OK,
                Json(GrammarResponse {
                    original_phrase: request.phrase.clone(),
                    explanations,
                    success: true,
                    error: None,
                }),
            )
        }
        Err(e) => {
            tracing::error!("Grammar explanation failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GrammarResponse {
                    original_phrase: request.phrase.clone(),
                    explanations: vec![],
                    success: false,
                    error: Some(format!("Grammar explanation failed: {}", e)),
                }),
            )
        }
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
