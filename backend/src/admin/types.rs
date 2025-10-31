// Re-export admin types from shared crate
pub use dialect_coach_shared::{
    AdminStatusResponse, AnthropicStats, DialectCount, ElevenLabsStats, QdrantStats,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anthropic_stats_serialization() {
        let stats = AnthropicStats {
            uncached_input_tokens: Some(1000),
            cached_input_tokens: Some(500),
            cache_creation_tokens: Some(100),
            output_tokens: Some(750),
            total_cost_usd: Some("5.25".to_string()),
            error: None,
        };
        let json = serde_json::to_string(&stats).unwrap();
        let deserialized: AnthropicStats = serde_json::from_str(&json).unwrap();
        assert_eq!(stats.uncached_input_tokens, deserialized.uncached_input_tokens);
        assert_eq!(stats.total_cost_usd, deserialized.total_cost_usd);
    }

    #[test]
    fn test_elevenlabs_stats_serialization() {
        let stats = ElevenLabsStats {
            tier: "creator".to_string(),
            characters_used: 5000,
            characters_limit: 100000,
            characters_remaining: 95000,
            reset_date: "2025-11-01T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&stats).unwrap();
        let deserialized: ElevenLabsStats = serde_json::from_str(&json).unwrap();
        assert_eq!(stats.tier, deserialized.tier);
        assert_eq!(stats.characters_used, deserialized.characters_used);
    }

    #[test]
    fn test_qdrant_stats_serialization() {
        let stats = QdrantStats {
            points_count: 10911,
            vectors_count: 10911,
            segments_count: 4,
            dialect_counts: vec![
                DialectCount {
                    dialect: "es-MX".to_string(),
                    count: 2341,
                },
                DialectCount {
                    dialect: "ar-EG".to_string(),
                    count: 1456,
                },
            ],
            memory_bytes: Some(134217728),
        };
        let json = serde_json::to_string(&stats).unwrap();
        let deserialized: QdrantStats = serde_json::from_str(&json).unwrap();
        assert_eq!(stats.points_count, deserialized.points_count);
        assert_eq!(stats.dialect_counts.len(), deserialized.dialect_counts.len());
    }

    #[test]
    fn test_admin_status_response_serialization() {
        let response = AdminStatusResponse {
            timestamp: "2025-10-31T12:00:00Z".to_string(),
            anthropic: AnthropicStats {
                uncached_input_tokens: Some(1000),
                cached_input_tokens: Some(500),
                cache_creation_tokens: Some(100),
                output_tokens: Some(750),
                total_cost_usd: Some("5.25".to_string()),
                error: None,
            },
            elevenlabs: Some(ElevenLabsStats {
                tier: "pro".to_string(),
                characters_used: 1000,
                characters_limit: 500000,
                characters_remaining: 499000,
                reset_date: "2025-11-01T00:00:00Z".to_string(),
            }),
            qdrant: None,
        };
        let json = serde_json::to_string(&response).unwrap();
        let deserialized: AdminStatusResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(response.timestamp, deserialized.timestamp);
        assert!(deserialized.elevenlabs.is_some());
        assert!(deserialized.qdrant.is_none());
    }
}
