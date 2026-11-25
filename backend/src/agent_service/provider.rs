use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use base64::prelude::*;
use dialect_coach_shared::config::{ProviderDriver, ReasoningConfig, ResolvedChannel};
use rig::client::Nothing;
use rig::client::completion::CompletionClient;
use rig::completion::message::{AssistantContent, ImageMediaType, UserContent};
use rig::completion::{
    CompletionError, CompletionModel, CompletionRequest as RigCompletionRequest,
    Message as RigMessage,
};
use rig::http_client::{HttpClientExt, ReqwestClient};
use rig::one_or_many::OneOrMany;
use rig::providers::anthropic::Client as AnthropicClient;
use rig::providers::ollama;
use rig::providers::openai::Client as OpenAIClient;

use super::keyword_extraction::ExtractedKeywords;
use std::sync::Arc;
use std::time::Duration;

pub const ANTHROPIC_PROVIDER: &str = "anthropic";
pub const OPENAI_PROVIDER: &str = "openai";
pub const OLLAMA_PROVIDER: &str = "ollama";
const KEYWORD_OUTPUT_TOKENS: u64 = 1024;

#[derive(Debug, Clone)]
pub enum CompletionFormat {
    Text,
    Json,
    JsonSchema(schemars::Schema),
    Structured(schemars::Schema),
}

#[derive(Debug, Clone)]
pub struct CompletionRequest<'a> {
    pub preamble: &'a str,
    pub prompt: &'a str,
    pub history: &'a [RigMessage],
    pub max_tokens: u64,
    pub temperature: f64,
    pub format: CompletionFormat,
}

#[derive(Debug, Clone)]
pub struct CompletionOutcome {
    pub text: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum CompletionAgentError {
    #[error(transparent)]
    Retryable(anyhow::Error),
    #[error(transparent)]
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

    fn supports_temperature(&self) -> bool {
        false
    }

    async fn completion(
        &self,
        request: &CompletionRequest<'_>,
    ) -> Result<CompletionOutcome, CompletionAgentError>;

    async fn extract_keywords(&self, _prompt: &str) -> Result<ExtractedKeywords> {
        anyhow::bail!("The selected integration does not support keyword extraction")
    }

    async fn complete_images(
        &self,
        _prompt: &str,
        _images: &[Vec<u8>],
        _max_tokens: u64,
    ) -> Result<CompletionOutcome, CompletionAgentError> {
        Err(CompletionAgentError::fatal(anyhow!(
            "The selected integration does not support image completion"
        )))
    }
}

fn rig_request(
    preamble: Option<String>,
    chat_history: OneOrMany<RigMessage>,
    max_tokens: u64,
    temperature: Option<f64>,
) -> RigCompletionRequest {
    RigCompletionRequest {
        preamble,
        chat_history,
        documents: vec![],
        tools: vec![],
        temperature,
        max_tokens: Some(max_tokens),
        tool_choice: None,
        additional_params: None,
        model: None,
        output_schema: None,
        record_telemetry_content: false,
    }
}

struct UnifiedCompletionAgent<M> {
    completion_model: M,
    model_name: String,
    provider_name: &'static str,
    reasoning: Option<ReasoningConfig>,
    think: Option<bool>,
}

impl<M: CompletionModel> UnifiedCompletionAgent<M> {
    fn new(completion_model: M, channel: &ResolvedChannel, provider_name: &'static str) -> Self {
        Self {
            completion_model,
            model_name: channel.model.clone(),
            provider_name,
            reasoning: channel.reasoning.clone(),
            think: channel.think,
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
                tracing::error!(
                    provider = %self.provider_name,
                    model = %self.model_name,
                    "Fatal provider error: {}",
                    other
                );
                CompletionAgentError::fatal(other)
            }
        }
    }

