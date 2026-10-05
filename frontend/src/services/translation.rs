use crate::services::connection::Connection;
use anyhow::Result;
use dialect_coach_shared::models::{Dialect, Formality, TranslateRequest, TranslateResponse};
use dialect_coach_shared::{ClientMessage, Reply, StudyRequest};

/// Translation service for AI-powered phrase translation
pub struct TranslationService {
    connection: Connection,
}

impl TranslationService {
    pub fn new(connection: Connection) -> Self {
        Self { connection }
    }

    /// Translate an English phrase to dialect-specific phrase
    pub async fn translate_phrase(
        &self,
        phrase: &str,
        context: String,
        dialect: Dialect,
        formality: Option<Formality>,
    ) -> Result<TranslateResponse> {
        // Use ONLY canonical serde ID formats
        let request = TranslateRequest {
            phrase: phrase.to_string(),
            context,
            dialect: dialect.id().to_string(),
            formality: formality.map(|f| f.id().to_string()),
        };
        let study = StudyRequest::Translate(request);
        let reply = self.connection.request(ClientMessage::Study(study)).await?;
        translated(reply)
    }
}

fn translated(reply: Reply) -> Result<TranslateResponse> {
    match reply {
        Reply::Translated(response) if response.success => Ok(response),
        Reply::Translated(response) => Err(anyhow::anyhow!(
            "Translation failed: {}",
            response
                .error
                .unwrap_or_else(|| "Unknown error".to_string())
        )),
        other => unreachable!(
            "a translate request is answered with Translated, got {:?}",
            other
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_serialization() {
        let request = TranslateRequest {
            phrase: "Hello, how are you?".to_string(),
            context: "ctx".to_string(),
            dialect: Dialect::SpanishMexican.id().to_string(),
            formality: Some(Formality::Informal.id().to_string()),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("spanish_mexican"));
        assert!(json.contains("informal"));
        assert!(json.contains("Hello, how are you?"));
    }

    #[test]
    fn test_request_with_all_dialects() {
        let test_cases = vec![
            (Dialect::SpanishMexican, "spanish_mexican"),
            (Dialect::ArabicEgyptian, "arabic_egyptian"),
            (Dialect::FrenchParisian, "french_parisian"),
        ];

        for (dialect, expected_id) in test_cases {
            let request = TranslateRequest {
                phrase: "Test phrase".to_string(),
                context: "ctx".to_string(),
                dialect: dialect.id().to_string(),
                formality: None,
            };

            let json = serde_json::to_string(&request).unwrap();
            assert!(
                json.contains(expected_id),
                "JSON should contain {}",
                expected_id
            );
        }
    }

    #[test]
    fn test_request_with_all_formalities() {
        let test_cases = vec![
            (Formality::Formal, "formal"),
            (Formality::ProfessionalCasual, "professional_casual"),
            (Formality::Informal, "informal"),
            (Formality::Slang, "slang"),
        ];

        for (formality, expected_id) in test_cases {
            let request = TranslateRequest {
                phrase: "Test phrase".to_string(),
                context: "ctx".to_string(),
                dialect: Dialect::SpanishMexican.id().to_string(),
                formality: Some(formality.id().to_string()),
            };

            let json = serde_json::to_string(&request).unwrap();
            assert!(
                json.contains(expected_id),
                "JSON should contain {}",
                expected_id
            );
        }
    }

    #[test]
    fn test_response_deserialization() {
        // Test successful response
        let success_json = r#"{"original_sentence":"Hello, how are you?","segmented_phrases":[{"target_text":"Hola","english":"Hello"},{"target_text":"¿cómo estás?","english":"how are you?"}],"success":true,"error":null}"#;
        let response: TranslateResponse = serde_json::from_str(success_json).unwrap();
        assert!(response.success);
        assert_eq!(response.original_sentence, "Hello, how are you?");
        assert_eq!(response.segmented_phrases.len(), 2);
        assert_eq!(response.segmented_phrases[0].target_text, "Hola");
        assert_eq!(response.segmented_phrases[0].english, "Hello");
        assert!(response.error.is_none());

        // Test error response
        let error_json = r#"{"original_sentence":"test","segmented_phrases":[],"success":false,"error":"Invalid dialect"}"#;
        let response: TranslateResponse = serde_json::from_str(error_json).unwrap();
        assert!(!response.success);
        assert_eq!(response.segmented_phrases.len(), 0);
        assert_eq!(response.error.unwrap(), "Invalid dialect");
    }

    #[test]
    fn test_translated_returns_success_and_names_failure() {
        let response = |success: bool, error: Option<&str>| TranslateResponse {
            original_sentence: "che".to_string(),
            segmented_phrases: vec![],
            success,
            error: error.map(str::to_string),
        };
        let ok = translated(Reply::Translated(response(true, None))).unwrap();
        assert_eq!(ok.original_sentence, "che");
        let refused = translated(Reply::Translated(response(
            false,
            Some("Invalid dialect: x"),
        )));
        assert_eq!(
            refused.unwrap_err().to_string(),
            "Translation failed: Invalid dialect: x"
        );
    }

    #[test]
    fn test_canonical_format_usage() {
        // Test that dialect.id() returns canonical format
        assert_eq!(Dialect::SpanishMexican.id(), "spanish_mexican");
        assert_eq!(Dialect::ArabicEgyptian.id(), "arabic_egyptian");
        assert_eq!(Dialect::FrenchParisian.id(), "french_parisian");

        // Test that formality.id() returns canonical format
        assert_eq!(Formality::Formal.id(), "formal");
        assert_eq!(Formality::ProfessionalCasual.id(), "professional_casual");
        assert_eq!(Formality::Informal.id(), "informal");
        assert_eq!(Formality::Slang.id(), "slang");
    }
}
