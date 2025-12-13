use crate::agent_service::planning::PlanGenerator;
use crate::parsing::{self, ParsedContent};
use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use dialect_coach_shared::Dialect;
use serde_json::json;
use std::sync::Arc;

pub async fn generate_plan_handler(
    State(planning_generator): State<Arc<PlanGenerator>>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    // 1. Extract parts
    let (content, dialect) = match extract_data_from_multipart(&mut multipart).await {
        Ok(data) => data,
        Err(e) => return e.into_response(),
    };

    // 2. Generate Plan
    let result = match content {
        ParsedContent::Text(text) => planning_generator.generate_plan(&text, dialect).await,
        ParsedContent::ScannedImages(images) => {
            planning_generator
                .generate_plan_from_images(&images, dialect)
                .await
        }
    };

    match result {
        Ok(plan) => Json(plan).into_response(),
        Err(e) => {
            tracing::error!("Plan generation failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
                .into_response()
        }
    }
}

async fn extract_data_from_multipart(
    multipart: &mut Multipart,
) -> Result<(ParsedContent, Dialect), (StatusCode, Json<serde_json::Value>)> {
    let mut content = None;
    let mut dialect_opt = None;

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let name = field.name().unwrap_or("").to_string();
        if name == "dialect" {
            dialect_opt = parse_dialect_field(field).await;
        } else if name == "file" || name == "text" {
            content = parse_file_field(field).await?;
        }
    }

    validate_inputs(content, dialect_opt)
}

async fn parse_dialect_field(field: axum::extract::multipart::Field<'_>) -> Option<Dialect> {
    if let Ok(val) = field.text().await {
        return serde_json::from_str::<Dialect>(&format!("\"{}\"", val)).ok();
    }
    None
}

async fn parse_file_field(
    field: axum::extract::multipart::Field<'_>,
) -> Result<Option<ParsedContent>, (StatusCode, Json<serde_json::Value>)> {
    let content_type = field.content_type().unwrap_or("text/plain").to_string();
    if let Ok(bytes) = field.bytes().await {
        let source = map_bytes_to_source(&bytes, &content_type)?;
        return parsing::extract_clean_text(source).map(Some).map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": e.to_string()})),
            )
        });
    }
    Ok(None)
}

fn map_bytes_to_source(
    bytes: &[u8],
    content_type: &str,
) -> Result<parsing::DocumentSource, (StatusCode, Json<serde_json::Value>)> {
    match content_type {
        "text/plain" => std::str::from_utf8(bytes)
            .map(|s| parsing::DocumentSource::Text(s.to_string()))
            .map_err(|_| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(json!({"error": "Invalid UTF-8 in text file"})),
                )
            }),
        "text/html" => std::str::from_utf8(bytes)
            .map(|s| parsing::DocumentSource::Html(s.to_string()))
            .map_err(|_| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(json!({"error": "Invalid UTF-8 in HTML file"})),
                )
            }),
        "application/pdf" => Ok(parsing::DocumentSource::Pdf(bytes.to_vec())),
        _ => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Unsupported file type"})),
        )),
    }
}

fn validate_inputs(
    content: Option<ParsedContent>,
    dialect: Option<Dialect>,
) -> Result<(ParsedContent, Dialect), (StatusCode, Json<serde_json::Value>)> {
    let content = match content {
        Some(c) => match &c {
            ParsedContent::Text(t) if t.trim().is_empty() => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(json!({"error": "Empty text content"})),
                ));
            }
            ParsedContent::Text(_) => c,
            ParsedContent::ScannedImages(imgs) if imgs.is_empty() => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(json!({"error": "No images found in scanned content"})),
                ));
            }
            ParsedContent::ScannedImages(_) => c,
        },
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "No file or text content found"})),
            ));
        }
    };

    let dialect = match dialect {
        Some(d) => d,
        None => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "Missing or invalid dialect"})),
            ));
        }
    };
    Ok((content, dialect))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_service::provider::{
        CompletionAgent, CompletionAgentError, CompletionOutcome, CompletionRequest,
    };
    use async_trait::async_trait;
    use axum::{
        Router,
        body::Body,
        http::{Request, header},
        routing::post,
    };
    use tower::ServiceExt;

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

    #[tokio::test]
    async fn test_generate_plan_endpoint() {
        let mock_agent = Arc::new(MockAgent);
        let config = crate::agent_service::provider::ProviderAgentConfig::openai(
            "test".to_string(),
            None,
            200,
        );
        let generator = Arc::new(PlanGenerator::new(mock_agent, config));

        let app = Router::new()
            .route("/generate", post(generate_plan_handler))
            .with_state(generator);

        let boundary = "boundary123";
        let body_bytes = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"dialect\"\r\n\r\nspanish_mexican\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"text\"\r\n\r\nSome text content\r\n--{boundary}--\r\n",
            boundary = boundary
        );

        let request = Request::builder()
            .uri("/generate")
            .method("POST")
            .header(
                header::CONTENT_TYPE,
                format!("multipart/form-data; boundary={}", boundary),
            )
            .body(Body::from(body_bytes))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let bytes = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

        assert_eq!(json["title"], "Mock Plan");
        assert_eq!(json["dialect"], "spanish_mexican");
    }
}
