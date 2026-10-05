use crate::services::connection::Connection;
use anyhow::Result;
use dialect_coach_shared::models::{EnrichRequest, EnrichResponse};
use dialect_coach_shared::{ClientMessage, EnrichFailure, Reply, StudyRequest};

#[derive(PartialEq)]
pub struct EnrichmentService {
    connection: Connection,
}

impl EnrichmentService {
    pub fn new(connection: Connection) -> Self {
        Self { connection }
    }

    pub async fn enrich_learning_item(&self, request: EnrichRequest) -> Result<EnrichResponse> {
        let study = StudyRequest::Enrich(request);
        let reply = self.connection.request(ClientMessage::Study(study)).await?;
        enriched(reply)
    }
}

fn enriched(reply: Reply) -> Result<EnrichResponse> {
    match reply {
        Reply::Enriched(Ok(response)) => Ok(response),
        Reply::Enriched(Err(EnrichFailure::InvalidInput)) => {
            Err(anyhow::anyhow!("Invalid learning item data"))
        }
        Reply::Enriched(Err(EnrichFailure::Server)) => {
            Err(anyhow::anyhow!("Server error while enriching item"))
        }
        other => unreachable!(
            "an enrich request is answered with Enriched, got {:?}",
            other
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dialect_coach_shared::models::{Dialect, PartialLearningItem, PartialMistake};

    #[test]
    fn test_enriched_names_each_failure() {
        let item = serde_json::json!({"correction": "vos sos"});
        let ok = enriched(Reply::Enriched(Ok(EnrichResponse {
            enriched_item: item.clone(),
        })));
        assert_eq!(ok.unwrap().enriched_item, item);
        let invalid = enriched(Reply::Enriched(Err(EnrichFailure::InvalidInput)));
        assert_eq!(
            invalid.unwrap_err().to_string(),
            "Invalid learning item data"
        );
        let server = enriched(Reply::Enriched(Err(EnrichFailure::Server)));
        assert_eq!(
            server.unwrap_err().to_string(),
            "Server error while enriching item"
        );
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
