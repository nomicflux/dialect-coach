use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use dialect_coach_shared::config::{ProviderDriver, ReasoningConfig, ResolvedChannel};
use rig::client::Nothing;
use rig::client::completion::CompletionClient;
use rig::completion::message::{AssistantContent, Text, UserContent};
use rig::completion::{
    CompletionError, CompletionModel, CompletionRequest as RigCompletionRequest,
    Message as RigMessage,
};
use rig::http_client::{HttpClientExt, ReqwestClient};
use rig::one_or_many::OneOrMany;
use rig::providers::anthropic::Client as AnthropicClient;
use rig::providers::ollama;
use rig::providers::openai::Client as OpenAIClient;

use std::sync::Arc;

pub const ANTHROPIC_PROVIDER: &str = "anthropic";
pub const OPENAI_PROVIDER: &str = "openai";
pub const OLLAMA_PROVIDER: &str = "ollama";

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
    pub base_url: Option<String>,
    pub api_key: String,
    pub reasoning_budget: u32,
    pub reasoning_effort: String,
}

impl ProviderAgentConfig {
    pub fn anthropic(api_key: String, model: String, reasoning_budget: u32) -> Self {
        ProviderAgentConfig {
            provider: ANTHROPIC_PROVIDER.to_string(),
            model,
            base_url: None,
            api_key,
            reasoning_budget,
            reasoning_effort: "low".to_string(),
        }
    }

    pub fn openai(api_key: String, model: String, reasoning_budget: u32) -> Self {
        ProviderAgentConfig {
            provider: OPENAI_PROVIDER.to_string(),
            model,
            base_url: None,
            api_key,
            reasoning_budget,
            reasoning_effort: "low".to_string(),
        }
    }
}

struct UnifiedCompletionAgent<M> {
    completion_model: M,
    model_name: String,
    provider_name: &'static str,
    reasoning: Option<ReasoningConfig>,
}

