use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use dialect_coach_shared::models::{EnrichRequest, EnrichResponse};

use crate::AppState;

/// Enrichment endpoint - fill missing fields in partial learning items
pub async fn enrich_handler(
    State(state): State<AppState>,
    Json(request): Json<EnrichRequest>,
) -> impl IntoResponse {
    tracing::info!("Enrichment request received");
    match state.agent.enrich_learning_item(request).await {
        Ok(response) => handle_success(response),
        Err(e) => handle_error(&e),
    }
}

fn handle_success(response: EnrichResponse) -> (StatusCode, Json<EnrichResponse>) {
    tracing::info!("Enrichment success");
    (StatusCode::OK, Json(response))
}

fn handle_error(error: &anyhow::Error) -> (StatusCode, Json<EnrichResponse>) {
    let status = map_enrich_error(error);
    tracing::error!("Enrichment failed: {}", error);
    (
        status,
        Json(EnrichResponse {
            enriched_item: serde_json::json!({"error": error.to_string()}),
        }),
    )
}

fn map_enrich_error(error: &anyhow::Error) -> StatusCode {
    let error_msg = error.to_string();
    if error_msg.contains("required") || error_msg.contains("invalid") {
        StatusCode::BAD_REQUEST
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_enrich_error_validation() {
        let validation_error = anyhow::anyhow!("required field missing");
        assert_eq!(map_enrich_error(&validation_error), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_map_enrich_error_invalid() {
        let invalid_error = anyhow::anyhow!("invalid data format");
        assert_eq!(map_enrich_error(&invalid_error), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_map_enrich_error_server() {
        let server_error = anyhow::anyhow!("AI service unavailable");
        assert_eq!(
            map_enrich_error(&server_error),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }
}
