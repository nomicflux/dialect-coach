use dialect_coach_shared::models::usage_stats::{AgentUsage, TtsUsage, UsageStats};

/// Filters agent events to only those within the rolling window
pub fn prune_old_agent_events(
    events: &[AgentUsage],
    window_hours: u32,
    now: i64,
) -> Vec<AgentUsage> {
    let window_seconds = (window_hours as i64) * 3600;
    let cutoff = now - window_seconds;
    events
        .iter()
        .filter(|e| e.timestamp > cutoff)
        .cloned()
        .collect()
}

/// Filters TTS events to only those within the rolling window
pub fn prune_old_tts_events(events: &[TtsUsage], window_hours: u32, now: i64) -> Vec<TtsUsage> {
    let window_seconds = (window_hours as i64) * 3600;
    let cutoff = now - window_seconds;
    events
        .iter()
        .filter(|e| e.timestamp > cutoff)
        .cloned()
        .collect()
}

/// Adds response agent usage events and prunes old events
pub fn add_response_usage(
    stats: &mut UsageStats,
    new_usages: Vec<AgentUsage>,
    now: i64,
    window_hours: u32,
) {
    stats.response_events.extend(new_usages);
    stats.response_events = prune_old_agent_events(&stats.response_events, window_hours, now);
}

/// Adds analysis agent usage events and prunes old events
pub fn add_analysis_usage(
    stats: &mut UsageStats,
    new_usages: Vec<AgentUsage>,
    now: i64,
    window_hours: u32,
) {
    stats.analysis_events.extend(new_usages);
    stats.analysis_events = prune_old_agent_events(&stats.analysis_events, window_hours, now);
}

/// Adds TTS usage event and prunes old events
pub fn add_tts_usage(stats: &mut UsageStats, characters: u64, now: i64, window_hours: u32) {
    let tts_event = TtsUsage {
        timestamp: now,
        characters,
    };
    stats.tts_events.push(tts_event);
    stats.tts_events = prune_old_tts_events(&stats.tts_events, window_hours, now);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prune_old_agent_events_keeps_recent() {
        let now = 1000000;
        let events = vec![
            AgentUsage {
                timestamp: now - 3600,
                input_tokens: 100,
                output_tokens: 50,
                is_retry: false,
                is_estimate: false,
            },
            AgentUsage {
                timestamp: now - 7200,
                input_tokens: 200,
                output_tokens: 100,
                is_retry: false,
                is_estimate: false,
            },
        ];

        let pruned = prune_old_agent_events(&events, 2, now);
        assert_eq!(pruned.len(), 1);
        assert_eq!(pruned[0].input_tokens, 100);
    }

    #[test]
    fn test_prune_old_agent_events_removes_old() {
        let now = 1000000;
        let events = vec![AgentUsage {
            timestamp: now - 10000,
            input_tokens: 100,
            output_tokens: 50,
            is_retry: false,
            is_estimate: false,
        }];

        let pruned = prune_old_agent_events(&events, 1, now);
        assert_eq!(pruned.len(), 0);
    }

    #[test]
    fn test_prune_old_tts_events_keeps_recent() {
        let now = 1000000;
        let events = vec![
            TtsUsage {
                timestamp: now - 3600,
                characters: 100,
            },
            TtsUsage {
                timestamp: now - 10000,
                characters: 200,
            },
        ];

        let pruned = prune_old_tts_events(&events, 2, now);
        assert_eq!(pruned.len(), 1);
        assert_eq!(pruned[0].characters, 100);
    }

    #[test]
    fn test_add_response_usage() {
        let mut stats = UsageStats::default();
        let now = 1000000;

        let new_usages = vec![AgentUsage {
            timestamp: now,
            input_tokens: 100,
            output_tokens: 50,
            is_retry: false,
            is_estimate: false,
        }];

        add_response_usage(&mut stats, new_usages, now, 24);
        assert_eq!(stats.response_events.len(), 1);
        assert_eq!(stats.response_events[0].input_tokens, 100);
    }

    #[test]
    fn test_add_response_usage_prunes_old() {
        let now = 1000000;
        let mut stats = UsageStats {
            response_events: vec![AgentUsage {
                timestamp: now - 100000,
                input_tokens: 999,
                output_tokens: 999,
                is_retry: false,
                is_estimate: false,
            }],
            analysis_events: Vec::new(),
            tts_events: Vec::new(),
        };

        let new_usages = vec![AgentUsage {
            timestamp: now,
            input_tokens: 100,
            output_tokens: 50,
            is_retry: false,
            is_estimate: false,
        }];

        add_response_usage(&mut stats, new_usages, now, 24);
        assert_eq!(stats.response_events.len(), 1);
        assert_eq!(stats.response_events[0].input_tokens, 100);
    }

    #[test]
    fn test_add_analysis_usage() {
        let mut stats = UsageStats::default();
        let now = 1000000;

        let new_usages = vec![AgentUsage {
            timestamp: now,
            input_tokens: 200,
            output_tokens: 100,
            is_retry: false,
            is_estimate: false,
        }];

        add_analysis_usage(&mut stats, new_usages, now, 24);
        assert_eq!(stats.analysis_events.len(), 1);
        assert_eq!(stats.analysis_events[0].input_tokens, 200);
    }

    #[test]
    fn test_add_tts_usage() {
        let mut stats = UsageStats::default();
        let now = 1000000;

        add_tts_usage(&mut stats, 150, now, 24);
        assert_eq!(stats.tts_events.len(), 1);
        assert_eq!(stats.tts_events[0].characters, 150);
        assert_eq!(stats.tts_events[0].timestamp, now);
    }

    #[test]
    fn test_add_tts_usage_prunes_old() {
        let now = 1000000;
        let mut stats = UsageStats {
            response_events: Vec::new(),
            analysis_events: Vec::new(),
            tts_events: vec![TtsUsage {
                timestamp: now - 100000,
                characters: 999,
            }],
        };

        add_tts_usage(&mut stats, 150, now, 24);
        assert_eq!(stats.tts_events.len(), 1);
        assert_eq!(stats.tts_events[0].characters, 150);
    }

    #[test]
    fn test_add_response_usage_multiple_events() {
        let mut stats = UsageStats::default();
        let now = 1000000;

        let new_usages = vec![
            AgentUsage {
                timestamp: now,
                input_tokens: 100,
                output_tokens: 50,
                is_retry: false,
                is_estimate: false,
            },
            AgentUsage {
                timestamp: now + 1,
                input_tokens: 150,
                output_tokens: 0,
                is_retry: true,
                is_estimate: true,
            },
        ];

        add_response_usage(&mut stats, new_usages, now, 24);
        assert_eq!(stats.response_events.len(), 2);
        assert_eq!(stats.response_events[0].input_tokens, 100);
        assert_eq!(stats.response_events[1].is_retry, true);
    }
}
