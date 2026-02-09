#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub response_calls_limit: u32,
    pub response_tokens_limit: u64,
    pub analysis_calls_limit: u32,
    pub analysis_tokens_limit: u64,
    pub tts_calls_limit: u32,
    pub tts_characters_limit: u64,
    pub rolling_window_hours: u32,
}

impl RateLimitConfig {
    pub fn new(
        response_calls_limit: u32,
        response_tokens_limit: u64,
        analysis_calls_limit: u32,
        analysis_tokens_limit: u64,
        tts_calls_limit: u32,
        tts_characters_limit: u64,
        rolling_window_hours: u32,
    ) -> Self {
        Self {
            response_calls_limit,
            response_tokens_limit,
            analysis_calls_limit,
            analysis_tokens_limit,
            tts_calls_limit,
            tts_characters_limit,
            rolling_window_hours,
        }
    }

    pub fn from_yaml_config(c: &dialect_coach_shared::config::RateLimitsConfig) -> Self {
        Self::new(
            c.response_calls,
            c.response_tokens,
            c.analysis_calls,
            c.analysis_tokens,
            c.tts_calls,
            c.tts_characters,
            c.window_hours,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_yaml_config() {
        let yaml_config = dialect_coach_shared::config::RateLimitsConfig {
            response_calls: 50,
            response_tokens: 50_000,
            analysis_calls: 25,
            analysis_tokens: 25_000,
            tts_calls: 100,
            tts_characters: 10_000,
            window_hours: 12,
        };
        let config = RateLimitConfig::from_yaml_config(&yaml_config);
        assert_eq!(config.response_calls_limit, 50);
        assert_eq!(config.response_tokens_limit, 50_000);
        assert_eq!(config.analysis_calls_limit, 25);
        assert_eq!(config.analysis_tokens_limit, 25_000);
        assert_eq!(config.tts_calls_limit, 100);
        assert_eq!(config.tts_characters_limit, 10_000);
        assert_eq!(config.rolling_window_hours, 12);
    }
}