    fn build_completion_request(&self, request: &CompletionRequest<'_>) -> RigCompletionRequest {
        let mut chat_history = request.history.to_vec();
        if matches!(
            request.format,
            CompletionFormat::Json | CompletionFormat::JsonSchema(_)
        ) && self.provider_name == ANTHROPIC_PROVIDER
        {
            chat_history.push(RigMessage::assistant("{"));
        }
        chat_history.push(RigMessage::user(request.prompt));

        let mut rig_request = rig_request(
            Some(request.preamble.to_string()),
            OneOrMany::many(chat_history).expect("chat_history should not be empty"),
            request.max_tokens,
            Some(request.temperature),
        );

        if self.provider_name == OLLAMA_PROVIDER {
            rig_request.output_schema = match &request.format {
                CompletionFormat::Text => None,
                CompletionFormat::Json => Some(schemars::json_schema!({
                    "type": "object", "additionalProperties": true
                })),
                CompletionFormat::JsonSchema(schema) | CompletionFormat::Structured(schema) => {
                    Some(schema.clone())
                }
            };
        }

        if let Some(reasoning) = &self.reasoning {
            rig_request.temperature = None;
            rig_request.max_tokens = Some(request.max_tokens + u64::from(reasoning.budget));
            rig_request.additional_params = Some(serde_json::json!({
                "reasoning": { "effort": reasoning.effort }
            }));
        }

        rig_request
    }

