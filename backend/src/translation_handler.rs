use anyhow::{Context, Result};
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use dialect_coach_shared::models::{Dialect, Formality, PhraseTranslation};
use serde::{Deserialize, Serialize};

use crate::AppState;

#[derive(Deserialize)]
pub struct TranslateRequest {
    pub phrase: String,
    pub context: String,
    pub dialect: String, // Will be parsed using canonical Dialect::from_str()
    pub formality: Option<String>, // Will be parsed using canonical Formality::from_str()
}

#[derive(Serialize)]
pub struct TranslateResponse {
    pub original_sentence: String,
    pub segmented_phrases: Vec<PhraseTranslation>,
    pub success: bool,
    pub error: Option<String>,
}

/// Translation endpoint - translate English phrases to dialect-specific phrases
pub async fn translate_handler(
    State(state): State<AppState>,
    Json(request): Json<TranslateRequest>,
) -> impl IntoResponse {
    tracing::info!(
        "Translation request: {} -> {}",
        request.phrase,
        request.dialect
    );

    // Parse dialect using canonical method
    let dialect = match request.dialect.parse::<Dialect>() {
        Ok(d) => d,
        Err(e) => {
            tracing::error!("Invalid dialect '{}': {}", request.dialect, e);
            return (
                StatusCode::BAD_REQUEST,
                Json(TranslateResponse {
                    original_sentence: request.phrase.clone(),
                    segmented_phrases: vec![],
                    success: false,
                    error: Some(format!("Invalid dialect: {}", request.dialect)),
                }),
            );
        }
    };

    // Parse formality using canonical method (default to Casual)
    let formality = if let Some(formality_str) = request.formality {
        match formality_str.parse::<Formality>() {
            Ok(f) => f,
            Err(e) => {
                tracing::error!("Invalid formality '{}': {}", formality_str, e);
                return (
                    StatusCode::BAD_REQUEST,
                    Json(TranslateResponse {
                        original_sentence: request.phrase.clone(),
                        segmented_phrases: vec![],
                        success: false,
                        error: Some(format!("Invalid formality: {}", formality_str)),
                    }),
                );
            }
        }
    } else {
        Formality::Informal
    };

    // Translate the phrase
    match translate_phrase(
        &state,
        &request.phrase,
        &request.context,
        dialect,
        formality,
    )
    .await
    {
        Ok(segmented_phrases) => {
            tracing::info!(
                "Translation success: '{}' -> {} phrases",
                request.phrase,
                segmented_phrases.len()
            );
            (
                StatusCode::OK,
                Json(TranslateResponse {
                    original_sentence: request.phrase.clone(),
                    segmented_phrases,
                    success: true,
                    error: None,
                }),
            )
        }
        Err(e) => {
            tracing::error!("Translation failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(TranslateResponse {
                    original_sentence: request.phrase.clone(),
                    segmented_phrases: vec![],
                    success: false,
                    error: Some(format!("Translation failed: {}", e)),
                }),
            )
        }
    }
}

/// Core translation function using AgentService
async fn translate_phrase(
    state: &AppState,
    phrase: &str,
    context: &str,
    dialect: Dialect,
    formality: Formality,
) -> Result<Vec<PhraseTranslation>> {
    let formality_desc = match formality {
        Formality::Formal => "formal and polite",
        Formality::ProfessionalCasual => "professional yet casual",
        Formality::Informal => "casual and conversational",
        Formality::Slang => "informal with slang and colloquialisms",
    };

    let system_preamble = "Return ONLY valid JSON array format. Each element must have 'target_text' and 'english' fields. No other text, no markdown formatting, just the JSON array.";

    let translation_prompt = format!(
        "Translate the phrase \"{}\" into {} ({}), in the context of this sentence: \"{}\"\n\nOnly translate the given phrase, the context is only used for disambiguation of meaning.\n\nReturn JSON array: [{{\"target_text\": \"phrase in target language\", \"english\": \"English translation\"}}, ...]",
        phrase,
        dialect.name(),
        formality_desc,
        context
    );

    let response = state
        .agent
        .generate_simple_response(system_preamble, &translation_prompt)
        .await
        .context("Failed to get AI response")?;

    tracing::debug!("AI translation response: {}", response.response);
    parse_phrase_translations(&response.response)
}

