use dialect_coach_shared::models::{EnrichFailure, EnrichRequest, EnrichResponse};

use crate::AppState;

/// Fill missing fields in a partial learning item.
pub async fn enrich(
    state: &AppState,
    request: EnrichRequest,
) -> Result<EnrichResponse, EnrichFailure> {
    tracing::info!("Enrichment request received");
    state
        .agent
        .enrich_learning_item(request)
        .await
        .inspect(|_| tracing::info!("Enrichment success"))
        .map_err(|e| {
            tracing::error!("Enrichment failed: {}", e);
            map_enrich_error(&e)
        })
}

fn map_enrich_error(error: &anyhow::Error) -> EnrichFailure {
    let error_msg = error.to_string();
    if error_msg.contains("required") || error_msg.contains("invalid") {
        EnrichFailure::InvalidInput
    } else {
        EnrichFailure::Server
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_enrich_error_validation() {
        let validation_error = anyhow::anyhow!("required field missing");
        assert_eq!(
            map_enrich_error(&validation_error),
            EnrichFailure::InvalidInput
        );
    }

    #[test]
    fn test_map_enrich_error_invalid() {
        let invalid_error = anyhow::anyhow!("invalid data format");
        assert_eq!(
            map_enrich_error(&invalid_error),
            EnrichFailure::InvalidInput
        );
    }

    #[test]
    fn test_map_enrich_error_server() {
        let server_error = anyhow::anyhow!("AI service unavailable");
        assert_eq!(map_enrich_error(&server_error), EnrichFailure::Server);
    }
}
