use anyhow::{Context, Result};
use dialect_coach_shared::AgentUsage;
use rig::OneOrMany;
use rig::completion::{
    AssistantContent, Completion, CompletionError, CompletionResponse, Message as RigMessage,
};
use rig::providers::anthropic::completion::CompletionResponse as AnthropicResponse;

use super::util::{self, GenerationConfig, get_message_text};

pub struct RetryAttemptParams {
    pub attempt: usize,
    pub max_retries: usize,
}

pub struct RetryPromptParams<'a, F>
where
    F: Fn(&str, &str) -> String,
{
    pub original_preamble: &'a str,
    pub failed_response: &'a str,
    pub prompt: &'a str,
    pub preamble_builder: &'a F,
}

pub fn build_retry_preamble(
    original_preamble: &str,
    failed_response: &str,
    error_detector: impl Fn(&str) -> String,
    additional_instructions: &str,
) -> String {
    let cleaned = util::clean_response(failed_response);
    let error_detail = if util::is_incomplete_json(failed_response) {
        "Your previous response was TRUNCATED because it hit the token limit. The JSON was cut off mid-response, causing a parse error. You MUST keep your response shorter to fit within the token limit, or ensure the JSON is properly closed even if truncated.".to_string()
    } else {
        error_detector(&cleaned)
    };

    format!(
        "{}\n\n\
            # CRITICAL ERROR - SYSTEM CRASHED\n\
            {}\n\
            Previous response (INCORRECT): {}\n\
            You MUST respond with valid JSON only. Start with {{ and end with }}. {}\n",
        original_preamble,
        error_detail,
        failed_response.chars().take(200).collect::<String>(),
        additional_instructions
    )
}

pub fn detect_response_error(cleaned: &str) -> String {
    match serde_json::from_str::<dialect_coach_shared::AgentResponse>(cleaned) {
        Ok(parsed) if parsed.response.is_empty() => {
            "Your previous response had an EMPTY 'response' field. The response field must contain actual text content and cannot be empty.".to_string()
        }
        Ok(_) => {
            "Your previous response was not formatted correctly and caused the system to crash.".to_string()
        }
        Err(_) => {
            "Your previous response was not formatted correctly as JSON and caused the system to crash.".to_string()
        }
    }
}

pub fn detect_analysis_error(cleaned: &str) -> String {
    match serde_json::from_str::<dialect_coach_shared::AgentAnalysis>(cleaned) {
        Ok(_) => {
            "Your previous response was not formatted correctly and caused the system to crash.".to_string()
        }
        Err(_) => {
            "Your previous response was not formatted correctly as JSON and caused the system to crash.".to_string()
        }
    }
}

pub fn build_retry_response_preamble(original_preamble: &str, failed_response: &str) -> String {
    build_retry_preamble(
        original_preamble,
        failed_response,
        detect_response_error,
        "The 'response' field MUST be non-empty. You MUST NOT end the conversation.",
    )
}

pub fn build_retry_analysis_preamble(original_preamble: &str, failed_response: &str) -> String {
    build_retry_preamble(
        original_preamble,
        failed_response,
        detect_analysis_error,
        "You MUST return valid JSON with numeric scores only.",
    )
}

pub fn estimate_input_tokens(preamble: &str, history: &[RigMessage], prompt: &str) -> u64 {
    let preamble_chars = preamble.len();
    let history_chars: usize = history.iter().map(|msg| get_message_text(msg).len()).sum();
    let prompt_chars = prompt.len();
    let total_chars = preamble_chars + history_chars + prompt_chars;
    (total_chars as f64 * 0.25).ceil() as u64
}

fn create_usage_entry(is_retry: bool, is_estimate: bool, input: u64, output: u64) -> AgentUsage {
    AgentUsage {
        timestamp: chrono::Utc::now().timestamp(),
        input_tokens: input,
        output_tokens: output,
        is_retry,
        is_estimate,
    }
}