    async fn send(
        &self,
        mut request: RigCompletionRequest,
    ) -> Result<CompletionOutcome, CompletionAgentError> {
        if let Some(think) = self.think {
            request.additional_params = Some(serde_json::json!({"think": think}));
        }
        let response = self
            .completion_model
            .completion(request)
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

#[async_trait]
impl<M: CompletionModel + 'static> CompletionAgent for UnifiedCompletionAgent<M> {
    fn provider(&self) -> &str {
        self.provider_name
    }

    fn model(&self) -> &str {
        &self.model_name
    }

    fn supports_temperature(&self) -> bool {
        self.reasoning.is_none()
    }

    async fn completion(
        &self,
        request: &CompletionRequest<'_>,
    ) -> Result<CompletionOutcome, CompletionAgentError> {
        self.send(self.build_completion_request(request)).await
    }

    async fn extract_keywords(&self, prompt: &str) -> Result<ExtractedKeywords> {
        if self.provider_name == OLLAMA_PROVIDER {
            // Ollama accepts a JSON schema but cannot require a tool call.
            let mut request = rig_request(
                None,
                OneOrMany::one(RigMessage::user(prompt)),
                KEYWORD_OUTPUT_TOKENS,
                Some(0.0),
            );
            request.output_schema = Some(schemars::schema_for!(ExtractedKeywords));
            let outcome = self.send(request).await?;
            return serde_json::from_str(&outcome.text).context("Invalid keyword extraction JSON");
        }

        let builder = rig::extractor::ExtractorBuilder::<M, ExtractedKeywords>::new(
            self.completion_model.clone(),
        );
        let builder = match &self.reasoning {
            Some(reasoning) => builder.additional_params(serde_json::json!({
                "reasoning": { "effort": reasoning.effort }
            })),
            None => builder.max_tokens(KEYWORD_OUTPUT_TOKENS),
        };
        Ok(builder.build().extract(prompt).await?)
    }

    async fn complete_images(
        &self,
        prompt: &str,
        images: &[Vec<u8>],
        max_tokens: u64,
    ) -> Result<CompletionOutcome, CompletionAgentError> {
        let mut content = OneOrMany::one(UserContent::text(prompt));
        for image in images {
            content.push(UserContent::image_base64(
                BASE64_STANDARD.encode(image),
                Some(ImageMediaType::JPEG),
                None,
            ));
        }
        self.send(rig_request(
            None,
            OneOrMany::one(RigMessage::User { content }),
            max_tokens,
            None,
        ))
        .await
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CompletionApi {
    Standard,
    Ocr,
}

pub struct CompletionAgentFactory;

impl CompletionAgentFactory {
    pub fn build(
        channel: &ResolvedChannel,
        api_key: Option<&str>,
    ) -> Result<Arc<dyn CompletionAgent>> {
        Self::build_for_api(channel, api_key, CompletionApi::Standard)
    }

    pub fn build_ocr(
        channel: &ResolvedChannel,
        api_key: Option<&str>,
    ) -> Result<Arc<dyn CompletionAgent>> {
        Self::build_for_api(channel, api_key, CompletionApi::Ocr)
    }

    fn build_for_api(
        channel: &ResolvedChannel,
        api_key: Option<&str>,
        api: CompletionApi,
    ) -> Result<Arc<dyn CompletionAgent>> {
        let timeout = channel
            .request_timeout
            .or_else(|| (api == CompletionApi::Ocr).then_some(Duration::from_secs(60)));
        let client = match timeout {
            Some(timeout) => ReqwestClient::builder().timeout(timeout).build()?,
            None => ReqwestClient::default(),
        };
        Self::build_with_client(channel, api_key, client, api)
    }

    fn build_with_client<H: HttpClientExt + Clone + Default + std::fmt::Debug + 'static>(
        channel: &ResolvedChannel,
        api_key: Option<&str>,
        http_client: H,
        api: CompletionApi,
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
                if api == CompletionApi::Ocr {
                    Arc::new(UnifiedCompletionAgent::new(
                        client.completions_api().completion_model(&channel.model),
                        channel,
                        OPENAI_PROVIDER,
                    ))
                } else {
                    Arc::new(UnifiedCompletionAgent::new(
                        client.completion_model(&channel.model),
                        channel,
                        OPENAI_PROVIDER,
                    ))
                }
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
            format: CompletionFormat::Text,
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
        for (credential, think) in [(None, None), (Some("test-local-key"), Some(false))] {
            let settings = if let Some(think) = think {
                format!("{settings}    api_key_env: TEST_OLLAMA_KEY\n    think: {think}\n")
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
                CompletionApi::Standard,
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
            assert_eq!(json.get("think").and_then(Value::as_bool), think);

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
            CompletionApi::Standard,
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
                CompletionApi::Standard,
            )
            .unwrap();
            let mut request = request(2048);
            request.format = CompletionFormat::Json;
            assert_eq!(agent.completion(&request).await.unwrap().text, "Hello");
            let requests = http.requests();
            assert_eq!(requests.len(), 1);
            assert_eq!(requests[0].uri, "https://openai.test/v1/responses");
            let json: Value = serde_json::from_slice(&requests[0].body).unwrap();
            assert_eq!(json["reasoning"]["effort"], effort);
            assert_eq!(json["max_output_tokens"], 2560);
            assert_eq!(json["model"], "gpt-5.4");
            assert_eq!(json["instructions"], request.preamble);
            assert_eq!(json["input"].as_array().unwrap().len(), 1);
            assert_eq!(json["input"][0]["content"][0]["text"], request.prompt);
            assert!(json.get("temperature").is_none());
        }
    }

    #[tokio::test]
    async fn hosted_keyword_extraction_uses_the_rig_submit_tool() {
        let channel = resolved_channel(
            r#"
    driver: openai
    base_url: https://openai.test/v1
    model: configured-keyword-model
    reasoning_effort: minimal
"#,
        );
        let http = RecordingHttpClient::new(
            json!({
                "id": "resp_test", "object": "response", "created_at": 0,
                "status": "completed", "model": channel.model, "tools": [],
                "usage": {"input_tokens": 3, "output_tokens": 1, "total_tokens": 4},
                "output": [{
                    "type": "function_call", "id": "fc_test", "call_id": "call_test",
                    "name": "submit", "status": "completed",
                    "arguments": r#"{"keywords":["bicycle"]}"#
                }]
            })
            .to_string(),
        );
        let agent = CompletionAgentFactory::build_with_client(
            &channel,
            Some("test-key"),
            RecordingPostClient(http.clone()),
            CompletionApi::Standard,
        )
        .unwrap();

        assert_eq!(
            agent
                .extract_keywords("Find a bicycle")
                .await
                .unwrap()
                .keywords,
            ["bicycle"]
        );
        let requests = http.requests();
        assert_eq!(requests.len(), 1);
        let body: Value = serde_json::from_slice(&requests[0].body).unwrap();
        assert_eq!(body["model"], channel.model);
        assert_eq!(body["reasoning"]["effort"], "minimal");
        assert_eq!(body["tool_choice"], "required");
        assert_eq!(body["tools"][0]["name"], "submit");
    }

    #[tokio::test]
    async fn anthropic_keyword_extraction_supports_an_unrecognized_model_tag() {
        let channel = resolved_channel(
            r#"
    driver: anthropic
    base_url: https://anthropic.test
    model: configured-keyword-model
"#,
        );
        let http = RecordingHttpClient::new(
            json!({
                "id": "msg_test", "type": "message", "model": channel.model,
                "role": "assistant", "stop_reason": "tool_use",
                "content": [{
                    "type": "tool_use", "id": "tool_test", "name": "submit",
                    "input": {"keywords": ["bicycle"]}
                }],
                "usage": {"input_tokens": 3, "output_tokens": 1}
            })
            .to_string(),
        );
        let agent = CompletionAgentFactory::build_with_client(
            &channel,
            Some("test-key"),
            RecordingPostClient(http.clone()),
            CompletionApi::Standard,
        )
        .unwrap();

        let extracted = agent.extract_keywords("Find a bicycle").await.unwrap();
        assert_eq!(extracted.keywords, ["bicycle"]);
        let requests = http.requests();
        assert_eq!(requests.len(), 1);
        let body: Value = serde_json::from_slice(&requests[0].body).unwrap();
        assert_eq!(body["model"], channel.model);
        assert_eq!(body["max_tokens"], KEYWORD_OUTPUT_TOKENS);
        assert_eq!(body["tool_choice"]["type"], "any");
        assert_eq!(body["tools"][0]["name"], "submit");
    }

    #[tokio::test]
    async fn openai_ocr_uses_chat_completions_with_the_original_allowance() {
        let channel = resolved_channel(
            r#"
    driver: openai
    base_url: https://openai.test/v1
    model: configured-vision-model
    reasoning_budget: 512
"#,
        );
        let http = RecordingHttpClient::new(
            json!({
                "id": "chatcmpl_test", "object": "chat.completion",
                "created": 0, "model": channel.model,
                "choices": [{
                    "index": 0, "finish_reason": "stop",
                    "message": {"role": "assistant", "content": "Page text"}
                }],
                "usage": {"prompt_tokens": 3, "completion_tokens": 1, "total_tokens": 4}
            })
            .to_string(),
        );
        let agent = CompletionAgentFactory::build_with_client(
            &channel,
            Some("test-key"),
            RecordingPostClient(http.clone()),
            CompletionApi::Ocr,
        )
        .unwrap();

        let response = agent
            .complete_images("Transcribe", &[vec![1], vec![2]], 4096)
            .await
            .unwrap();
        assert_eq!(response.text, "Page text");
        let requests = http.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].uri, "https://openai.test/v1/chat/completions");
        let body: Value = serde_json::from_slice(&requests[0].body).unwrap();
        assert_eq!(body["model"], channel.model);
        assert_eq!(body["max_tokens"], 4096);
        assert!(body["temperature"].is_null());
        assert!(body["reasoning"].is_null());
        let content = &body["messages"][0]["content"];
        assert_eq!(content[0]["text"], "Transcribe");
        assert_eq!(
            content[1]["image_url"]["url"],
            "data:image/jpeg;base64,AQ=="
        );
        assert_eq!(
            content[2]["image_url"]["url"],
            "data:image/jpeg;base64,Ag=="
        );
    }
}
