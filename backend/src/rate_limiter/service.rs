use dialect_coach_shared::models::usage_stats::UsageStats;
use std::sync::Arc;

use super::config::RateLimitConfig;
use super::org_quota::OrgQuotaChecker;

pub trait RateLimiterService {
    fn can_make_response_call(&self, stats: &UsageStats, config: &RateLimitConfig) -> bool;
    fn can_make_analysis_call(&self, stats: &UsageStats, config: &RateLimitConfig) -> bool;
    fn can_make_tts_call(&self, stats: &UsageStats, config: &RateLimitConfig) -> bool;
    async fn anthropic_has_quota(&self) -> bool;
    async fn elevenlabs_has_quota(&self) -> bool;
}

pub struct RateLimiter {
    quota_checker: Arc<OrgQuotaChecker>,
}

impl RateLimiter {
    pub fn new(quota_checker: Arc<OrgQuotaChecker>) -> Self {
        Self { quota_checker }
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(Arc::new(OrgQuotaChecker::new()))
    }
}

fn count_calls(events: &[impl HasTimestamp], now: i64, window_hours: u32) -> u32 {
    let window_seconds = (window_hours as i64) * 3600;
    let cutoff = now - window_seconds;
    events.iter().filter(|e| e.timestamp() > cutoff).count() as u32
}

fn sum_tokens(events: &[impl HasTokens], now: i64, window_hours: u32) -> u64 {
    let window_seconds = (window_hours as i64) * 3600;
    let cutoff = now - window_seconds;
    events
        .iter()
        .filter(|e| e.timestamp() > cutoff)
        .map(|e| e.input_tokens() + e.output_tokens())
        .sum()
}

fn sum_characters(events: &[impl HasCharacters], now: i64, window_hours: u32) -> u64 {
    let window_seconds = (window_hours as i64) * 3600;
    let cutoff = now - window_seconds;
    events
        .iter()
        .filter(|e| e.timestamp() > cutoff)
        .map(|e| e.characters())
        .sum()
}

trait HasTimestamp {
    fn timestamp(&self) -> i64;
}

trait HasTokens: HasTimestamp {
    fn input_tokens(&self) -> u64;
    fn output_tokens(&self) -> u64;
}

trait HasCharacters: HasTimestamp {
    fn characters(&self) -> u64;
}

impl HasTimestamp for dialect_coach_shared::models::usage_stats::AgentUsage {
    fn timestamp(&self) -> i64 {
        self.timestamp
    }
}

impl HasTokens for dialect_coach_shared::models::usage_stats::AgentUsage {
    fn input_tokens(&self) -> u64 {
        self.input_tokens
    }
    fn output_tokens(&self) -> u64 {
        self.output_tokens
    }
}

impl HasTimestamp for dialect_coach_shared::models::usage_stats::TtsUsage {
    fn timestamp(&self) -> i64 {
        self.timestamp
    }
}

impl HasCharacters for dialect_coach_shared::models::usage_stats::TtsUsage {
    fn characters(&self) -> u64 {
        self.characters
    }
}

impl RateLimiterService for RateLimiter {
    fn can_make_response_call(&self, stats: &UsageStats, config: &RateLimitConfig) -> bool {
        let now = chrono::Utc::now().timestamp();
        let calls = count_calls(&stats.response_events, now, config.rolling_window_hours);
        let tokens = sum_tokens(&stats.response_events, now, config.rolling_window_hours);
        calls < config.response_calls_limit && tokens < config.response_tokens_limit
    }

    fn can_make_analysis_call(&self, stats: &UsageStats, config: &RateLimitConfig) -> bool {
        let now = chrono::Utc::now().timestamp();
        let calls = count_calls(&stats.analysis_events, now, config.rolling_window_hours);
        let tokens = sum_tokens(&stats.analysis_events, now, config.rolling_window_hours);
        calls < config.analysis_calls_limit && tokens < config.analysis_tokens_limit
    }

    fn can_make_tts_call(&self, stats: &UsageStats, config: &RateLimitConfig) -> bool {
        let now = chrono::Utc::now().timestamp();
        let calls = count_calls(&stats.tts_events, now, config.rolling_window_hours);
        let characters = sum_characters(&stats.tts_events, now, config.rolling_window_hours);
        calls < config.tts_calls_limit && characters < config.tts_characters_limit
    }

    async fn anthropic_has_quota(&self) -> bool {
        self.quota_checker.has_anthropic_quota().await
    }

    async fn elevenlabs_has_quota(&self) -> bool {
        self.quota_checker.has_elevenlabs_quota().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dialect_coach_shared::models::usage_stats::{AgentUsage, TtsUsage};

    #[test]
    fn test_can_make_response_call_within_limits() {
        let limiter = RateLimiter::default();
        let config = RateLimitConfig::default();
        let now = chrono::Utc::now().timestamp();

        let stats = UsageStats {
            response_events: vec![AgentUsage {
                timestamp: now - 100,
                input_tokens: 1000,
                output_tokens: 500,
                is_retry: false,
                is_estimate: false,
            }],
            analysis_events: vec![],
            tts_events: vec![],
        };

        assert!(limiter.can_make_response_call(&stats, &config));
    }

    #[test]
    fn test_can_make_response_call_exceeds_call_limit() {
        let limiter = RateLimiter::default();
        let mut config = RateLimitConfig::default();
        config.response_calls_limit = 2;
        let now = chrono::Utc::now().timestamp();

        let stats = UsageStats {
            response_events: vec![
                AgentUsage {
                    timestamp: now - 100,
                    input_tokens: 100,
                    output_tokens: 50,
                    is_retry: false,
                    is_estimate: false,
                },
                AgentUsage {
                    timestamp: now - 200,
                    input_tokens: 100,
                    output_tokens: 50,
                    is_retry: false,
                    is_estimate: false,
                },
            ],
            analysis_events: vec![],
            tts_events: vec![],
        };

        assert!(!limiter.can_make_response_call(&stats, &config));
    }

    #[test]
    fn test_can_make_response_call_exceeds_token_limit() {
        let limiter = RateLimiter::default();
        let mut config = RateLimitConfig::default();
        config.response_tokens_limit = 2000;
        let now = chrono::Utc::now().timestamp();

        let stats = UsageStats {
            response_events: vec![AgentUsage {
                timestamp: now - 100,
                input_tokens: 1500,
                output_tokens: 600,
                is_retry: false,
                is_estimate: false,
            }],
            analysis_events: vec![],
            tts_events: vec![],
        };

        assert!(!limiter.can_make_response_call(&stats, &config));
    }

    #[test]
    fn test_can_make_tts_call_within_limits() {
        let limiter = RateLimiter::default();
        let config = RateLimitConfig::default();
        let now = chrono::Utc::now().timestamp();

        let stats = UsageStats {
            response_events: vec![],
            analysis_events: vec![],
            tts_events: vec![TtsUsage {
                timestamp: now - 100,
                characters: 1000,
            }],
        };

        assert!(limiter.can_make_tts_call(&stats, &config));
    }

    #[tokio::test]
    async fn test_anthropic_quota_defaults_true() {
        let limiter = RateLimiter::default();
        assert!(limiter.anthropic_has_quota().await);
    }

    #[tokio::test]
    async fn test_elevenlabs_quota_defaults_true() {
        let limiter = RateLimiter::default();
        assert!(limiter.elevenlabs_has_quota().await);
    }
}
