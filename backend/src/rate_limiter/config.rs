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

    pub fn from_fetch<F>(mut fetch: F) -> Self
    where
        F: FnMut(&str) -> Option<String>,
    {
        Self::new(
            parse_value("RATE_LIMIT_RESPONSE_CALLS", 100, &mut fetch),
            parse_value("RATE_LIMIT_RESPONSE_TOKENS", 100_000, &mut fetch),
            parse_value("RATE_LIMIT_ANALYSIS_CALLS", 100, &mut fetch),
            parse_value("RATE_LIMIT_ANALYSIS_TOKENS", 50_000, &mut fetch),
            parse_value("RATE_LIMIT_TTS_CALLS", 200, &mut fetch),
            parse_value("RATE_LIMIT_TTS_CHARACTERS", 50_000, &mut fetch),
            parse_value("RATE_LIMIT_WINDOW_HOURS", 24, &mut fetch),
        )
    }

    pub fn from_env() -> Self {
        Self::from_fetch(|key| env::var(key).ok())
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

fn parse_value<T: std::str::FromStr, F: FnMut(&str) -> Option<String>>(
    key: &str,
    default: T,
    fetch: &mut F,
) -> T {
    fetch(key).and_then(|v| v.parse().ok()).unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

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
    fn test_from_fetch_uses_defaults_when_missing() {
        let values: HashMap<String, String> = HashMap::new();
        let config = RateLimitConfig::from_fetch(|key| values.get(key).cloned());
        assert_eq!(config.response_calls_limit, 100);
        assert_eq!(config.rolling_window_hours, 24);
    }

    #[test]
    fn test_from_fetch_reads_values() {
        let mut values: HashMap<String, String> = HashMap::new();
        values.insert("RATE_LIMIT_RESPONSE_CALLS".into(), "50".into());
        values.insert("RATE_LIMIT_WINDOW_HOURS".into(), "12".into());

        let config = RateLimitConfig::from_fetch(|key| values.get(key).cloned());
        assert_eq!(config.response_calls_limit, 50);
        assert_eq!(config.rolling_window_hours, 12);
    }

    #[test]
    fn test_from_fetch_ignores_invalid_values() {
        let mut values: HashMap<String, String> = HashMap::new();
        values.insert("RATE_LIMIT_RESPONSE_CALLS".into(), "invalid".into());

        let config = RateLimitConfig::from_fetch(|key| values.get(key).cloned());
        assert_eq!(config.response_calls_limit, 100);
    }
}
