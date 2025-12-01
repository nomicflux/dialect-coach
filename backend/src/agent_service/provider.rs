use anyhow::{Result, anyhow};
use async_trait::async_trait;
use rig::client::completion::CompletionClient;
use rig::completion::message::AssistantContent;
use rig::completion::{
    CompletionError, CompletionModel, CompletionRequest as RigCompletionRequest,
    Message as RigMessage,
};
use rig::one_or_many::OneOrMany;
use rig::providers::anthropic::completion::CompletionModel as AnthropicCompletionModel;
use rig::providers::anthropic::{CLAUDE_3_5_SONNET, Client as AnthropicClient};
use rig::providers::openai::responses_api::ResponsesCompletionModel as OpenAICompletionModel;
use rig::providers::openai::{Client as OpenAIClient, GPT_4O};

pub const ANTHROPIC_PROVIDER: &str = "anthropic";
pub const OPENAI_PROVIDER: &str = "openai";

#[derive(Debug, Clone)]
pub struct CompletionRequest<'a> {
    pub preamble: &'a str,
    pub prompt: &'a str,
    pub history: &'a [RigMessage],
    pub max_tokens: u64,
    pub temperature: f64,
}

#[derive(Debug, Clone)]
pub struct CompletionOutcome {
    pub text: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum CompletionAgentError {
    #[error("{0}")]
    Retryable(anyhow::Error),
    #[error("{0}")]
    Fatal(anyhow::Error),
}

impl CompletionAgentError {
    pub fn retryable(message: impl Into<anyhow::Error>) -> Self {
        CompletionAgentError::Retryable(message.into())
    }

    pub fn fatal(message: impl Into<anyhow::Error>) -> Self {
        CompletionAgentError::Fatal(message.into())
    }
}

#[derive(Clone)]
pub(crate) enum ProviderCompletionModel {
    Anthropic(AnthropicCompletionModel),
    OpenAI(OpenAICompletionModel),
}

#[async_trait]
pub trait CompletionAgent: Send + Sync {
    fn provider(&self) -> &str;
    fn model(&self) -> &str;

    async fn completion(
        &self,
        request: &CompletionRequest<'_>,
    ) -> Result<CompletionOutcome, CompletionAgentError>;
}

#[derive(Debug, Clone)]
pub struct ProviderAgentConfig {
    pub provider: String,
    pub model: String,
    pub api_key: String,
    pub reasoning_budget: u32,
}

impl ProviderAgentConfig {
    pub fn anthropic(api_key: String, model: Option<String>, reasoning_budget: u32) -> Self {
        ProviderAgentConfig {
            provider: ANTHROPIC_PROVIDER.to_string(),
            model: model.unwrap_or_else(|| CLAUDE_3_5_SONNET.to_string()),
            api_key,
            reasoning_budget,
        }
    }

    pub fn openai(api_key: String, model: Option<String>, reasoning_budget: u32) -> Self {
        ProviderAgentConfig {
            provider: OPENAI_PROVIDER.to_string(),
            model: model.unwrap_or_else(|| GPT_4O.to_string()),
            api_key,
            reasoning_budget,
        }
    }
}

#[derive(Clone)]
pub struct UnifiedCompletionAgent {
    completion_model: ProviderCompletionModel,
    model_name: String,
    provider_name: String,
    reasoning_budget: u32,
}

impl UnifiedCompletionAgent {
    pub(crate) fn new(
        completion_model: ProviderCompletionModel,
        model_name: String,
        provider_name: String,
        reasoning_budget: u32,
    ) -> Self {
        Self {
            completion_model,
            model_name,
            provider_name,
            reasoning_budget,
        }
    }

    fn extract_text(
        &self,
        choice: &OneOrMany<AssistantContent>,
    ) -> Result<String, CompletionAgentError> {
        let mut text = String::new();
        for piece in choice.iter() {
            if let AssistantContent::Text(t) = piece {
                text.push_str(&t.text);
            }
        }
        if text.is_empty() {
            return Err(CompletionAgentError::fatal(anyhow!("No text in response")));
        }
        Ok(text)
    }

