use super::{Dialect, PartialLearningItem};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnrichRequest {
    pub dialect: Dialect,
    pub partial_data: PartialLearningItem,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrichResponse {
    pub enriched_item: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enrich_request_serialization() {
        let partial = PartialLearningItem::Mistake(super::super::PartialMistake {
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

    #[test]
    fn test_enrich_request_deserialization() {
        let json = r#"{"dialect": "spanish_mexican", "partial_data": {"type": "mistake", "specific_mistake": "error", "correction": null, "mistake_category": null}}"#;
        let request: EnrichRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.dialect, Dialect::SpanishMexican);
    }

    #[test]
    fn test_enrich_response_serialization() {
        let response = EnrichResponse {
            enriched_item: serde_json::json!({"type": "mistake", "specific_mistake": "error"}),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("enriched_item"));
    }

    #[test]
    fn test_enrich_response_deserialization() {
        let json = r#"{"enriched_item": {"type": "mistake"}}"#;
        let response: EnrichResponse = serde_json::from_str(json).unwrap();
        assert!(response.enriched_item.is_object());
    }
}
