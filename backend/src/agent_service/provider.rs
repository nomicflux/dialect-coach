use anyhow::{Result, anyhow};
use async_trait::async_trait;
use rig::completion::message::AssistantContent;
use rig::completion::{
    CompletionModel, CompletionRequest as RigCompletionRequest, CompletionError,
    CompletionResponse as RigCompletionResponse, Message as RigMessage,
};
use rig::providers::anthropic::completion::{
    CompletionModel as AnthropicCompletionModel, CompletionResponse as AnthropicCompletionResponse,
};
use rig::providers::anthropic::{CLAUDE_3_5_SONNET, ClientBuilder};
use rig::providers::openai::{
    Client as OpenAIClient, CompletionModel as OpenAICompletionModel,
    CompletionResponse as OpenAICompletionResponse, GPT_4O,
};

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

pub(crate) enum ProviderResponse {
    OpenAI(OpenAICompletionResponse),
    Anthropic(AnthropicCompletionResponse),
}

impl ProviderResponse {
    fn input_tokens(&self) -> u64 {
        match self {
            Self::Anthropic(r) => r.usage.input_tokens,
            Self::OpenAI(r) => r.usage
                .as_ref()
                .map(|u| u.prompt_tokens as u64)
                .unwrap_or(0),
        }
    }

    fn output_tokens(&self) -> u64 {
        match self {
            Self::Anthropic(r) => r.usage.output_tokens,
            Self::OpenAI(r) => r.usage
                .as_ref()
                .map(|u| (u.total_tokens - u.prompt_tokens) as u64)
                .unwrap_or(0),
        }
    }
}

#[derive(Clone)]
pub(crate) enum ProviderCompletionModel {
    Anthropic(AnthropicCompletionModel),
    OpenAI(OpenAICompletionModel),
}

impl CompletionModel for ProviderCompletionModel {
    type Response = ProviderResponse;

    async fn completion(
        &self,
        request: RigCompletionRequest,
    ) -> Result<RigCompletionResponse<ProviderResponse>, CompletionError> {
        match self {
            Self::Anthropic(model) => {
                let response = model.completion(request).await?;
                Ok(RigCompletionResponse {
                    choice: response.choice,
                    raw_response: ProviderResponse::Anthropic(response.raw_response),
                })
            }
            Self::OpenAI(model) => {
                let response = model.completion(request).await?;
                Ok(RigCompletionResponse {
                    choice: response.choice,
                    raw_response: ProviderResponse::OpenAI(response.raw_response),
                })
            }
        }
    }
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
}

impl ProviderAgentConfig {
    pub fn anthropic(api_key: String, model: Option<String>) -> Self {
        ProviderAgentConfig {
            provider: ANTHROPIC_PROVIDER.to_string(),
            model: model.unwrap_or_else(|| CLAUDE_3_5_SONNET.to_string()),
            api_key,
        }
    }

    pub fn openai(api_key: String, model: Option<String>) -> Self {
        ProviderAgentConfig {
            provider: OPENAI_PROVIDER.to_string(),
            model: model.unwrap_or_else(|| GPT_4O.to_string()),
            api_key,
        }
    }
}

#[derive(Clone)]
pub struct UnifiedCompletionAgent {
    completion_model: ProviderCompletionModel,
    model_name: String,
    provider_name: String,
}

impl UnifiedCompletionAgent {
    pub(crate) fn new(
        completion_model: ProviderCompletionModel,
        model_name: String,
        provider_name: String,
    ) -> Self {
        Self {
            completion_model,
            model_name,
            provider_name,
        }
    }

    fn extract_text(
        &self,
        response: &RigCompletionResponse<ProviderResponse>,
    ) -> Result<String, CompletionAgentError> {
        let mut text = String::new();
        for piece in response.choice.iter() {
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
                CompletionAgentError::retryable(anyhow!(msg))
            }
            other => CompletionAgentError::fatal(anyhow!(other.to_string())),
        }
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
        // Build Rig CompletionRequest
        let rig_request = RigCompletionRequest {
            prompt: request.prompt.into(),
            preamble: Some(request.preamble.to_string()),
            chat_history: request.history.to_vec(),
            documents: vec![],
            tools: vec![],
            temperature: Some(request.temperature),
            max_tokens: Some(request.max_tokens),
            additional_params: None,
        };

        // Call completion() once - no provider-specific code!
        let completion = self
            .completion_model
            .completion(rig_request)
            .await
            .map_err(|err| self.categorize_error(err))?;

        let text = self.extract_text(&completion)?;

        Ok(CompletionOutcome {
            text,
            input_tokens: completion.raw_response.input_tokens(),
            output_tokens: completion.raw_response.output_tokens(),
        })
    }
}