fn extract_text_from_choice(choice: &OneOrMany<AssistantContent>) -> Result<String> {
    let text: String = choice
        .iter()
        .filter_map(|c| match c {
            AssistantContent::Text(t) => Some(t.text.as_str()),
            _ => None,
        })
        .collect();
    if text.is_empty() {
        Err(anyhow::anyhow!("No text in response"))
    } else {
        Ok(text)
    }
}

enum AttemptResult {
    Success(String, AgentUsage),
    Retry(AgentUsage, CompletionError),
    Failed(AgentUsage, CompletionError),
}

async fn attempt_completion<F, Fut>(
    completion_call: F,
    estimate_input: impl Fn() -> u64,
    is_retry: bool,
) -> AttemptResult
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<CompletionResponse<AnthropicResponse>, CompletionError>>,
{
    match completion_call().await {
        Ok(response) => {
            let text = match extract_text_from_choice(&response.choice) {
                Ok(t) => t,
                Err(e) => {
                    let usage = create_usage_entry(is_retry, true, estimate_input(), 0);
                    return AttemptResult::Failed(
                        usage,
                        CompletionError::ResponseError(e.to_string()),
                    );
                }
            };
            let usage = create_usage_entry(
                is_retry,
                false,
                response.raw_response.usage.input_tokens,
                response.raw_response.usage.output_tokens,
            );
            AttemptResult::Success(text, usage)
        }
        Err(e) if is_retryable_completion_error(&e) => {
            let usage = create_usage_entry(is_retry, true, estimate_input(), 0);
            AttemptResult::Retry(usage, e)
        }
        Err(e) => {
            let usage = create_usage_entry(is_retry, true, estimate_input(), 0);
            AttemptResult::Failed(usage, e)
        }
    }
}

fn is_retryable_completion_error(error: &CompletionError) -> bool {
    matches!(
        error,
        CompletionError::ProviderError(_) | CompletionError::ResponseError(_)
    )
}

async fn handle_completion_retry_delay(attempt: usize, max: usize, error: &CompletionError) {
    tracing::warn!("Retry {}/{}: {}", attempt, max, error);
    tokio::time::sleep(tokio::time::Duration::from_secs(2_u64.pow(attempt as u32))).await;
}

pub async fn retry_completion_call<F, Fut>(
    completion_call: F,
    estimate_input: impl Fn() -> u64,
    max_attempts: usize,
) -> Result<(String, Vec<AgentUsage>)>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<CompletionResponse<AnthropicResponse>, CompletionError>>,
{
    let mut usages = Vec::new();

    for attempt in 1..=max_attempts {
        let is_retry = attempt > 1;
        match attempt_completion(&completion_call, &estimate_input, is_retry).await {
            AttemptResult::Success(text, usage) => {
                usages.push(usage);
                return Ok((text, usages));
            }
            AttemptResult::Retry(usage, error) => {
                usages.push(usage);
                if attempt < max_attempts {
                    handle_completion_retry_delay(attempt, max_attempts, &error).await;
                }
            }
            AttemptResult::Failed(usage, error) => {
                usages.push(usage);
                return Err(anyhow::anyhow!("API call failed: {}", error));
            }
        }
    }
    Err(anyhow::anyhow!("Failed after {} attempts", max_attempts))
}

pub fn build_retry_failure_error(
    max_retries: usize,
    last_error: &str,
    last_failed_response: &str,
) -> anyhow::Error {
    anyhow::anyhow!(
        "Claude returned invalid JSON after {} retry attempts: {}. Last failed response: {}",
        max_retries,
        last_error,
        &last_failed_response.chars().take(200).collect::<String>()
    )
}

pub fn log_retry_parse_error(attempt: usize, error: &anyhow::Error, response: &str) {
    tracing::error!(
        "JSON parse error on retry attempt {}: {}. First 200 chars of retry response: {}",
        attempt,
        error,
        &response.chars().take(200).collect::<String>()
    );
}

pub fn detect_truncation_error(error: &anyhow::Error, response: &str) -> bool {
    let error_str = format!("{}", error);
    error_str.contains("EOF while parsing") || util::is_incomplete_json(response)
}

