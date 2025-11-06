use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UsageStats {
    pub response_events: Vec<AgentUsage>,
    pub analysis_events: Vec<AgentUsage>,
    pub tts_events: Vec<TtsUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentUsage {
    pub timestamp: i64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub is_retry: bool,
    pub is_estimate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TtsUsage {
    pub timestamp: i64,
    pub characters: u64,
}

impl Default for UsageStats {
    fn default() -> Self {
        Self {
            response_events: Vec::new(),
            analysis_events: Vec::new(),
            tts_events: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_usage_stats() {
        let stats = UsageStats::default();
        assert_eq!(stats.response_events.len(), 0);
        assert_eq!(stats.analysis_events.len(), 0);
        assert_eq!(stats.tts_events.len(), 0);
    }

    #[test]
    fn test_usage_stats_serialization() {
        let stats = UsageStats {
            response_events: vec![
                AgentUsage {
                    timestamp: 1699564800,
                    input_tokens: 100,
                    output_tokens: 50,
                    is_retry: false,
                    is_estimate: false,
                },
                AgentUsage {
                    timestamp: 1699564900,
                    input_tokens: 150,
                    output_tokens: 0,
                    is_retry: true,
                    is_estimate: true,
                },
            ],
            analysis_events: vec![AgentUsage {
                timestamp: 1699564850,
                input_tokens: 200,
                output_tokens: 100,
                is_retry: false,
                is_estimate: false,
            }],
            tts_events: vec![TtsUsage {
                timestamp: 1699564920,
                characters: 250,
            }],
        };

        let json = serde_json::to_string(&stats).unwrap();
        let deserialized: UsageStats = serde_json::from_str(&json).unwrap();
        assert_eq!(stats, deserialized);
    }
}
