use anyhow::{Context, Result};
use dialect_coach_shared::models::{Dialect, Formality};
use gloo_net::http::Request;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct TranslateRequest {
    phrase: String,
    dialect: String,           // Will send canonical serde ID format
    formality: Option<String>, // Will send canonical serde ID format
}

#[derive(Deserialize)]
struct TranslateResponse {
    translated: String,
    success: bool,
    error: Option<String>,
}

/// Translation service for AI-powered phrase translation
pub struct TranslationService {
    base_url: String,
}

impl TranslationService {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
        }
    }

    /// Translate an English phrase to dialect-specific phrase
    pub async fn translate_phrase(
        &self,
        phrase: &str,
        dialect: Dialect,
        formality: Option<Formality>,
    ) -> Result<String> {
        // Use ONLY canonical serde ID formats
        let request_body = TranslateRequest {
            phrase: phrase.to_string(),
            dialect: dialect.id().to_string(), // Canonical serde ID format
            formality: formality.map(|f| f.id().to_string()), // Canonical serde ID format
        };

        let url = format!("{}/api/translate", self.base_url);

        let response = Request::post(&url)
            .json(&request_body)?
            .send()
            .await
            .context("Failed to send translation request")?;

        if !response.ok() {
            return Err(anyhow::anyhow!(
                "Translation request failed with status: {}",
                response.status()
            ));
        }

        let translate_response: TranslateResponse = response
            .json()
            .await
            .context("Failed to parse translation response")?;

        if !translate_response.success {
            return Err(anyhow::anyhow!(
                "Translation failed: {}",
                translate_response
                    .error
                    .unwrap_or_else(|| "Unknown error".to_string())
            ));
        }

        Ok(translate_response.translated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_serialization() {
        let request = TranslateRequest {
            phrase: "Hello, how are you?".to_string(),
            dialect: Dialect::SpanishMexican.id().to_string(),
            formality: Some(Formality::Casual.id().to_string()),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("spanish_mexican"));
        assert!(json.contains("casual"));
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
            (Formality::Casual, "casual"),
            (Formality::DialectRich, "dialect_rich"),
            (Formality::Slang, "slang"),
        ];

        for (formality, expected_id) in test_cases {
            let request = TranslateRequest {
                phrase: "Test phrase".to_string(),
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
        let success_json = r#"{"translated":"¡Hola! ¿Cómo estás?","success":true,"error":null}"#;
        let response: TranslateResponse = serde_json::from_str(success_json).unwrap();
        assert!(response.success);
        assert_eq!(response.translated, "¡Hola! ¿Cómo estás?");
        assert!(response.error.is_none());

        // Test error response
        let error_json = r#"{"translated":"","success":false,"error":"Invalid dialect"}"#;
        let response: TranslateResponse = serde_json::from_str(error_json).unwrap();
        assert!(!response.success);
        assert_eq!(response.translated, "");
        assert_eq!(response.error.unwrap(), "Invalid dialect");
    }

    #[test]
    fn test_canonical_format_usage() {
        // Verify that we always use canonical serde ID formats
        let service = TranslationService::new("http://localhost:3000");
        assert_eq!(service.base_url, "http://localhost:3000");

        // Test that dialect.id() returns canonical format
        assert_eq!(Dialect::SpanishMexican.id(), "spanish_mexican");
        assert_eq!(Dialect::ArabicEgyptian.id(), "arabic_egyptian");
        assert_eq!(Dialect::FrenchParisian.id(), "french_parisian");

        // Test that formality.id() returns canonical format
        assert_eq!(Formality::Formal.id(), "formal");
        assert_eq!(Formality::Casual.id(), "casual");
        assert_eq!(Formality::DialectRich.id(), "dialect_rich");
        assert_eq!(Formality::Slang.id(), "slang");
    }
}