fn parse_phrase_translations(json_str: &str) -> Result<Vec<PhraseTranslation>> {
    serde_json::from_str(json_str).map_err(|e| {
        tracing::error!("JSON parse error: {}. Raw response: {}", e, json_str);
        anyhow::anyhow!(
            "Failed to parse AI response as JSON array of PhraseTranslation: {}",
            e
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_requests() {
        // Test valid dialect parsing
        let request = TranslateRequest {
            phrase: "Hello".to_string(),
            context: "ctx".to_string(),
            dialect: "spanish_mexican".to_string(),
            formality: Some("informal".to_string()),
        };

        assert!(request.dialect.parse::<Dialect>().is_ok());
        assert_eq!(
            request.dialect.parse::<Dialect>().unwrap(),
            Dialect::SpanishMexican
        );
        assert!(
            request
                .formality
                .as_ref()
                .unwrap()
                .parse::<Formality>()
                .is_ok()
        );
        assert_eq!(
            request.formality.unwrap().parse::<Formality>().unwrap(),
            Formality::Informal
        );

        // Test invalid dialect parsing
        let invalid_request = TranslateRequest {
            phrase: "Hello".to_string(),
            context: "ctx".to_string(),
            dialect: "invalid_dialect".to_string(),
            formality: None,
        };

        assert!(invalid_request.dialect.parse::<Dialect>().is_err());
    }

    #[test]
    fn test_all_supported_dialects() {
        // Test all dialect variants can be parsed using canonical format
        let test_cases = vec![
            "spanish_mexican",
            "spanish_castilian",
            "arabic_egyptian",
            "arabic_levantine",
            "french_parisian",
            "french_quebecois",
        ];

        for dialect_str in test_cases {
            let result = dialect_str.parse::<Dialect>();
            assert!(result.is_ok(), "Failed to parse dialect: {}", dialect_str);
        }
    }

    #[test]
    fn test_all_supported_formalities() {
        // Test all formality variants can be parsed using canonical format
        let test_cases = vec!["formal", "professional_casual", "informal", "slang"];

        for formality_str in test_cases {
            let result = formality_str.parse::<Formality>();
            assert!(
                result.is_ok(),
                "Failed to parse formality: {}",
                formality_str
            );
        }
    }

    #[test]
    fn test_translation_prompt_generation() {
        let phrase = "Hello, how are you?";
        let dialect = Dialect::SpanishMexican;
        let formality_desc = "casual and conversational";

        let prompt = format!(
            "Segment this sentence into useful 2-4 word phrases and translate each to {} ({}):\n\n\"{}\"\n\nReturn JSON array: [{{\"target_text\": \"phrase in target language\", \"english\": \"English translation\"}}, ...]",
            dialect.name(),
            formality_desc,
            phrase
        );

        assert!(prompt.contains(phrase));
        assert!(prompt.contains(dialect.name()));
        assert!(prompt.contains(formality_desc));
        assert!(prompt.contains("2-4 word phrases"));
        assert!(prompt.contains("JSON array"));
    }

    #[test]
    fn test_parse_phrase_translations_valid() {
        let json = r#"[{"target_text": "Hola", "english": "Hello"}, {"target_text": "¿Cómo estás?", "english": "How are you?"}]"#;
        let result = parse_phrase_translations(json);
        assert!(result.is_ok());
        let phrases = result.unwrap();
        assert_eq!(phrases.len(), 2);
        assert_eq!(phrases[0].target_text, "Hola");
        assert_eq!(phrases[0].english, "Hello");
        assert_eq!(phrases[1].target_text, "¿Cómo estás?");
        assert_eq!(phrases[1].english, "How are you?");
    }

    #[test]
    fn test_parse_phrase_translations_invalid() {
        let invalid_json = "not valid json";
        let result = parse_phrase_translations(invalid_json);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_phrase_translations_empty_array() {
        let json = "[]";
        let result = parse_phrase_translations(json);
        assert!(result.is_ok());
        let phrases = result.unwrap();
        assert_eq!(phrases.len(), 0);
    }
}
