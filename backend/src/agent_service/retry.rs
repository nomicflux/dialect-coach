use anyhow::{Context, Result};
use rig::completion::{Chat, CompletionError, Message as RigMessage, PromptError};

use super::util::{self, GenerationConfig};

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

pub fn is_retryable_error(error: &PromptError) -> bool {
    match error {
        PromptError::CompletionError(CompletionError::ProviderError(msg)) => {
            msg.contains("Overloaded")
                || msg.contains("Response contained no message")
                || msg.contains("Internal server error")
        }
        _ => false,
    }
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

pub fn calculate_backoff_delay(attempt: usize) -> tokio::time::Duration {
    tokio::time::Duration::from_secs(2_u64.pow(attempt as u32))
}

pub fn log_retry_attempt(
    attempt: usize,
    max_attempts: usize,
    error: &PromptError,
    delay: tokio::time::Duration,
) {
    if let PromptError::CompletionError(CompletionError::ProviderError(msg)) = error {
        if msg.contains("Response contained no message") {
            tracing::error!(
                "Empty response error detected (attempt {}/{}): Claude returned no message.",
                attempt,
                max_attempts
            );
        } else if msg.contains("Internal server error") {
            tracing::error!(
                "Internal server error detected (attempt {}/{}): Anthropic API server error.",
                attempt,
                max_attempts
            );
        } else if msg.contains("Overloaded") {
            tracing::error!(
                "Overloaded error detected (attempt {}/{}): Anthropic API is overloaded.",
                attempt,
                max_attempts
            );
        }
    }
    tracing::warn!(
        "API call failed (attempt {}/{}): {}. Retrying in {:?}...",
        attempt,
        max_attempts,
        error,
        delay
    );
}

pub async fn handle_retry_delay(attempt: usize, max_attempts: usize, error: &PromptError) {
    let delay = calculate_backoff_delay(attempt);
    log_retry_attempt(attempt, max_attempts, error, delay);
    tokio::time::sleep(delay).await;
}

pub async fn retry_chat_call<F, Fut>(chat_call: F, max_attempts: usize) -> Result<String>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<String, PromptError>>,
{
    let mut last_error = None;
    for attempt in 1..=max_attempts {
        match chat_call().await {
            Ok(response) => return Ok(response),
            Err(e) if is_retryable_error(&e) && attempt < max_attempts => {
                handle_retry_delay(attempt, max_attempts, &e).await;
                last_error = Some(e);
            }
            Err(e) => return Err(anyhow::anyhow!("API call failed: {}", e)),
        }
    }
    Err(anyhow::anyhow!(
        "API call failed after {} attempts: {}",
        max_attempts,
        last_error.unwrap()
    ))
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
    pub fn create_retry_agent(
        &self,
        original_preamble: &str,
        failed_response: &str,
        preamble_builder: &impl Fn(&str, &str) -> String,
        max_tokens: u64,
        temperature: f64,
    ) -> impl Chat {
        let retry_preamble = preamble_builder(original_preamble, failed_response);
        self.client
            .agent(self.model_name)
            .preamble(&retry_preamble)
            .max_tokens(max_tokens)
            .temperature(temperature)
            .build()
    }

    pub async fn attempt_retry_with_feedback<F>(
        &self,
        prompt_params: &RetryPromptParams<'_, F>,
        conversation_history: &[RigMessage],
        config: &GenerationConfig,
    ) -> Result<String>
    where
        F: Fn(&str, &str) -> String,
    {
        let retry_agent = self.create_retry_agent(
            prompt_params.original_preamble,
            prompt_params.failed_response,
            prompt_params.preamble_builder,
            config.max_tokens,
            config.temperature,
        );
        // conversation_history already has examples - just use it as-is
        let mut history_with_prefill = conversation_history.to_vec();
        history_with_prefill.push(util::create_prefilled_assistant_message());
        retry_chat_call(
            || retry_agent.chat(prompt_params.prompt, history_with_prefill.clone()),
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
    ) -> Result<(Option<T>, String)>
    where
        F: Fn(&str, &str) -> String,
    {
        tracing::warn!(
            "Retrying with error feedback (attempt {}/{})",
            attempt_params.attempt,
            attempt_params.max_retries
        );

        let response = self
            .attempt_retry_with_feedback(prompt_params, conversation_history, config)
            .await?;

        tracing::info!("Raw retry response from Claude: {}", response);
        process_retry_response(
            attempt_params,
            response,
            prompt_params,
            parse_fn,
            log_success,
        )
    }

    pub async fn retry_with_error_feedback_tracked<T, F>(
        &self,
        prompt_params: &RetryPromptParams<'_, F>,
        conversation_history: &[RigMessage],
        config: &GenerationConfig,
        parse_fn: &impl Fn(&str) -> Result<T>,
        log_success: &impl Fn(&T),
    ) -> Result<(T, u32)>
    where
        F: Fn(&str, &str) -> String,
    {
        let max_retries = 3;
        let mut last_failed_response = prompt_params.failed_response.to_string();

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
            let (result, response) = self
                .handle_retry_attempt(
                    &attempt_params_iter,
                    &prompt_params_iter,
                    conversation_history,
                    config,
                    parse_fn,
                    log_success,
                )
                .await?;

            if let Some(parsed_response) = result {
                return Ok((parsed_response, attempt as u32));
            }
            last_failed_response = response;
        }

        Err(anyhow::anyhow!(
            "Failed to get valid response after {} retry attempts",
            max_retries
        ))
    }
}