impl<M: CompletionModel> UnifiedCompletionAgent<M> {
    fn new(completion_model: M, channel: &ResolvedChannel, provider_name: &'static str) -> Self {
        Self {
            completion_model,
            model_name: channel.model.clone(),
            provider_name,
            reasoning: channel.reasoning.clone(),
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
            return Err(CompletionAgentError::retryable(anyhow!(
                "No text in response - output may have been truncated"
            )));
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

    fn build_completion_request(&self, request: &CompletionRequest<'_>) -> RigCompletionRequest {
        let mut chat_history = request.history.to_vec();
        chat_history.push(RigMessage::User {
            content: OneOrMany::one(UserContent::Text(Text {
                text: request.prompt.to_string(),
                additional_params: None,
            })),
        });

        let mut rig_request = RigCompletionRequest {
            preamble: Some(request.preamble.to_string()),
            chat_history: OneOrMany::many(chat_history).expect("chat_history should not be empty"),
            documents: vec![],
            tools: vec![],
            temperature: Some(request.temperature),
            max_tokens: Some(request.max_tokens),
            tool_choice: None,
            additional_params: None,
            model: None,
            output_schema: None,
            record_telemetry_content: false,
        };

        if let Some(reasoning) = &self.reasoning {
            rig_request.temperature = None;
            rig_request.max_tokens = Some(request.max_tokens + u64::from(reasoning.budget));
            rig_request.additional_params = Some(serde_json::json!({
                "reasoning": { "effort": reasoning.effort }
            }));
        }

        rig_request
    }
}

#[async_trait]
impl<M: CompletionModel> CompletionAgent for UnifiedCompletionAgent<M> {
    fn provider(&self) -> &str {
        self.provider_name
    }

    fn model(&self) -> &str {
        &self.model_name
    }

    async fn completion(
        &self,
        request: &CompletionRequest<'_>,
    ) -> Result<CompletionOutcome, CompletionAgentError> {
        let response = self
            .completion_model
            .completion(self.build_completion_request(request))
            .await
            .map_err(|err| self.categorize_error(err))?;
        let text = self.extract_text(&response.choice)?;
        Ok(CompletionOutcome {
            text,
            input_tokens: response.usage.input_tokens,
            output_tokens: response.usage.output_tokens,
        })
    }
}

pub struct CompletionAgentFactory;

impl CompletionAgentFactory {
    pub fn build(
        channel: &ResolvedChannel,
        api_key: Option<&str>,
    ) -> Result<Arc<dyn CompletionAgent>> {
        Self::build_with_client(channel, api_key, ReqwestClient::default())
    }

    fn build_with_client<H: HttpClientExt + Clone + Default + std::fmt::Debug + 'static>(
        channel: &ResolvedChannel,
        api_key: Option<&str>,
        http_client: H,
    ) -> Result<Arc<dyn CompletionAgent>> {
        let agent: Arc<dyn CompletionAgent> = match channel.driver {
            ProviderDriver::Anthropic => {
                let client = AnthropicClient::builder()
                    .api_key(api_key.context("Anthropic API key is required")?)
                    .base_url(&channel.base_url)
                    .http_client(http_client)
                    .build()?;
                Arc::new(UnifiedCompletionAgent::new(
                    client.completion_model(&channel.model),
                    channel,
                    ANTHROPIC_PROVIDER,
                ))
            }
            ProviderDriver::OpenAi => {
                let client = OpenAIClient::builder()
                    .api_key(api_key.context("OpenAI API key is required")?)
                    .base_url(&channel.base_url)
                    .http_client(http_client)
                    .build()?;
                Arc::new(UnifiedCompletionAgent::new(
                    client.completion_model(&channel.model),
                    channel,
                    OPENAI_PROVIDER,
                ))
            }
            ProviderDriver::Ollama => {
                let api_key = api_key.map_or_else(|| Nothing.into(), ollama::OllamaApiKey::from);
                let client = ollama::Client::builder()
                    .api_key(api_key)
                    .base_url(&channel.base_url)
                    .http_client(http_client)
                    .build()?;
                Arc::new(UnifiedCompletionAgent::new(
                    client.completion_model(&channel.model),
                    channel,
                    OLLAMA_PROVIDER,
                ))
            }
        };
        Ok(agent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_service::retry::retry_completion_call;
    use axum::body::Bytes;
    use dialect_coach_shared::config::LlmConfig;
    use rig::core::test_utils::{MockHttpResponse, RecordingHttpClient};
    use rig::http_client::{
        self, LazyBody, Method, MultipartForm, Request, Response, StreamingResponse,
    };
    use serde_json::{Value, json};
    use std::future::Future;

    // Rig's recorder captures the URI, headers and body. Assert the method at
    // the same boundary before handing the request to the recorder.
    #[derive(Clone, Debug, Default)]
    struct RecordingPostClient(RecordingHttpClient);

    impl HttpClientExt for RecordingPostClient {
        fn send<T, U>(
            &self,
            request: Request<T>,
        ) -> impl Future<Output = http_client::Result<Response<LazyBody<U>>>> + Send + 'static
        where
            T: Into<Bytes> + Send,
            U: From<Bytes> + Send + 'static,
        {
            assert_eq!(request.method(), Method::POST);
            self.0.send(request)
        }

        fn send_multipart<U>(
            &self,
            _: Request<MultipartForm>,
        ) -> impl Future<Output = http_client::Result<Response<LazyBody<U>>>> + Send + 'static
        where
            U: From<Bytes> + Send + 'static,
        {
            async { panic!("Unexpected multipart request") }
        }

        async fn send_streaming<T>(&self, _: Request<T>) -> http_client::Result<StreamingResponse>
        where
            T: Into<Bytes> + Send,
        {
            panic!("Unexpected streaming request")
        }
    }

    fn resolved_channel(provider: &str) -> ResolvedChannel {
        let yaml = format!("default_provider: test\nproviders:\n  test:\n{provider}");
        let config: LlmConfig = serde_yaml::from_str(&yaml).unwrap();
        config.resolve_channel("response").unwrap()
    }

    fn request(max_tokens: u64) -> CompletionRequest<'static> {
        CompletionRequest {
            preamble: "Use a brief answer",
            prompt: "Say hello",
            history: &[],
            max_tokens,
            temperature: 0.7,
        }
    }

    fn ollama_reply(text: &str) -> String {
        json!({
            "model": "custom-local:42",
            "created_at": "2026-01-01T00:00:00Z",
            "message": {"role": "assistant", "content": text, "thinking": "private reasoning"},
            "done": true,
            "done_reason": "stop",
            "prompt_eval_count": 13,
            "eval_count": 9
        })
        .to_string()
    }

    #[tokio::test]
    async fn ollama_native_request_preserves_model_limits_and_authentication() {
        let settings = r#"
    driver: ollama
    base_url: http://ollama.test:11434
    model: custom-local:42
"#;
        for credential in [None, Some("test-local-key")] {
            let settings = if credential.is_some() {
                format!("{settings}    api_key_env: TEST_OLLAMA_KEY\n")
            } else {
                settings.into()
            };
            let channel = resolved_channel(&settings);
            let key = channel
                .api_key(|name| {
                    assert_eq!(name, "TEST_OLLAMA_KEY");
                    credential.map(str::to_owned)
                })
                .unwrap();
            let http = RecordingHttpClient::new(ollama_reply("Hello there"));
            let agent = CompletionAgentFactory::build_with_client(
                &channel,
                key.as_deref(),
                RecordingPostClient(http.clone()),
            )
            .unwrap();

            let (text, usage) = retry_completion_call(agent.as_ref(), &request(256), 1).await;
            assert_eq!(text.unwrap(), "Hello there");
            assert_eq!(usage.len(), 1);
            assert_eq!(usage[0].input_tokens, 13);
            assert_eq!(usage[0].output_tokens, 9);
            assert!(!usage[0].is_estimate);
            assert_eq!(usage[0].provider, OLLAMA_PROVIDER);
            assert_eq!(usage[0].model, "custom-local:42");

            let requests = http.requests();
            assert_eq!(requests.len(), 1);
            assert_eq!(requests[0].uri, "http://ollama.test:11434/api/chat");
            let authorization = requests[0].headers.get("authorization");
            assert_eq!(
                authorization.map(|header| header.to_str().unwrap().to_owned()),
                credential.map(|key| format!("Bearer {key}"))
            );
            let json: Value = serde_json::from_slice(&requests[0].body).unwrap();
            assert_eq!(json["model"], "custom-local:42");
            assert_eq!(json["options"]["temperature"], 0.7);
            assert_eq!(json["options"]["num_predict"], 256);

            // The fixture still includes a reasoning block, so this exercises
            // the application's missing-text handling after Rig parses the reply.
            http.set_response(MockHttpResponse::success(ollama_reply("")));
            assert!(matches!(
                agent.completion(&request(256)).await,
                Err(CompletionAgentError::Retryable(_))
            ));
        }
    }

    #[tokio::test]
    async fn anthropic_wire_request_preserves_model_and_temperature() {
        let channel = resolved_channel(
            r#"
    driver: anthropic
    base_url: https://anthropic.test
    model: claude-haiku-4-5
"#,
        );
        let http = RecordingHttpClient::new(
            json!({
                "id": "msg_test",
                "type": "message",
                "model": "claude-haiku-4-5",
                "role": "assistant",
                "content": [{"type": "text", "text": "Hello"}],
                "stop_reason": "end_turn",
                "usage": {"input_tokens": 3, "output_tokens": 1}
            })
            .to_string(),
        );
        let agent = CompletionAgentFactory::build_with_client(
            &channel,
            Some("test-key"),
            RecordingPostClient(http.clone()),
        )
        .unwrap();
        assert_eq!(agent.completion(&request(256)).await.unwrap().text, "Hello");
        let requests = http.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].uri, "https://anthropic.test/v1/messages");
        let json: Value = serde_json::from_slice(&requests[0].body).unwrap();
        assert_eq!(json["model"], "claude-haiku-4-5");
        assert_eq!(json["temperature"], 0.7);
        assert_eq!(json["max_tokens"], 256);
    }

