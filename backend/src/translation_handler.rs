use anyhow::{Context, Result};
use dialect_coach_shared::models::{
    Dialect, Formality, PhraseTranslation, TranslateRequest, TranslateResponse,
};

use crate::AppState;
use crate::selection_cache::SelectionCache;

/// Translate a selected phrase, using the cache when the same phrase was translated before.
pub async fn translate(state: &AppState, request: TranslateRequest) -> TranslateResponse {
    tracing::info!(
        "Translation request: {} -> {}",
        request.phrase,
        request.dialect
    );
    let outcome = async {
        let (dialect, formality) = parse_settings(&request)?;
        cached_translation(state, &request.phrase, &request.context, dialect, formality).await
    }
    .await;
    translate_response(&request.phrase, outcome)
}

pub(crate) fn parse_dialect(dialect: &str) -> Result<Dialect, String> {
    dialect
        .parse::<Dialect>()
        .map_err(|_| format!("Invalid dialect: {}", dialect))
}

/// The request's dialect and formality (casual when none is given).
fn parse_settings(request: &TranslateRequest) -> Result<(Dialect, Formality), String> {
    let dialect = parse_dialect(&request.dialect)?;
    let formality = match &request.formality {
        Some(formality) => formality
            .parse::<Formality>()
            .map_err(|_| format!("Invalid formality: {}", formality))?,
        None => Formality::Informal,
    };
    Ok((dialect, formality))
}

async fn cached_translation(
    state: &AppState,
    phrase: &str,
    context: &str,
    dialect: Dialect,
    formality: Formality,
) -> Result<Vec<PhraseTranslation>, String> {
    let cache = &state.translation_cache;
    let key = SelectionCache::generate_key("translate", dialect.id(), phrase);
    if let Some(cached) = cache.get_translation(&key).await {
        tracing::info!("Cache hit for translation: {}", key);
        return Ok(cached);
    }
    tracing::info!("Cache miss for translation: {}", key);
    let translated = translate_phrase(state, phrase, context, dialect, formality).await;
    let phrases = translated.map_err(|e| format!("Translation failed: {}", e))?;
    cache.set_translation(&key, phrases.clone()).await;
    Ok(phrases)
}

fn translate_response(
    phrase: &str,
    outcome: Result<Vec<PhraseTranslation>, String>,
) -> TranslateResponse {
    let (segmented_phrases, error) = match outcome {
        Ok(phrases) => (phrases, None),
        Err(error) => {
            tracing::error!("{}", error);
            (vec![], Some(error))
        }
    };
    TranslateResponse {
        original_sentence: phrase.to_string(),
        segmented_phrases,
        success: error.is_none(),
        error,
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
        r#"The phrase \"{}\" appears in this {} ({}) sentence: \"{}\"\n\n\
Provide the ENGLISH translation of \"{}\". The context is only used for disambiguation of meaning.\n\n\
CRITICAL: The 'target_text' field MUST be the EXACT phrase \"{}\" character-for-character.\n\
Do NOT modify, correct, normalize, or transliterate \"{}\". Copy it exactly as given.\n\n\
Return JSON array: [{{\"target_text\": \"{}\", \"english\": \"English meaning of {}\"}}, ...]"#,
        phrase,
        dialect.name(),
        formality_desc,
        context,
        phrase,
        phrase,
        phrase,
        phrase,
        phrase,
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

    fn request(dialect: &str, formality: Option<&str>) -> TranslateRequest {
        TranslateRequest {
            phrase: "che".to_string(),
            context: "¿Qué hacés, che?".to_string(),
            dialect: dialect.to_string(),
            formality: formality.map(str::to_string),
        }
    }

    #[test]
    fn test_parse_dialect_names_an_unknown_dialect() {
        assert_eq!(
            parse_dialect("spanish_argentinian"),
            Ok(Dialect::SpanishArgentinian)
        );
        assert_eq!(
            parse_dialect("klingon"),
            Err("Invalid dialect: klingon".to_string())
        );
    }

    #[test]
    fn test_parse_settings_defaults_to_informal() {
        assert_eq!(
            parse_settings(&request("spanish_argentinian", None)),
            Ok((Dialect::SpanishArgentinian, Formality::Informal))
        );
        assert_eq!(
            parse_settings(&request("spanish_argentinian", Some("formal"))),
            Ok((Dialect::SpanishArgentinian, Formality::Formal))
        );
        assert_eq!(
            parse_settings(&request("spanish_argentinian", Some("stiff"))),
            Err("Invalid formality: stiff".to_string())
        );
    }

    #[test]
    fn test_translate_response_reports_success_or_error() {
        let phrases = vec![PhraseTranslation {
            target_text: "che".to_string(),
            english: "hey".to_string(),
        }];
        let ok = translate_response("che", Ok(phrases.clone()));
        assert_eq!(
            (ok.success, ok.segmented_phrases, ok.error),
            (true, phrases, None)
        );
        let failed = translate_response("che", Err("Invalid dialect: x".to_string()));
        assert!(!failed.success && failed.segmented_phrases.is_empty());
        assert_eq!(failed.error.as_deref(), Some("Invalid dialect: x"));
        assert_eq!(failed.original_sentence, "che");
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
