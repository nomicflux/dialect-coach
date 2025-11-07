use std::env;

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
    pub fn from_env() -> Self {
        Self {
            response_calls_limit: parse_env("RATE_LIMIT_RESPONSE_CALLS", 100),
            response_tokens_limit: parse_env("RATE_LIMIT_RESPONSE_TOKENS", 100_000),
            analysis_calls_limit: parse_env("RATE_LIMIT_ANALYSIS_CALLS", 100),
            analysis_tokens_limit: parse_env("RATE_LIMIT_ANALYSIS_TOKENS", 50_000),
            tts_calls_limit: parse_env("RATE_LIMIT_TTS_CALLS", 200),
            tts_characters_limit: parse_env("RATE_LIMIT_TTS_CHARACTERS", 50_000),
            rolling_window_hours: parse_env("RATE_LIMIT_WINDOW_HOURS", 24),
        }
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            response_calls_limit: 100,
            response_tokens_limit: 100_000,
            analysis_calls_limit: 100,
            analysis_tokens_limit: 50_000,
            tts_calls_limit: 200,
            tts_characters_limit: 50_000,
            rolling_window_hours: 24,
        }
    }
}

fn parse_env<T: std::str::FromStr>(key: &str, default: T) -> T {
    env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = RateLimitConfig::default();
        assert_eq!(config.response_calls_limit, 100);
        assert_eq!(config.response_tokens_limit, 100_000);
        assert_eq!(config.analysis_calls_limit, 100);
        assert_eq!(config.analysis_tokens_limit, 50_000);
        assert_eq!(config.tts_calls_limit, 200);
        assert_eq!(config.tts_characters_limit, 50_000);
        assert_eq!(config.rolling_window_hours, 24);
    }

    #[test]
    fn test_from_env_uses_defaults_when_no_env() {
        unsafe {
            env::remove_var("RATE_LIMIT_RESPONSE_CALLS");
            env::remove_var("RATE_LIMIT_WINDOW_HOURS");
        }
        let config = RateLimitConfig::from_env();
        assert_eq!(config.response_calls_limit, 100);
        assert_eq!(config.rolling_window_hours, 24);
    }

    #[test]
    fn test_from_env_reads_env_vars() {
        unsafe {
            env::remove_var("RATE_LIMIT_RESPONSE_CALLS");
            env::remove_var("RATE_LIMIT_WINDOW_HOURS");
            env::set_var("RATE_LIMIT_RESPONSE_CALLS", "50");
            env::set_var("RATE_LIMIT_WINDOW_HOURS", "12");
        }

        let config = RateLimitConfig::from_env();
        assert_eq!(config.response_calls_limit, 50);
        assert_eq!(config.rolling_window_hours, 12);

        unsafe {
            env::remove_var("RATE_LIMIT_RESPONSE_CALLS");
            env::remove_var("RATE_LIMIT_WINDOW_HOURS");
        }
    }

    #[test]
    fn test_from_env_ignores_invalid_values() {
        unsafe {
            env::remove_var("RATE_LIMIT_RESPONSE_CALLS");
            env::set_var("RATE_LIMIT_RESPONSE_CALLS", "invalid");
        }

        let config = RateLimitConfig::from_env();
        assert_eq!(config.response_calls_limit, 100);

        unsafe {
            env::remove_var("RATE_LIMIT_RESPONSE_CALLS");
        }
    }
}