    #[tokio::test]
    async fn openai_wire_request_preserves_reasoning_effort_and_output_allowance() {
        for effort in ["low", "none"] {
            let channel = resolved_channel(&format!(
                r#"
    driver: openai
    base_url: https://openai.test/v1
    model: gpt-5.4
    reasoning_budget: 512
    reasoning_effort: {effort}
"#
            ));
            let http = RecordingHttpClient::new(
                json!({
                    "id": "resp_test",
                    "object": "response",
                    "created_at": 0,
                    "status": "completed",
                    "model": "gpt-5.4",
                    "usage": {"input_tokens": 3, "output_tokens": 1, "total_tokens": 4},
                    "output": [{
                        "type": "message",
                        "id": "msg_test",
                        "status": "completed",
                        "role": "assistant",
                        "content": [{"type": "output_text", "annotations": [], "text": "Hello"}]
                    }],
                    "tools": []
                })
                .to_string(),
            );
            let agent = CompletionAgentFactory::build_with_client(
                &channel,
                Some("test-key"),
                RecordingPostClient(http.clone()),
            )
            .unwrap();
            let request = request(2048);
            assert_eq!(agent.completion(&request).await.unwrap().text, "Hello");
            let requests = http.requests();
            assert_eq!(requests.len(), 1);
            assert_eq!(requests[0].uri, "https://openai.test/v1/responses");
            let json: Value = serde_json::from_slice(&requests[0].body).unwrap();
            assert_eq!(json["reasoning"]["effort"], effort);
            assert_eq!(json["max_output_tokens"], 2560);
            assert_eq!(json["model"], "gpt-5.4");
            assert_eq!(json["instructions"], request.preamble);
            assert_eq!(json["input"][0]["content"][0]["text"], request.prompt);
            assert!(json.get("temperature").is_none());
        }
    }
}