pub struct CompletionAgentFactory;

impl CompletionAgentFactory {
    pub fn build(config: ProviderAgentConfig) -> Result<Box<dyn CompletionAgent>> {
        let (completion_model, provider_name) = match config.provider.as_str() {
            ANTHROPIC_PROVIDER => {
                let client = ClientBuilder::new(&config.api_key)
                    .anthropic_version("2023-06-01")
                    .build();
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

        let agent = UnifiedCompletionAgent::new(completion_model, config.model, provider_name);
        Ok(Box::new(agent))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rig::providers::anthropic::completion::Usage as AnthropicUsage;
    use rig::providers::openai::Usage as OpenAIUsage;

    #[test]
    fn test_provider_response_anthropic_token_extraction() {
        let usage = AnthropicUsage {
            input_tokens: 150,
            cache_read_input_tokens: None,
            cache_creation_input_tokens: None,
            output_tokens: 75,
        };
        let response = AnthropicCompletionResponse {
            id: "test".to_string(),
            role: "assistant".to_string(),
            content: vec![],
            model: "claude-3-5-sonnet-20241022".to_string(),
            stop_reason: None,
            stop_sequence: None,
            usage,
        };

        let provider_response = ProviderResponse::Anthropic(response);
        assert_eq!(provider_response.input_tokens(), 150);
        assert_eq!(provider_response.output_tokens(), 75);
    }

    #[test]
    fn test_provider_response_openai_token_extraction() {
        let response = OpenAICompletionResponse {
            id: "test".to_string(),
            object: "text_completion".to_string(),
            created: 0,
            model: "gpt-4o".to_string(),
            choices: vec![],
            usage: Some(OpenAIUsage {
                prompt_tokens: 100,
                total_tokens: 150,
            }),
            system_fingerprint: None,
        };

        let provider_response = ProviderResponse::OpenAI(response);
        assert_eq!(provider_response.input_tokens(), 100);
        assert_eq!(provider_response.output_tokens(), 50);
    }

    #[test]
    fn test_provider_response_openai_missing_usage_returns_zero() {
        let response = OpenAICompletionResponse {
            id: "test".to_string(),
            object: "text_completion".to_string(),
            created: 0,
            model: "gpt-4o".to_string(),
            choices: vec![],
            usage: None,
            system_fingerprint: None,
        };

        let provider_response = ProviderResponse::OpenAI(response);
        assert_eq!(provider_response.input_tokens(), 0);
        assert_eq!(provider_response.output_tokens(), 0);
    }

    #[test]
    fn test_provider_agent_config_openai_default_model() {
        let config = ProviderAgentConfig::openai("sk-test-key".to_string(), None);
        assert_eq!(config.provider, OPENAI_PROVIDER);
        assert_eq!(config.model, GPT_4O);
        assert_eq!(config.api_key, "sk-test-key");
    }

    #[test]
    fn test_provider_agent_config_openai_custom_model() {
        let config = ProviderAgentConfig::openai("sk-test-key".to_string(), Some("gpt-4-turbo".to_string()));
        assert_eq!(config.provider, OPENAI_PROVIDER);
        assert_eq!(config.model, "gpt-4-turbo");
        assert_eq!(config.api_key, "sk-test-key");
    }

    #[test]
    fn test_provider_agent_config_anthropic_default_model() {
        let config = ProviderAgentConfig::anthropic("test-key".to_string(), None);
        assert_eq!(config.provider, ANTHROPIC_PROVIDER);
        assert_eq!(config.model, CLAUDE_3_5_SONNET);
        assert_eq!(config.api_key, "test-key");
    }

    #[test]
    fn test_provider_agent_config_anthropic_custom_model() {
        let config = ProviderAgentConfig::anthropic("test-key".to_string(), Some("claude-3-opus".to_string()));
        assert_eq!(config.provider, ANTHROPIC_PROVIDER);
        assert_eq!(config.model, "claude-3-opus");
        assert_eq!(config.api_key, "test-key");
    }

    #[test]
    fn test_completion_agent_factory_anthropic() {
        let config = ProviderAgentConfig::anthropic("test-key".to_string(), None);
        let result = CompletionAgentFactory::build(config);
        assert!(result.is_ok());
        let agent = result.unwrap();
        assert_eq!(agent.provider(), ANTHROPIC_PROVIDER);
    }

    #[test]
    fn test_completion_agent_factory_openai() {
        let config = ProviderAgentConfig::openai("sk-test-key".to_string(), None);
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
        };
        let result = CompletionAgentFactory::build(config);
        assert!(result.is_err());
        match result {
            Err(err) => assert!(err.to_string().contains("Unsupported provider")),
            Ok(_) => panic!("Expected error"),
        }
    }
}