    fn categorize_error(&self, error: CompletionError) -> CompletionAgentError {
        match error {
            CompletionError::ProviderError(msg) | CompletionError::ResponseError(msg) => {
                let message = msg;
                tracing::warn!(
                    provider = %self.provider_name,
                    model = %self.model_name,
                    "Retryable provider error: {}",
                    message
                );
                CompletionAgentError::retryable(anyhow!(message))
            }
            other => {
                let message = other.to_string();
                tracing::error!(
                    provider = %self.provider_name,
                    model = %self.model_name,
                    "Fatal provider error: {}",
                    message
                );
                CompletionAgentError::fatal(anyhow!(message))
            }
        }
    }

    fn provider_supports_temperature(&self) -> bool {
        self.provider_name != OPENAI_PROVIDER
    }

    fn build_completion_request(
        &self,
        request: &CompletionRequest<'_>,
        include_temperature: bool,
    ) -> RigCompletionRequest {
        let mut chat_history = request.history.to_vec();
        chat_history.push(RigMessage::User {
            content: OneOrMany::one(rig::completion::message::UserContent::Text(
                rig::completion::message::Text {
                    text: request.prompt.to_string(),
                },
            )),
        });

        let mut rig_request = RigCompletionRequest {
            preamble: Some(request.preamble.to_string()),
            chat_history: OneOrMany::many(chat_history).expect("chat_history should not be empty"),
            documents: vec![],
            tools: vec![],
            temperature: None,
            max_tokens: Some(request.max_tokens),
            tool_choice: None,
            additional_params: None,
        };

        if include_temperature {
            rig_request.temperature = Some(request.temperature);
        }

        rig_request
    }
}

#[async_trait]
impl CompletionAgent for UnifiedCompletionAgent {
    fn provider(&self) -> &str {
        &self.provider_name
    }

    fn model(&self) -> &str {
        &self.model_name
    }

