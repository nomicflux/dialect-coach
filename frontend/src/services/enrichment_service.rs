use anyhow::{Context, Result};
use dialect_coach_shared::models::{EnrichRequest, EnrichResponse};
use gloo_net::http::Request;

#[derive(PartialEq)]
pub struct EnrichmentService {
    base_url: String,
}

impl EnrichmentService {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
        }
    }

    pub async fn enrich_learning_item(&self, request: EnrichRequest) -> Result<EnrichResponse> {
        let url = format!("{}/api/learning/enrich", self.base_url);

        let response = Request::post(&url)
            .json(&request)?
            .send()
            .await
            .context("Failed to connect to server")?;

        if !response.ok() {
            let error_msg = match response.status() {
                400 => "Invalid learning item data".to_string(),
                500 => "Server error while enriching item".to_string(),
                _ => format!("Server error ({})", response.status_text()),
            };
            return Err(anyhow::anyhow!(error_msg));
        }

        let enrich_response: EnrichResponse = response
            .json()
            .await
            .context("Invalid response from server")?;

        Ok(enrich_response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dialect_coach_shared::models::{Dialect, PartialLearningItem, PartialMistake};

    #[test]
    fn test_service_creation() {
        let service = EnrichmentService::new("http://localhost:3000");
        assert_eq!(service.base_url, "http://localhost:3000");
    }

    #[test]
    fn test_request_serialization() {
        let partial = PartialLearningItem::Mistake(PartialMistake {
            specific_mistake: Some("error".to_string()),
            correction: None,
            mistake_category: None,
        });

        let request = EnrichRequest {
            dialect: Dialect::SpanishMexican,
            partial_data: partial,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("spanish_mexican"));
        assert!(json.contains("error"));
    }
}
