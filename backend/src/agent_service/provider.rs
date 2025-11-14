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

enum ProviderResponse {
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

trait ClientProvider: Send + Sync {
    fn provider_name(&self) -> &str;

    fn completion_with_history(
        &self,
        model: &str,
        preamble: &str,
        max_tokens: u64,
        temperature: f64,
        prompt: &str,
        history: Vec<RigMessage>,
    ) -> impl std::future::Future<Output = Result<ProviderResponse, CompletionAgentError>> + Send;
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
pub struct AnthropicCompletionAgent {
    client: Client,
    model: String,
}

impl AnthropicCompletionAgent {
    pub fn new(api_key: &str, model: &str) -> Result<Self> {
        let client = ClientBuilder::new(api_key)
            .anthropic_version("2023-06-01")
            .build();
        Ok(Self {
            client,
            model: model.to_string(),
        })
    }

    fn extract_text(
        &self,
        response: &RigCompletionResponse<AnthropicCompletionResponse>,
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
impl CompletionAgent for AnthropicCompletionAgent {
    fn provider(&self) -> &str {
        ANTHROPIC_PROVIDER
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn completion(
        &self,
        request: &CompletionRequest<'_>,
    ) -> Result<CompletionOutcome, CompletionAgentError> {
        let rig_agent = self
            .client
            .agent(&self.model)
            .preamble(request.preamble)
            .max_tokens(request.max_tokens)
            .temperature(request.temperature)
            .build();
        let completion = rig_agent
            .completion(request.prompt, request.history.to_vec())
            .await
            .map_err(|err| self.categorize_error(err))?
            .send()
            .await
            .map_err(|err| self.categorize_error(err))?;

        let text = self.extract_text(&completion)?;
        let usage = completion.raw_response.usage;

        Ok(CompletionOutcome {
            text,
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
        })
    }
}

pub struct CompletionAgentFactory;

impl CompletionAgentFactory {
    pub fn build(config: ProviderAgentConfig) -> Result<Box<dyn CompletionAgent>> {
        match config.provider.as_str() {
            ANTHROPIC_PROVIDER => {
                let agent = AnthropicCompletionAgent::new(&config.api_key, &config.model)?;
                Ok(Box::new(agent))
            }
            other => Err(anyhow!("Unsupported provider: {}", other)),
        }
    }
}
