use crate::agent_service::planning::PlanGenerator;
use crate::parsing::{self, ParsedContent};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use dialect_coach_shared::models::plan::import::SimpleImportLanguagePlan;
use dialect_coach_shared::{PlanRequest, PlanSource};

const MAX_FILE_BYTES: usize = 10 * 1024 * 1024;

/// Generate a language plan from pasted text or an uploaded file.
pub async fn generate_plan(
    generator: &PlanGenerator,
    request: PlanRequest,
) -> Result<SimpleImportLanguagePlan, String> {
    let dialect = request.dialect;
    let result = match parse_source(request.source)? {
        ParsedContent::Text(text) => generator.generate_plan(&text, dialect).await,
        ParsedContent::ScannedImages(images) => {
            generator.generate_plan_from_images(&images, dialect).await
        }
    };
    result.map_err(|e| {
        tracing::error!("Plan generation failed: {}", e);
        e.to_string()
    })
}

fn parse_source(source: PlanSource) -> Result<ParsedContent, String> {
    let document = match source {
        PlanSource::Text(text) => parsing::DocumentSource::Text(text),
        PlanSource::File {
            content_type,
            base64,
        } => map_bytes_to_source(&decode_file(&base64)?, &content_type)?,
    };
    let content = parsing::extract_clean_text(document).map_err(|e| e.to_string())?;
    validate_inputs(content)
}

/// The uploaded file's bytes, at most 10 MB.
fn decode_file(base64: &str) -> Result<Vec<u8>, String> {
    let bytes = STANDARD
        .decode(base64)
        .map_err(|_| "Invalid file encoding".to_string())?;
    if bytes.len() > MAX_FILE_BYTES {
        return Err("File is larger than 10 MB".to_string());
    }
    Ok(bytes)
}

fn map_bytes_to_source(
    bytes: &[u8],
    content_type: &str,
) -> Result<parsing::DocumentSource, String> {
    match content_type {
        "text/plain" => std::str::from_utf8(bytes)
            .map(|s| parsing::DocumentSource::Text(s.to_string()))
            .map_err(|_| "Invalid UTF-8 in text file".to_string()),
        "text/html" => std::str::from_utf8(bytes)
            .map(|s| parsing::DocumentSource::Html(s.to_string()))
            .map_err(|_| "Invalid UTF-8 in HTML file".to_string()),
        "application/pdf" => Ok(parsing::DocumentSource::Pdf(bytes.to_vec())),
        _ => Err("Unsupported file type".to_string()),
    }
}

fn validate_inputs(content: ParsedContent) -> Result<ParsedContent, String> {
    match &content {
        ParsedContent::Text(t) if t.trim().is_empty() => Err("Empty text content".to_string()),
        ParsedContent::ScannedImages(imgs) if imgs.is_empty() => {
            Err("No images found in scanned content".to_string())
        }
        _ => Ok(content),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_service::provider::{
        CompletionAgent, CompletionAgentError, CompletionOutcome, CompletionRequest,
    };
    use async_trait::async_trait;
    use dialect_coach_shared::Dialect;
    use std::sync::Arc;

    struct MockAgent;

    #[async_trait]
    impl CompletionAgent for MockAgent {
        fn provider(&self) -> &'static str {
            "mock"
        }
        fn model(&self) -> &'static str {
            "mock-model"
        }
        async fn completion(
            &self,
            _request: &CompletionRequest<'_>,
        ) -> Result<CompletionOutcome, CompletionAgentError> {
            Ok(CompletionOutcome {
                text: r#"{
                    "title": "Mock Plan",
                    "dialect": "spanish_mexican",
                    "steps": []
                }"#
                .to_string(),
                input_tokens: 10,
                output_tokens: 10,
            })
        }
    }

    fn generator() -> PlanGenerator {
        let config = crate::agent_service::provider::ProviderAgentConfig::openai(
            "test".to_string(),
            "gpt-4o".to_string(),
            200,
        );
        PlanGenerator::new(Arc::new(MockAgent), config)
    }

    fn request(source: PlanSource) -> PlanRequest {
        PlanRequest {
            dialect: Dialect::SpanishMexican,
            source,
        }
    }

    #[tokio::test]
    async fn test_generate_plan_from_text() {
        let source = PlanSource::Text("Some text content".to_string());
        let plan = generate_plan(&generator(), request(source)).await.unwrap();
        assert_eq!(plan.title, "Mock Plan");
        assert_eq!(plan.dialect, Dialect::SpanishMexican);
    }

    #[tokio::test]
    async fn test_generate_plan_from_text_file() {
        let source = PlanSource::File {
            content_type: "text/plain".to_string(),
            base64: STANDARD.encode("Some text content"),
        };
        let plan = generate_plan(&generator(), request(source)).await.unwrap();
        assert_eq!(plan.title, "Mock Plan");
    }

    #[tokio::test]
    async fn test_generate_plan_rejects_empty_text() {
        let source = PlanSource::Text("   ".to_string());
        let refused = generate_plan(&generator(), request(source)).await;
        assert_eq!(refused.unwrap_err(), "Empty text content");
    }

    #[test]
    fn test_decode_file_rejects_bad_encoding_and_oversize() {
        assert_eq!(decode_file("aGk="), Ok(b"hi".to_vec()));
        assert_eq!(
            decode_file("not base64!"),
            Err("Invalid file encoding".to_string())
        );
        let oversize = STANDARD.encode(vec![0u8; MAX_FILE_BYTES + 1]);
        assert_eq!(
            decode_file(&oversize),
            Err("File is larger than 10 MB".to_string())
        );
    }

    #[test]
    fn test_map_bytes_to_source_rejects_unsupported_type() {
        let refused = map_bytes_to_source(b"GIF89a", "image/gif");
        assert_eq!(refused.err(), Some("Unsupported file type".to_string()));
        assert!(matches!(
            map_bytes_to_source(b"hola", "text/plain"),
            Ok(parsing::DocumentSource::Text(t)) if t == "hola"
        ));
    }
}
