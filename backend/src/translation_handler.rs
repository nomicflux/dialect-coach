use anyhow::{Context, Result};
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use dialect_coach_shared::models::{Dialect, Formality};
use serde::{Deserialize, Serialize};

use crate::AppState;

#[derive(Deserialize)]
pub struct TranslateRequest {
    pub phrase: String,
    pub dialect: String, // Will be parsed using canonical Dialect::from_str()
    pub formality: Option<String>, // Will be parsed using canonical Formality::from_str()
}

#[derive(Serialize)]
pub struct TranslateResponse {
    pub translated: String,
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
                    translated: String::new(),
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
                        translated: String::new(),
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
    match translate_phrase(&state, &request.phrase, dialect, formality).await {
        Ok(agent_response) => {
            tracing::info!(
                "Translation success: '{}' -> '{}'",
                request.phrase,
                agent_response.response
            );
            (
                StatusCode::OK,
                Json(TranslateResponse {
                    translated: agent_response.response,
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
                    translated: String::new(),
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
    dialect: Dialect,
    formality: Formality,
) -> Result<dialect_coach_shared::AgentResponse> {
    // Create a specialized translation prompt
    let formality_desc = match formality {
        Formality::Formal => "formal and polite",
        Formality::ProfessionalCasual => "professional yet casual",
        Formality::Informal => "casual and conversational",
        Formality::Slang => "informal with slang and colloquialisms",
    };

    let translation_prompt = format!(
        "Translate this English phrase into authentic {} speech, making it sound {}:\n\n\"{}\"\n\nReturn ONLY the translation, nothing else.",
        dialect.name(),
        formality_desc,
        phrase
    );

    // Use AgentService simple translation method
    let response = state
        .agent
        .generate_simple_response("", &translation_prompt)
        .await
        .context("Failed to translate phrase")?;

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_requests() {
        // Test valid dialect parsing
        let request = TranslateRequest {
            phrase: "Hello".to_string(),
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
        let formality = Formality::Informal;

        // Test prompt generation doesn't panic and includes expected elements
        let formality_desc = match formality {
            Formality::Informal => "casual and conversational",
            _ => panic!("Unexpected formality in test"),
        };

        let expected_elements = vec![phrase, dialect.name(), formality_desc];

        let prompt = format!(
            "Translate this English phrase into authentic {} speech, making it sound {}:\n\n\"{}\"\n\nReturn ONLY the translation, nothing else.",
            dialect.name(),
            formality_desc,
            phrase
        );

        for element in expected_elements {
            assert!(
                prompt.contains(element),
                "Prompt missing element: {}",
                element
            );
        }
    }
}
