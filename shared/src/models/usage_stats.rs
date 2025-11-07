use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct UsageStats {
    pub response_events: Vec<AgentUsage>,
    pub analysis_events: Vec<AgentUsage>,
    pub tts_events: Vec<TtsUsage>,
}

impl UsageStats {
    pub fn response_count(&self) -> usize {
        self.response_events.len()
    }

    pub fn analysis_count(&self) -> usize {
        self.analysis_events.len()
    }

    pub fn tts_count(&self) -> usize {
        self.tts_events.len()
    }

    pub fn response_input_tokens(&self) -> u64 {
        self.response_events.iter().map(|u| u.input_tokens).sum()
    }

    pub fn response_output_tokens(&self) -> u64 {
        self.response_events.iter().map(|u| u.output_tokens).sum()
    }

    pub fn analysis_input_tokens(&self) -> u64 {
        self.analysis_events.iter().map(|u| u.input_tokens).sum()
    }

    pub fn analysis_output_tokens(&self) -> u64 {
        self.analysis_events.iter().map(|u| u.output_tokens).sum()
    }

    pub fn tts_characters(&self) -> u64 {
        self.tts_events.iter().map(|u| u.characters).sum()
    }

    pub fn response_retry_count(&self) -> usize {
        self.response_events.iter().filter(|u| u.is_retry).count()
    }

    pub fn response_estimate_count(&self) -> usize {
        self.response_events
            .iter()
            .filter(|u| u.is_estimate)
            .count()
    }

    pub fn analysis_retry_count(&self) -> usize {
        self.analysis_events.iter().filter(|u| u.is_retry).count()
    }

    pub fn analysis_estimate_count(&self) -> usize {
        self.analysis_events
            .iter()
            .filter(|u| u.is_estimate)
            .count()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentUsage {
    pub timestamp: i64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub is_retry: bool,
    pub is_estimate: bool,
}

impl AgentUsage {
    pub fn input_tokens_total(events: &[AgentUsage]) -> u64 {
        events.iter().map(|u| u.input_tokens).sum()
    }

    pub fn output_tokens_total(events: &[AgentUsage]) -> u64 {
        events.iter().map(|u| u.output_tokens).sum()
    }

    pub fn retry_count(events: &[AgentUsage]) -> usize {
        events.iter().filter(|u| u.is_retry).count()
    }

    pub fn estimate_count(events: &[AgentUsage]) -> usize {
        events.iter().filter(|u| u.is_estimate).count()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TtsUsage {
    pub timestamp: i64,
    pub characters: u64,
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
