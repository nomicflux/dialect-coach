use anyhow::{Result, anyhow, ensure};
use serde::Deserialize;
use std::collections::BTreeMap;

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
#[serde(try_from = "LlmConfigFile")]
pub struct LlmConfig {
    default_provider: Option<String>,
    providers: BTreeMap<String, ProviderConfig>,
    channels: BTreeMap<String, ChannelConfig>,
}

#[derive(Deserialize)]
struct LlmConfigFile {
    default_provider: Option<String>,
    providers: Option<BTreeMap<String, ProviderConfig>>,
    anthropic: Option<LegacyAnthropicConfig>,
    openai: Option<LegacyOpenAiConfig>,
    #[serde(default)]
    channels: BTreeMap<String, ChannelConfig>,
}

#[derive(Deserialize)]
struct LegacyAnthropicConfig {
    model: String,
}

#[derive(Deserialize)]
struct LegacyOpenAiConfig {
    model: String,
    reasoning_budget: u32,
}

#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderDriver {
    Anthropic,
    #[serde(rename = "openai")]
    OpenAi,
    Ollama,
}

impl ProviderDriver {
    fn default_base_url(self) -> &'static str {
        match self {
            Self::Anthropic => "https://api.anthropic.com",
            Self::OpenAi => "https://api.openai.com/v1",
            Self::Ollama => "http://localhost:11434",
        }
    }

    fn cloud_key_env(self) -> Option<&'static str> {
        match self {
            Self::Anthropic => Some("ANTHROPIC_API_KEY"),
            Self::OpenAi => Some("OPENAI_API_KEY"),
            Self::Ollama => None,
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
struct ProviderConfig {
    driver: ProviderDriver,
    model: String,
    base_url: Option<String>,
    api_key_env: Option<String>,
    reasoning_budget: Option<u32>,
    reasoning_effort: Option<String>,
}

#[derive(Deserialize, Clone, Debug)]
struct ChannelConfig {
    provider: Option<String>,
    model: Option<String>,
    reasoning_budget: Option<u32>,
    reasoning_effort: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReasoningConfig {
    pub budget: u32,
    pub effort: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedChannel {
    pub provider: String,
    pub driver: ProviderDriver,
    pub base_url: String,
    pub model: String,
    pub reasoning: Option<ReasoningConfig>,
    channel: String,
    channel_key_env: Option<String>,
    provider_key_env: Option<String>,
}

impl ResolvedChannel {
    pub fn api_key(&self, read_env: impl Fn(&str) -> Option<String>) -> Result<Option<String>> {
        let read_key = |name| read_env(name).filter(|key| !key.trim().is_empty());
        let key = self.channel_key_env.as_deref().and_then(read_key);
        if let (None, Some(name)) = (&key, self.provider_key_env.as_deref()) {
            return read_key(name).map(Some).ok_or_else(|| {
                anyhow!("Missing API key for {} channel (set {name})", self.channel)
            });
        }
        Ok(key)
    }
}

impl TryFrom<LlmConfigFile> for LlmConfig {
    type Error = anyhow::Error;

    fn try_from(file: LlmConfigFile) -> Result<Self> {
        let mut channels = file.channels;
        let providers = match file.providers {
            Some(providers) => {
                ensure!(
                    file.anthropic.is_none() && file.openai.is_none(),
                    "llm.providers cannot be combined with legacy llm.anthropic or llm.openai"
                );
                providers
            }
            None => {
                let anthropic = file
                    .anthropic
                    .ok_or_else(|| anyhow!("Missing llm.anthropic"))?;
                let openai = file.openai.ok_or_else(|| anyhow!("Missing llm.openai"))?;
                channels
                    .entry("keywords".into())
                    .or_insert_with(|| ChannelConfig {
                        provider: Some("openai".into()),
                        model: Some("gpt-5-nano".into()),
                        reasoning_budget: None,
                        reasoning_effort: Some("minimal".into()),
                    });
                BTreeMap::from([
                    (
                        "anthropic".into(),
                        ProviderConfig {
                            driver: ProviderDriver::Anthropic,
                            model: anthropic.model,
                            base_url: None,
                            api_key_env: None,
                            reasoning_budget: None,
                            reasoning_effort: None,
                        },
                    ),
                    (
                        "openai".into(),
                        ProviderConfig {
                            driver: ProviderDriver::OpenAi,
                            model: openai.model,
                            base_url: None,
                            api_key_env: None,
                            reasoning_budget: Some(openai.reasoning_budget),
                            reasoning_effort: None,
                        },
                    ),
                ])
            }
        };
        Ok(Self {
            default_provider: file.default_provider,
            providers,
            channels,
        })
    }
}

impl LlmConfig {
    pub fn resolve_channel(&self, name: &str) -> Result<ResolvedChannel> {
        let channel = self.channels.get(name);
        let selected_provider = channel.and_then(|channel| channel.provider.as_deref());
        let inherited_channel = if name == "ocr" && selected_provider.is_none() {
            "planning"
        } else {
            name
        };
        let inherited = if inherited_channel == name {
            None
        } else {
            self.channels.get(inherited_channel)
        };
        let provider_name = selected_provider
            .or_else(|| inherited.and_then(|channel| channel.provider.as_deref()))
            .or(self.default_provider.as_deref())
            .ok_or_else(|| anyhow!("No provider configured for {name} channel"))?;
        let provider = self
            .providers
            .get(provider_name)
            .ok_or_else(|| anyhow!("Unknown provider '{provider_name}' for {name} channel"))?;
        let overrides = [channel, inherited];
        let model = overrides
            .iter()
            .flatten()
            .find_map(|channel| channel.model.as_deref())
            .unwrap_or(&provider.model);
        ensure!(!model.trim().is_empty(), "Empty model for {name} channel");

        let base_url = provider
            .base_url
            .as_deref()
            .unwrap_or_else(|| provider.driver.default_base_url());
        let valid_url = url::Url::parse(base_url).is_ok_and(|url| {
            matches!(url.scheme(), "http" | "https")
                && url.host().is_some()
                && url.username().is_empty()
                && url.password().is_none()
                && url.query().is_none()
                && url.fragment().is_none()
        });
        ensure!(valid_url, "Invalid base_url for {name} channel");

        let reasoning = if provider.driver == ProviderDriver::OpenAi {
            let budget = overrides
                .iter()
                .flatten()
                .find_map(|channel| channel.reasoning_budget)
                .or(provider.reasoning_budget)
                .unwrap_or_default();
            let effort = overrides
                .iter()
                .flatten()
                .find_map(|channel| channel.reasoning_effort.as_deref())
                .or(provider.reasoning_effort.as_deref())
                .unwrap_or("low");
            ensure!(
                matches!(
                    effort,
                    "none" | "minimal" | "low" | "medium" | "high" | "xhigh" | "max"
                ),
                "Invalid reasoning_effort for {name} channel"
            );
            Some(ReasoningConfig {
                budget,
                effort: effort.into(),
            })
        } else {
            None
        };

        let cloud_key_env = provider.driver.cloud_key_env();
        let provider_key_env = provider.api_key_env.as_deref().or(cloud_key_env);
        ensure!(
            provider_key_env
                .is_none_or(|name| !name.trim().is_empty() && !name.contains(['=', '\0'])),
            "Invalid api_key_env for {name} channel"
        );

        Ok(ResolvedChannel {
            provider: provider_name.into(),
            driver: provider.driver,
            base_url: base_url.into(),
            model: model.into(),
            reasoning,
            channel: name.into(),
            channel_key_env: cloud_key_env
                .map(|_| format!("{}_API_KEY", inherited_channel.to_ascii_uppercase())),
            provider_key_env: provider_key_env.map(str::to_owned),
        })
    }
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
        assert_eq!(
            config.llm.resolve_channel("response").unwrap().driver,
            ProviderDriver::Anthropic
        );
        assert_eq!(config.tts.eleven_labs.model, "eleven_multilingual_v2");
        assert_eq!(config.rate_limits.response_calls, 100);
        assert_eq!(config.rate_limits.window_hours, 24);
    }

    const LEGACY_HOSTED: &str = r#"
anthropic:
  model: claude-haiku-4-5
openai:
  model: gpt-5.4
  reasoning_budget: 512
channels:
  response: { provider: openai, model: gpt-5.4 }
  learning: { provider: anthropic, model: claude-sonnet-4-6 }
  analysis: { provider: anthropic, model: claude-haiku-4-5 }
  pronunciation: { provider: openai, model: gpt-5.4, reasoning_effort: none }
  planning: { provider: anthropic, model: claude-sonnet-4-6 }
"#;

    const LOCAL: &str = r#"
default_provider: local
providers:
  local:
    driver: ollama
    base_url: http://localhost:11434
    model: qwen3:8b
channels:
  ocr:
    model: qwen3-vl:8b
"#;

    #[test]
    fn hosted_migration_preserves_effective_channel_settings() {
        let old: LlmConfig = serde_yaml::from_str(LEGACY_HOSTED).unwrap();
        let migrated = load_config(concat!(env!("CARGO_MANIFEST_DIR"), "/../config.yaml")).unwrap();
        for (name, driver, model, effort) in [
            ("response", ProviderDriver::OpenAi, "gpt-5.4", Some("low")),
            (
                "learning",
                ProviderDriver::Anthropic,
                "claude-sonnet-4-6",
                None,
            ),
            (
                "analysis",
                ProviderDriver::Anthropic,
                "claude-haiku-4-5",
                None,
            ),
            (
                "pronunciation",
                ProviderDriver::OpenAi,
                "gpt-5.4",
                Some("none"),
            ),
            (
                "planning",
                ProviderDriver::Anthropic,
                "claude-sonnet-4-6",
                None,
            ),
            (
                "keywords",
                ProviderDriver::OpenAi,
                "gpt-5-nano",
                Some("minimal"),
            ),
            ("ocr", ProviderDriver::Anthropic, "claude-sonnet-4-6", None),
        ] {
            let channel = migrated.llm.resolve_channel(name).unwrap();
            assert_eq!(old.resolve_channel(name).unwrap(), channel, "{name}");
            assert_eq!(channel.driver, driver, "{name}");
            assert_eq!(channel.model, model, "{name}");
            assert_eq!(
                channel.reasoning,
                effort.map(|effort| ReasoningConfig {
                    budget: 512,
                    effort: effort.into()
                }),
                "{name}"
            );
        }
    }

    #[test]
    fn local_channels_inherit_ollama_without_reading_credentials() {
        let config: LlmConfig = serde_yaml::from_str(LOCAL).unwrap();
        for name in [
            "response",
            "learning",
            "analysis",
            "pronunciation",
            "planning",
            "keywords",
            "ocr",
        ] {
            let channel = config.resolve_channel(name).unwrap();
            assert_eq!(channel.provider, "local");
            assert_eq!(channel.driver, ProviderDriver::Ollama);
            assert_eq!(channel.base_url, "http://localhost:11434");
            assert_eq!(
                channel.model,
                if name == "ocr" {
                    "qwen3-vl:8b"
                } else {
                    "qwen3:8b"
                }
            );
            assert_eq!(channel.reasoning, None);
            assert_eq!(
                channel
                    .api_key(|key| panic!("unexpected lookup of {key}"))
                    .unwrap(),
                None
            );
        }
    }

    #[test]
    fn mixed_ocr_inherits_resolved_planning_until_its_provider_is_explicit() {
        let yaml = r#"
default_provider: local
providers:
  local: { driver: ollama, model: local-default }
  hosted: { driver: openai, model: hosted-default, reasoning_budget: 512 }
channels:
  planning: { provider: hosted, model: planning-override, reasoning_budget: 256 }
"#;
        let read_key = |name: &str| (name == "PLANNING_API_KEY").then(|| "planning-key".into());
        let mut config: LlmConfig = serde_yaml::from_str(yaml).unwrap();
        let planning = config.resolve_channel("planning").unwrap();
        assert_eq!(planning.model, "planning-override");
        assert_eq!(planning.reasoning.as_ref().unwrap().budget, 256);
        let inherited = config.resolve_channel("ocr").unwrap();
        assert_eq!(inherited.driver, planning.driver);
        assert_eq!(inherited.base_url, planning.base_url);
        assert_eq!(inherited.model, planning.model);
        assert_eq!(inherited.reasoning, planning.reasoning);
        assert_eq!(
            inherited.api_key(read_key).unwrap().as_deref(),
            Some("planning-key")
        );

        config.channels.insert(
            "ocr".into(),
            serde_yaml::from_str("model: vision-override").unwrap(),
        );
        let overridden = config.resolve_channel("ocr").unwrap();
        let mut expected = inherited;
        expected.model = "vision-override".into();
        assert_eq!(overridden, expected);

        config.channels.insert(
            "ocr".into(),
            serde_yaml::from_str("provider: local").unwrap(),
        );
        let explicit = config.resolve_channel("ocr").unwrap();
        assert_eq!(explicit.driver, ProviderDriver::Ollama);
        assert_eq!(explicit.model, "local-default");
        assert_eq!(
            explicit
                .api_key(|key| panic!("unexpected lookup of {key}"))
                .unwrap(),
            None
        );
    }

    #[test]
    fn cloud_key_precedence_and_configuration_errors_name_the_channel() {
        let config: LlmConfig = serde_yaml::from_str(LEGACY_HOSTED).unwrap();
        let response = config.resolve_channel("response").unwrap();
        let provider_key = |name: &str| (name == "OPENAI_API_KEY").then(|| "provider-key".into());
        assert_eq!(
            response
                .api_key(|name| {
                    if name == "RESPONSE_API_KEY" {
                        Some("channel-key".into())
                    } else {
                        provider_key(name)
                    }
                })
                .unwrap()
                .as_deref(),
            Some("channel-key")
        );
        assert_eq!(
            response.api_key(provider_key).unwrap().as_deref(),
            Some("provider-key")
        );
        let error = response.api_key(|_| None).unwrap_err().to_string();
        assert!(error.contains("response channel") && error.contains("OPENAI_API_KEY"));

        let unknown = LEGACY_HOSTED.replace(
            "provider: openai, model: gpt-5.4",
            "provider: missing, model: gpt-5.4",
        );
        let config: LlmConfig = serde_yaml::from_str(&unknown).unwrap();
        let error = config.resolve_channel("response").unwrap_err().to_string();
        assert!(error.contains("response channel") && error.contains("'missing'"));

        let invalid = LOCAL.replace("http://localhost:11434", "file:///tmp/models");
        let config: LlmConfig = serde_yaml::from_str(&invalid).unwrap();
        assert_eq!(
            config.resolve_channel("response").unwrap_err().to_string(),
            "Invalid base_url for response channel"
        );
    }

    #[test]
    fn ollama_uses_an_explicit_credential_reference() {
        let secured = LOCAL.replace(
            "    model: qwen3:8b",
            "    model: qwen3:8b\n    api_key_env: LOCAL_TOKEN",
        );
        let config: LlmConfig = serde_yaml::from_str(&secured).unwrap();
        let response = config.resolve_channel("response").unwrap();
        assert_eq!(
            response
                .api_key(|name| {
                    assert_eq!(name, "LOCAL_TOKEN");
                    Some("local-test-token".into())
                })
                .unwrap()
                .as_deref(),
            Some("local-test-token")
        );
        assert!(
            response
                .api_key(|_| None)
                .unwrap_err()
                .to_string()
                .contains("response channel")
        );
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