    async fn completion(
        &self,
        request: &CompletionRequest<'_>,
    ) -> Result<CompletionOutcome, CompletionAgentError> {
        match &self.completion_model {
            ProviderCompletionModel::Anthropic(model) => {
                let rig_request =
                    self.build_completion_request(request, self.provider_supports_temperature());
                let response = model
                    .completion(rig_request)
                    .await
                    .map_err(|err| self.categorize_error(err))?;
                let text = self.extract_text(&response.choice)?;
                Ok(CompletionOutcome {
                    text,
                    input_tokens: response.usage.input_tokens,
                    output_tokens: response.usage.output_tokens,
                })
            }
            ProviderCompletionModel::OpenAI(model) => {
                let mut rig_request = self.build_completion_request(request, false);
                let actual_max_tokens = request.max_tokens + self.reasoning_budget as u64;
                rig_request.max_tokens = Some(actual_max_tokens);
                tracing::debug!(
                    "OpenAI reasoning budget: adding {} tokens to max_tokens={}",
                    self.reasoning_budget,
                    request.max_tokens
                );
                rig_request.additional_params = Some(serde_json::json!({
                    "reasoning": {
                        "effort": "low"
                    }
                }));
                let response = model
                    .completion(rig_request)
                    .await
                    .map_err(|err| self.categorize_error(err))?;
                let text = match self.extract_text(&response.choice) {
                    Ok(t) => t,
                    Err(e) => {
                        tracing::error!(
                            provider = %self.provider_name,
                            model = %self.model_name,
                            raw_response = ?response,
                            "OpenAI response missing text content"
                        );
                        return Err(e);
                    }
                };
                Ok(CompletionOutcome {
                    text,
                    input_tokens: response.usage.input_tokens,
                    output_tokens: response.usage.output_tokens,
                })
            }
        }
    }
}

pub struct CompletionAgentFactory;

impl CompletionAgentFactory {
    pub fn build(config: ProviderAgentConfig) -> Result<Box<dyn CompletionAgent>> {
        let (completion_model, provider_name) = match config.provider.as_str() {
            ANTHROPIC_PROVIDER => {
                let client = AnthropicClient::new(&config.api_key);
                let model = client.completion_model(&config.model);
                (
                    ProviderCompletionModel::Anthropic(model),
                    ANTHROPIC_PROVIDER.to_string(),
                )
            }
            OPENAI_PROVIDER => {
                let client = OpenAIClient::new(&config.api_key);
                let model = client.completion_model(&config.model);
                (
                    ProviderCompletionModel::OpenAI(model),
                    OPENAI_PROVIDER.to_string(),
                )
            }
            other => return Err(anyhow!("Unsupported provider: {}", other)),
        };

        let agent = UnifiedCompletionAgent::new(
            completion_model,
            config.model,
            provider_name,
            config.reasoning_budget,
        );
        Ok(Box::new(agent))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_agent_config_openai_default_model() {
        let config = ProviderAgentConfig::openai("sk-test-key".to_string(), None, 200);
        assert_eq!(config.provider, OPENAI_PROVIDER);
        assert_eq!(config.model, GPT_4O);
        assert_eq!(config.api_key, "sk-test-key");
        assert_eq!(config.reasoning_budget, 200);
    }

    #[test]
    fn test_provider_agent_config_openai_custom_model() {
        let config = ProviderAgentConfig::openai(
            "sk-test-key".to_string(),
            Some("gpt-4-turbo".to_string()),
            300,
        );
        assert_eq!(config.provider, OPENAI_PROVIDER);
        assert_eq!(config.model, "gpt-4-turbo");
        assert_eq!(config.api_key, "sk-test-key");
        assert_eq!(config.reasoning_budget, 300);
    }

    #[test]
    fn test_provider_agent_config_anthropic_default_model() {
        let config = ProviderAgentConfig::anthropic("test-key".to_string(), None, 200);
        assert_eq!(config.provider, ANTHROPIC_PROVIDER);
        assert_eq!(config.model, CLAUDE_3_5_SONNET);
        assert_eq!(config.api_key, "test-key");
        assert_eq!(config.reasoning_budget, 200);
    }

    #[test]
    fn test_provider_agent_config_anthropic_custom_model() {
        let config = ProviderAgentConfig::anthropic(
            "test-key".to_string(),
            Some("claude-3-opus".to_string()),
            250,
        );
        assert_eq!(config.provider, ANTHROPIC_PROVIDER);
        assert_eq!(config.model, "claude-3-opus");
        assert_eq!(config.api_key, "test-key");
        assert_eq!(config.reasoning_budget, 250);
    }

    #[test]
    fn test_completion_agent_factory_anthropic() {
        let config = ProviderAgentConfig::anthropic("test-key".to_string(), None, 200);
        let result = CompletionAgentFactory::build(config);
        assert!(result.is_ok());
        let agent = result.unwrap();
        assert_eq!(agent.provider(), ANTHROPIC_PROVIDER);
    }

    #[test]
    fn test_completion_agent_factory_openai() {
        let config = ProviderAgentConfig::openai("sk-test-key".to_string(), None, 200);
        let result = CompletionAgentFactory::build(config);
        assert!(result.is_ok());
        let agent = result.unwrap();
        assert_eq!(agent.provider(), OPENAI_PROVIDER);
    }

    #[test]
    fn test_completion_agent_factory_unsupported_provider() {
        let config = ProviderAgentConfig {
            provider: "unsupported".to_string(),
            model: "test-model".to_string(),
            api_key: "test-key".to_string(),
            reasoning_budget: 200,
        };
        let result = CompletionAgentFactory::build(config);
        assert!(result.is_err());
        match result {
            Err(err) => assert!(err.to_string().contains("Unsupported provider")),
            Ok(_) => panic!("Expected error"),
        }
    }

    #[test]
    fn test_unified_completion_agent_uses_reasoning_budget() {
        let config = ProviderAgentConfig::openai("sk-test-key".to_string(), None, 350);
        let result = CompletionAgentFactory::build(config);
        assert!(result.is_ok());
        let _agent = result.unwrap();
    }

    #[test]
    fn test_provider_agent_config_stores_reasoning_budget() {
        let config = ProviderAgentConfig::openai("sk-test-key".to_string(), None, 275);
        assert_eq!(config.reasoning_budget, 275);
    }
}
