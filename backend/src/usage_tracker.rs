use dialect_coach_shared::models::usage_stats::{AgentUsage, TtsUsage, UsageStats};
use tracing::info;

/// Filters agent events to only those within the rolling window
pub fn prune_old_agent_events(
    events: &[AgentUsage],
    window_hours: u32,
    now: i64,
) -> Vec<AgentUsage> {
    let window_seconds = (window_hours as i64) * 3600;
    let cutoff = now - window_seconds;
    let initial_count = events.len();
    let pruned: Vec<AgentUsage> = events
        .iter()
        .filter(|e| e.timestamp > cutoff)
        .cloned()
        .collect();
    let removed_count = initial_count - pruned.len();
    if removed_count > 0 {
        info!(
            "Pruned {} old agent events (cutoff: {}, window: {} hours)",
            removed_count, cutoff, window_hours
        );
    }
    pruned
}

/// Filters TTS events to only those within the rolling window
pub fn prune_old_tts_events(events: &[TtsUsage], window_hours: u32, now: i64) -> Vec<TtsUsage> {
    let window_seconds = (window_hours as i64) * 3600;
    let cutoff = now - window_seconds;
    let initial_count = events.len();
    let pruned: Vec<TtsUsage> = events
        .iter()
        .filter(|e| e.timestamp > cutoff)
        .cloned()
        .collect();
    let removed_count = initial_count - pruned.len();
    if removed_count > 0 {
        info!(
            "Pruned {} old TTS events (cutoff: {}, window: {} hours)",
            removed_count, cutoff, window_hours
        );
    }
    pruned
}

/// Adds response agent usage events and prunes old events
pub fn add_response_usage(
    stats: &mut UsageStats,
    new_usages: Vec<AgentUsage>,
    now: i64,
    window_hours: u32,
) {
    let count = new_usages.len();
    let total_input_tokens = AgentUsage::input_tokens_total(&new_usages);
    let total_output_tokens = AgentUsage::output_tokens_total(&new_usages);
    let retry_count = AgentUsage::retry_count(&new_usages);
    let estimate_count = AgentUsage::estimate_count(&new_usages);

    info!(
        "Adding {} response usage events: {} input tokens, {} output tokens, {} retries, {} estimates",
        count, total_input_tokens, total_output_tokens, retry_count, estimate_count
    );

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
    let count = new_usages.len();
    let total_input_tokens = AgentUsage::input_tokens_total(&new_usages);
    let total_output_tokens = AgentUsage::output_tokens_total(&new_usages);
    let retry_count = AgentUsage::retry_count(&new_usages);
    let estimate_count = AgentUsage::estimate_count(&new_usages);

    info!(
        "Adding {} analysis usage events: {} input tokens, {} output tokens, {} retries, {} estimates",
        count, total_input_tokens, total_output_tokens, retry_count, estimate_count
    );

    stats.analysis_events.extend(new_usages);
    stats.analysis_events = prune_old_agent_events(&stats.analysis_events, window_hours, now);
}

/// Adds learning agent usage events and prunes old events
pub fn add_learning_usage(
    stats: &mut UsageStats,
    new_usages: Vec<AgentUsage>,
    now: i64,
    window_hours: u32,
) {
    let count = new_usages.len();
    let total_input_tokens = AgentUsage::input_tokens_total(&new_usages);
    let total_output_tokens = AgentUsage::output_tokens_total(&new_usages);
    let retry_count = AgentUsage::retry_count(&new_usages);
    let estimate_count = AgentUsage::estimate_count(&new_usages);

    info!(
        "Adding {} learning usage events: {} input tokens, {} output tokens, {} retries, {} estimates",
        count, total_input_tokens, total_output_tokens, retry_count, estimate_count
    );

    stats.learning_events.extend(new_usages);
    stats.learning_events = prune_old_agent_events(&stats.learning_events, window_hours, now);
}

/// Adds TTS usage event and prunes old events
pub fn add_tts_usage(stats: &mut UsageStats, characters: u64, now: i64, window_hours: u32) {
    info!(
        "Adding TTS usage event: {} characters at timestamp {}",
        characters, now
    );

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

    fn sample_usage(
        timestamp: i64,
        input_tokens: u64,
        output_tokens: u64,
        is_retry: bool,
        is_estimate: bool,
    ) -> AgentUsage {
        AgentUsage {
            timestamp,
            input_tokens,
            output_tokens,
            is_retry,
            is_estimate,
            provider: "anthropic".to_string(),
            model: "claude-test".to_string(),
        }
    }

    #[test]
    fn test_prune_old_agent_events_keeps_recent() {
        let now = 1000000;
        let events = vec![
            sample_usage(now - 3600, 100, 50, false, false),
            sample_usage(now - 7200, 200, 100, false, false),
        ];

        let pruned = prune_old_agent_events(&events, 2, now);
        assert_eq!(pruned.len(), 1);
        assert_eq!(pruned[0].input_tokens, 100);
    }

    #[test]
    fn test_prune_old_agent_events_removes_old() {
        let now = 1000000;
        let events = vec![sample_usage(now - 10000, 100, 50, false, false)];

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

        let new_usages = vec![sample_usage(now, 100, 50, false, false)];

        add_response_usage(&mut stats, new_usages, now, 24);
        assert_eq!(stats.response_events.len(), 1);
        assert_eq!(stats.response_events[0].input_tokens, 100);
    }

    #[test]
    fn test_add_response_usage_prunes_old() {
        let now = 1000000;
        let mut stats = UsageStats {
            response_events: vec![sample_usage(now - 100000, 999, 999, false, false)],
            analysis_events: Vec::new(),
            learning_events: Vec::new(),
            tts_events: Vec::new(),
        };

        let new_usages = vec![sample_usage(now, 100, 50, false, false)];

        add_response_usage(&mut stats, new_usages, now, 24);
        assert_eq!(stats.response_events.len(), 1);
        assert_eq!(stats.response_events[0].input_tokens, 100);
    }

    #[test]
    fn test_add_analysis_usage() {
        let mut stats = UsageStats::default();
        let now = 1000000;

        let new_usages = vec![sample_usage(now, 200, 100, false, false)];

        add_analysis_usage(&mut stats, new_usages, now, 24);
        assert_eq!(stats.analysis_events.len(), 1);
        assert_eq!(stats.analysis_events[0].input_tokens, 200);
    }

    #[test]
    fn test_add_learning_usage() {
        let mut stats = UsageStats::default();
        let now = 1000000;

        let new_usages = vec![sample_usage(now, 150, 75, false, false)];

        add_learning_usage(&mut stats, new_usages, now, 24);
        assert_eq!(stats.learning_events.len(), 1);
        assert_eq!(stats.learning_events[0].input_tokens, 150);
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
            learning_events: Vec::new(),
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
            sample_usage(now, 100, 50, false, false),
            sample_usage(now + 1, 150, 0, true, true),
        ];

        add_response_usage(&mut stats, new_usages, now, 24);
        assert_eq!(stats.response_events.len(), 2);
        assert_eq!(stats.response_events[0].input_tokens, 100);
        assert!(stats.response_events[1].is_retry);
    }
}
