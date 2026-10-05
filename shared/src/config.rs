use anyhow::Result;
use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub persistence: PersistenceConfig,
    pub llm: LlmConfig,
    pub tts: TtsConfig,
    pub rate_limits: RateLimitsConfig,
}

#[derive(Deserialize, Clone, Debug)]
pub struct ServerConfig {
    pub bind_address: String,
    pub backend_url: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct PersistenceConfig {
    pub db_path: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct LlmConfig {
    pub anthropic: AnthropicProviderConfig,
    pub openai: OpenAiProviderConfig,
    pub channels: ChannelsConfig,
}

#[derive(Deserialize, Clone, Debug)]
pub struct AnthropicProviderConfig {
    pub model: String,
    pub org_id: Option<String>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct OpenAiProviderConfig {
    pub model: String,
    pub reasoning_budget: u32,
}

#[derive(Deserialize, Clone, Debug)]
pub struct ChannelConfig {
    pub provider: String,
    pub model: Option<String>,
    pub reasoning_budget: Option<u32>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct ChannelsConfig {
    pub response: ChannelConfig,
    pub learning: ChannelConfig,
    pub analysis: ChannelConfig,
    pub pronunciation: ChannelConfig,
    pub planning: ChannelConfig,
}

#[derive(Deserialize, Clone, Debug)]
pub struct TtsConfig {
    pub eleven_labs: ElevenLabsTtsConfig,
    pub azure: AzureTtsConfig,
}

#[derive(Deserialize, Clone, Debug)]
pub struct ElevenLabsTtsConfig {
    pub url: String,
    pub model: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct AzureTtsConfig {
    pub region: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct RateLimitsConfig {
    pub response_calls: u32,
    pub response_tokens: u64,
    pub analysis_calls: u32,
    pub analysis_tokens: u64,
    pub tts_calls: u32,
    pub tts_characters: u64,
    pub window_hours: u32,
}

pub fn load_config(path: &str) -> Result<AppConfig> {
    let contents = std::fs::read_to_string(path)?;
    let config: AppConfig = serde_yaml::from_str(&contents)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_YAML: &str = r#"
server:
  bind_address: "0.0.0.0:3000"
  backend_url: "http://localhost:3000"

persistence:
  db_path: "data/dialect-coach.db"

llm:
  anthropic:
    model: "claude-haiku-4-5"
    org_id: ~
  openai:
    model: "gpt-4o"
    reasoning_budget: 512
  channels:
    response:
      provider: "anthropic"
    learning:
      provider: "anthropic"
    analysis:
      provider: "anthropic"
    pronunciation:
      provider: "anthropic"
    planning:
      provider: "anthropic"

tts:
  eleven_labs:
    url: "https://api.elevenlabs.io/v1/"
    model: "eleven_multilingual_v2"
  azure:
    region: "eastus"

rate_limits:
  response_calls: 100
  response_tokens: 100000
  analysis_calls: 100
  analysis_tokens: 50000
  tts_calls: 200
  tts_characters: 50000
  window_hours: 24
"#;

    #[test]
    fn test_deserialize_full_config() {
        let config: AppConfig = serde_yaml::from_str(TEST_YAML).unwrap();
        assert_eq!(config.server.bind_address, "0.0.0.0:3000");
        assert_eq!(config.server.backend_url, "http://localhost:3000");
        assert_eq!(config.persistence.db_path, "data/dialect-coach.db");
        assert_eq!(config.llm.openai.reasoning_budget, 512);
        assert_eq!(config.llm.channels.response.provider, "anthropic");
        assert_eq!(config.tts.eleven_labs.model, "eleven_multilingual_v2");
        assert_eq!(config.rate_limits.response_calls, 100);
        assert_eq!(config.rate_limits.window_hours, 24);
    }

    #[test]
    fn test_optional_fields_deserialize_as_none() {
        let config: AppConfig = serde_yaml::from_str(TEST_YAML).unwrap();
        assert!(config.llm.anthropic.org_id.is_none());
        assert!(config.llm.channels.response.model.is_none());
        assert!(config.llm.channels.response.reasoning_budget.is_none());
    }

    #[test]
    fn test_optional_fields_deserialize_with_values() {
        let yaml = r#"
server:
  bind_address: "0.0.0.0:3000"
  backend_url: "http://localhost:3000"
persistence:
  db_path: "data/test.db"
llm:
  anthropic:
    model: "claude-sonnet-4-5"
    org_id: "org-123"
  openai:
    model: "gpt-4o"
    reasoning_budget: 1024
  channels:
    response:
      provider: "openai"
      model: "gpt-4o-mini"
      reasoning_budget: 256
    learning:
      provider: "anthropic"
    analysis:
      provider: "anthropic"
    pronunciation:
      provider: "anthropic"
    planning:
      provider: "openai"
tts:
  eleven_labs:
    url: "https://api.elevenlabs.io/v1/"
    model: "eleven_multilingual_v2"
  azure:
    region: "westus"
rate_limits:
  response_calls: 50
  response_tokens: 50000
  analysis_calls: 50
  analysis_tokens: 25000
  tts_calls: 100
  tts_characters: 25000
  window_hours: 12
"#;
        let config: AppConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.llm.anthropic.model, "claude-sonnet-4-5");
        assert_eq!(config.llm.anthropic.org_id.as_deref(), Some("org-123"));
        assert_eq!(config.llm.openai.model, "gpt-4o");
        assert_eq!(
            config.llm.channels.response.model.as_deref(),
            Some("gpt-4o-mini")
        );
        assert_eq!(config.llm.channels.response.reasoning_budget, Some(256));
    }

    #[test]
    fn test_load_config_from_file() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let config_path = format!("{}/../config.yaml", manifest_dir);
        let config = load_config(&config_path).unwrap();
        assert_eq!(config.server.bind_address, "0.0.0.0:3000");
        assert_eq!(config.rate_limits.window_hours, 24);
    }
}