pub fn process_retry_response<T, F>(
    attempt_params: &RetryAttemptParams,
    response: String,
    prompt_params: &RetryPromptParams<'_, F>,
    parse_fn: &impl Fn(&str) -> Result<T>,
    log_success: &impl Fn(&T),
) -> Result<(Option<T>, String), anyhow::Error>
where
    F: Fn(&str, &str) -> String,
{
    match parse_fn(&response) {
        Ok(parsed_response) => {
            log_success(&parsed_response);
            Ok((Some(parsed_response), response))
        }
        Err(e) => {
            if detect_truncation_error(&e, &response) {
                tracing::error!(
                    "Token limit truncation detected on retry attempt {}",
                    attempt_params.attempt
                );
            }
            log_retry_parse_error(attempt_params.attempt, &e, &response);
            if attempt_params.attempt == attempt_params.max_retries {
                return Err(build_retry_failure_error(
                    attempt_params.max_retries,
                    &format!("{}", e),
                    prompt_params.failed_response,
                ));
            }
            Ok((None, response))
        }
    }
}

pub struct RetryContext<'a> {
    pub client: &'a rig::providers::anthropic::Client,
    pub model_name: &'a str,
}

impl<'a> RetryContext<'a> {
    pub async fn attempt_retry_with_feedback<F>(
        &self,
        prompt_params: &RetryPromptParams<'_, F>,
        conversation_history: &[RigMessage],
        config: &GenerationConfig,
    ) -> Result<(String, Vec<AgentUsage>)>
    where
        F: Fn(&str, &str) -> String,
    {
        let retry_preamble = (prompt_params.preamble_builder)(
            prompt_params.original_preamble,
            prompt_params.failed_response,
        );
        let mut history_with_prefill = conversation_history.to_vec();
        history_with_prefill.push(util::create_prefilled_assistant_message());
        let estimate_fn =
            || estimate_input_tokens(&retry_preamble, &history_with_prefill, prompt_params.prompt);
        let agent = self
            .client
            .agent(self.model_name)
            .preamble(&retry_preamble)
            .max_tokens(config.max_tokens)
            .temperature(config.temperature)
            .build();
        retry_completion_call(
            || async {
                agent
                    .completion(prompt_params.prompt, history_with_prefill.clone())
                    .await?
                    .send()
                    .await
            },
            estimate_fn,
            3,
        )
        .await
        .context("Failed to get retry completion from Claude")
    }

    pub async fn handle_retry_attempt<T, F>(
        &self,
        attempt_params: &RetryAttemptParams,
        prompt_params: &RetryPromptParams<'_, F>,
        conversation_history: &[RigMessage],
        config: &GenerationConfig,
        parse_fn: &impl Fn(&str) -> Result<T>,
        log_success: &impl Fn(&T),
    ) -> Result<(Option<T>, String, Vec<AgentUsage>)>
    where
        F: Fn(&str, &str) -> String,
    {
        tracing::warn!(
            "Retrying with error feedback (attempt {}/{})",
            attempt_params.attempt,
            attempt_params.max_retries
        );

        let (response, usage) = self
            .attempt_retry_with_feedback(prompt_params, conversation_history, config)
            .await?;

        tracing::info!("Raw retry response from Claude: {}", response);
        let (result, response_str) = process_retry_response(
            attempt_params,
            response,
            prompt_params,
            parse_fn,
            log_success,
        )?;
        Ok((result, response_str, usage))
    }

    pub async fn retry_with_error_feedback_tracked<T, F>(
        &self,
        prompt_params: &RetryPromptParams<'_, F>,
        conversation_history: &[RigMessage],
        config: &GenerationConfig,
        parse_fn: &impl Fn(&str) -> Result<T>,
        log_success: &impl Fn(&T),
    ) -> Result<(T, Vec<AgentUsage>)>
    where
        F: Fn(&str, &str) -> String,
    {
        let max_retries = 3;
        let mut last_failed_response = prompt_params.failed_response.to_string();
        let mut all_usage = Vec::new();

        for attempt in 1..=max_retries {
            let prompt_params_iter = RetryPromptParams {
                original_preamble: prompt_params.original_preamble,
                failed_response: &last_failed_response,
                prompt: prompt_params.prompt,
                preamble_builder: prompt_params.preamble_builder,
            };
            let attempt_params_iter = RetryAttemptParams {
                attempt,
                max_retries,
            };
            let (result, response, usage) = self
                .handle_retry_attempt(
                    &attempt_params_iter,
                    &prompt_params_iter,
                    conversation_history,
                    config,
                    parse_fn,
                    log_success,
                )
                .await?;

            all_usage.extend(usage);

            if let Some(parsed_response) = result {
                return Ok((parsed_response, all_usage));
            }
            last_failed_response = response;
        }

        Err(anyhow::anyhow!(
            "Failed to get valid response after {} retry attempts",
            max_retries
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rig::completion::message::{AssistantContent, Text, UserContent};

    #[test]
    fn test_create_usage_entry() {
        let usage = create_usage_entry(false, false, 100, 50);
        assert_eq!(usage.input_tokens, 100);
        assert_eq!(usage.output_tokens, 50);
        assert_eq!(usage.is_retry, false);
        assert_eq!(usage.is_estimate, false);
        assert!(usage.timestamp > 0);
    }

    #[test]
    fn test_create_usage_entry_retry() {
        let usage = create_usage_entry(true, false, 100, 50);
        assert_eq!(usage.is_retry, true);
        assert_eq!(usage.is_estimate, false);
    }

    #[test]
    fn test_create_usage_entry_estimate() {
        let usage = create_usage_entry(false, true, 100, 0);
        assert_eq!(usage.is_retry, false);
        assert_eq!(usage.is_estimate, true);
        assert_eq!(usage.output_tokens, 0);
    }

    #[test]
    fn test_extract_text_from_choice_single() {
        let text = Text {
            text: "Hello world".to_string(),
        };
        let choice = OneOrMany::one(AssistantContent::Text(text));
        let result = extract_text_from_choice(&choice).unwrap();
        assert_eq!(result, "Hello world");
    }

    #[test]
    fn test_extract_text_from_choice_no_text() {
        use rig::completion::message::{ToolCall, ToolFunction};
        let tool_call = ToolCall {
            id: "test".to_string(),
            function: ToolFunction {
                name: "test_fn".to_string(),
                arguments: serde_json::json!({}),
            },
        };
        let choice = OneOrMany::one(AssistantContent::ToolCall(tool_call));
        let result = extract_text_from_choice(&choice);
        assert!(result.is_err());
    }

    #[test]
    fn test_estimate_input_tokens() {
        let preamble = "You are a helpful assistant";
        let history = vec![];
        let prompt = "Hello";
        let tokens = estimate_input_tokens(preamble, &history, prompt);
        let expected = ((preamble.len() + prompt.len()) as f64 * 0.25).ceil() as u64;
        assert_eq!(tokens, expected);
    }

    #[test]
    fn test_estimate_input_tokens_with_history() {
        let preamble = "System";
        let history = vec![RigMessage::User {
            content: OneOrMany::one(UserContent::Text(Text {
                text: "First message".to_string(),
            })),
        }];
        let prompt = "Second";
        let tokens = estimate_input_tokens(preamble, &history, prompt);
        assert!(tokens > 0);
    }

    #[test]
    fn test_is_retryable_completion_error_provider() {
        let error = CompletionError::ProviderError("Overloaded".to_string());
        assert!(is_retryable_completion_error(&error));
    }

    #[test]
    fn test_is_retryable_completion_error_response() {
        let error = CompletionError::ResponseError("Empty response".to_string());
        assert!(is_retryable_completion_error(&error));
    }

    #[test]
    fn test_is_retryable_completion_error_json() {
        let error = CompletionError::JsonError(
            serde_json::from_str::<serde_json::Value>("invalid").unwrap_err(),
        );
        assert!(!is_retryable_completion_error(&error));
    }
}
