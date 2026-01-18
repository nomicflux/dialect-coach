use anyhow::{Context, Result, anyhow};
use dialect_coach_shared::AgentUsage;
use rig::completion::Message as RigMessage;
use std::sync::Arc;

use super::provider::{CompletionAgent, CompletionAgentError, CompletionRequest};
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
    pub error_message: &'a str,
    pub prompt: &'a str,
    pub preamble_builder: &'a F,
}

pub fn build_retry_preamble(
    original_preamble: &str,
    error_message: &str,
    additional_instructions: &str,
) -> String {
    let diagnosis = util::diagnose_json_error(error_message);

    format!(
        "{}\n\n\
        # YOUR PREVIOUS OUTPUT FAILED\n\n\
        {}\n\n\
        THIS TIME, {}",
        original_preamble, diagnosis, additional_instructions
    )
}

pub fn build_retry_response_preamble(original_preamble: &str, error_message: &str) -> String {
    build_retry_preamble(
        original_preamble,
        error_message,
        "output valid JSON with a non-empty 'response' field.",
    )
}

pub fn build_retry_learning_preamble(original_preamble: &str, error_message: &str) -> String {
    build_retry_preamble(
        original_preamble,
        error_message,
        "return valid JSON arrays for the learning items.",
    )
}

pub fn estimate_input_tokens(preamble: &str, history: &[RigMessage], prompt: &str) -> u64 {
    let preamble_chars = preamble.len();
    let history_chars: usize = history.iter().map(|msg| get_message_text(msg).len()).sum();
    let prompt_chars = prompt.len();
    let total_chars = preamble_chars + history_chars + prompt_chars;
    (total_chars as f64 * 0.25).ceil() as u64
}

fn create_usage_entry(
    agent: &dyn CompletionAgent,
    is_retry: bool,
    is_estimate: bool,
    input: u64,
    output: u64,
) -> AgentUsage {
    AgentUsage {
        timestamp: chrono::Utc::now().timestamp(),
        input_tokens: input,
        output_tokens: output,
        is_retry,
        is_estimate,
        provider: agent.provider().to_string(),
        model: agent.model().to_string(),
    }
}

enum AttemptResult {
    Success(String, AgentUsage),
    Retry(AgentUsage, anyhow::Error),
    Failed(AgentUsage, anyhow::Error),
}

enum AttemptFlow {
    Continue { retry_error: anyhow::Error },
    Complete(Result<String, anyhow::Error>),
}

fn process_attempt_result(
    usages: &mut Vec<AgentUsage>,
    attempt: usize,
    max_attempts: usize,
    provider: &str,
    model: &str,
    result: AttemptResult,
) -> AttemptFlow {
    match result {
        AttemptResult::Success(text, usage) => {
            usages.push(usage);
            AttemptFlow::Complete(Ok(text))
        }
        AttemptResult::Retry(usage, err) => {
            usages.push(usage);
            if attempt == max_attempts {
                AttemptFlow::Complete(Err(anyhow!(
                    "Failed after {} attempts for provider {} model {}",
                    max_attempts,
                    provider,
                    model
                )))
            } else {
                AttemptFlow::Continue { retry_error: err }
            }
        }
        AttemptResult::Failed(usage, err) => {
            usages.push(usage);
            AttemptFlow::Complete(Err(anyhow!(
                "Completion failed for provider {} model {}: {}",
                provider,
                model,
                err
            )))
        }
    }
}

async fn attempt_completion(
    agent: &dyn CompletionAgent,
    request: &CompletionRequest<'_>,
    estimate_tokens: u64,
    is_retry: bool,
) -> AttemptResult {
    match agent.completion(request).await {
        Ok(outcome) => {
            let usage = create_usage_entry(
                agent,
                is_retry,
                false,
                outcome.input_tokens,
                outcome.output_tokens,
            );
            AttemptResult::Success(outcome.text, usage)
        }
        Err(CompletionAgentError::Retryable(err)) => {
            let usage = create_usage_entry(agent, is_retry, true, estimate_tokens, 0);
            AttemptResult::Retry(usage, err)
        }
        Err(CompletionAgentError::Fatal(err)) => {
            let usage = create_usage_entry(agent, is_retry, true, estimate_tokens, 0);
            AttemptResult::Failed(usage, err)
        }
    }
}

fn is_rate_limit_error(error: &str) -> bool {
    let lower = error.to_lowercase();
    lower.contains("rate limit") || lower.contains("rate_limit") || lower.contains("429")
}

async fn handle_completion_retry_delay(attempt: usize, max: usize, error: &str) {
    let is_rate_limit = is_rate_limit_error(error);
    if is_rate_limit {
        tracing::warn!("Rate limit hit, retry {}/{}: {}", attempt, max, error);
    } else {
        tracing::warn!("Retry {}/{}: {}", attempt, max, error);
    }
    handle_completion_retry_delay_sleep(attempt, is_rate_limit).await;
}

#[cfg(not(test))]
async fn handle_completion_retry_delay_sleep(attempt: usize, is_rate_limit: bool) {
    // Base delay: 1, 2, 4 seconds (exponential starting at 1)
    // Rate limit: 2, 4, 8 seconds (double the normal delay)
    let base_delay = 1_u64 << (attempt - 1); // 1, 2, 4 for attempts 1, 2, 3
    let delay = if is_rate_limit {
        base_delay * 2
    } else {
        base_delay
    };
    tokio::time::sleep(tokio::time::Duration::from_secs(delay)).await;
}

#[cfg(test)]
async fn handle_completion_retry_delay_sleep(_attempt: usize, _is_rate_limit: bool) {}

pub async fn retry_completion_call(
    agent: &dyn CompletionAgent,
    request: &CompletionRequest<'_>,
    max_attempts: usize,
) -> (Result<String, anyhow::Error>, Vec<AgentUsage>) {
    let mut usages = Vec::new();
    let estimate_tokens = estimate_input_tokens(request.preamble, request.history, request.prompt);

    for attempt in 1..=max_attempts {
        let is_retry = attempt > 1;
        let flow = process_attempt_result(
            &mut usages,
            attempt,
            max_attempts,
            agent.provider(),
            agent.model(),
            attempt_completion(agent, request, estimate_tokens, is_retry).await,
        );

        match flow {
            AttemptFlow::Continue { retry_error } => {
                let message = retry_error.to_string();
                handle_completion_retry_delay(attempt, max_attempts, &message).await;
            }
            AttemptFlow::Complete(result) => return (result, usages),
        }
    }

    unreachable!("retry loop must exit on success or failure");
}

pub fn build_retry_failure_error(
    max_retries: usize,
    provider: &str,
    model: &str,
    last_error: &str,
    last_failed_response: &str,
) -> anyhow::Error {
    anyhow::anyhow!(
        "Provider {} model {} returned invalid JSON after {} retry attempts: {}. Last failed response: {}",
        provider,
        model,
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

pub fn process_retry_response<T>(
    attempt_params: &RetryAttemptParams,
    response: String,
    provider: &str,
    model: &str,
    parse_fn: &impl Fn(&str) -> Result<T>,
    log_success: &impl Fn(&T),
) -> Result<(Option<T>, String, Option<String>), anyhow::Error> {
    match parse_fn(&response) {
        Ok(parsed_response) => {
            log_success(&parsed_response);
            Ok((Some(parsed_response), response, None))
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
                    provider,
                    model,
                    &format!("{}", e),
                    &response,
                ));
            }
            Ok((None, response, Some(format!("{}", e))))
        }
    }
}

pub struct RetryContext {
    pub agent: Arc<dyn CompletionAgent>,
}

impl RetryContext {
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
            prompt_params.error_message,
        );
        let request = CompletionRequest {
            preamble: &retry_preamble,
            prompt: prompt_params.prompt,
            history: conversation_history,
            max_tokens: config.max_tokens,
            temperature: config.temperature,
        };
        let (result, usage) = retry_completion_call(self.agent.as_ref(), &request, 3).await;
        let response = result.context(format!(
            "Failed to get retry completion from provider {} model {}",
            self.agent.provider(),
            self.agent.model()
        ))?;
        Ok((response, usage))
    }

    pub async fn handle_retry_attempt<T, F>(
        &self,
        attempt_params: &RetryAttemptParams,
        prompt_params: &RetryPromptParams<'_, F>,
        conversation_history: &[RigMessage],
        config: &GenerationConfig,
        parse_fn: &impl Fn(&str) -> Result<T>,
        log_success: &impl Fn(&T),
    ) -> Result<(Option<T>, String, Option<String>, Vec<AgentUsage>)>
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
        let (result, response_str, error_msg) = process_retry_response(
            attempt_params,
            response,
            self.agent.provider(),
            self.agent.model(),
            parse_fn,
            log_success,
        )?;
        Ok((result, response_str, error_msg, usage))
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
        let mut last_error_message = prompt_params.error_message.to_string();
        let mut all_usage = Vec::new();

        for attempt in 1..=max_retries {
            let prompt_params_iter = RetryPromptParams {
                original_preamble: prompt_params.original_preamble,
                error_message: &last_error_message,
                prompt: prompt_params.prompt,
                preamble_builder: prompt_params.preamble_builder,
            };
            let attempt_params_iter = RetryAttemptParams {
                attempt,
                max_retries,
            };
            let (result, _response, error_msg_opt, usage) = self
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
            if let Some(msg) = error_msg_opt {
                last_error_message = msg;
            }
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
    use anyhow::anyhow;
    use async_trait::async_trait;
    use std::sync::Mutex;

    use super::super::provider::{CompletionOutcome, CompletionRequest};
    use rig::completion::message::{Text, UserContent};

    struct StubAgent {
        provider: String,
        model: String,
        responses: Mutex<Vec<Result<CompletionOutcome, CompletionAgentError>>>,
    }

    impl StubAgent {
        fn new(responses: Vec<Result<CompletionOutcome, CompletionAgentError>>) -> Self {
            Self {
                provider: "stub".to_string(),
                model: "test".to_string(),
                responses: Mutex::new(responses),
            }
        }
    }

    #[async_trait]
    impl CompletionAgent for StubAgent {
        fn provider(&self) -> &str {
            &self.provider
        }

        fn model(&self) -> &str {
            &self.model
        }

        async fn completion(
            &self,
            _: &CompletionRequest<'_>,
        ) -> Result<CompletionOutcome, CompletionAgentError> {
            let mut guard = self.responses.lock().unwrap();
            if guard.is_empty() {
                return Err(CompletionAgentError::fatal(anyhow!("Stub call exhausted")));
            }
            guard.remove(0)
        }
    }

    fn sample_usage(is_retry: bool) -> AgentUsage {
        AgentUsage {
            timestamp: 0,
            input_tokens: 1,
            output_tokens: 0,
            is_retry,
            is_estimate: true,
            provider: "stub".to_string(),
            model: "test".to_string(),
        }
    }

    #[test]
    fn test_process_attempt_result_sequence() {
        let mut usages = Vec::new();
        let provider = "stub";
        let model = "test";

        let flow1 = process_attempt_result(
            &mut usages,
            1,
            3,
            provider,
            model,
            AttemptResult::Retry(sample_usage(false), anyhow!("fail 1")),
        );
        assert!(matches!(flow1, AttemptFlow::Continue { .. }));
        assert_eq!(usages.len(), 1);

        let flow2 = process_attempt_result(
            &mut usages,
            2,
            3,
            provider,
            model,
            AttemptResult::Retry(sample_usage(true), anyhow!("fail 2")),
        );
        assert!(matches!(flow2, AttemptFlow::Continue { .. }));
        assert_eq!(usages.len(), 2);

        match process_attempt_result(
            &mut usages,
            3,
            3,
            provider,
            model,
            AttemptResult::Failed(sample_usage(true), anyhow!("fatal")),
        ) {
            AttemptFlow::Complete(Err(err)) => {
                assert!(err.to_string().contains("Completion failed"))
            }
            _ => panic!("expected final failure"),
        }

        assert_eq!(usages.len(), 3);
    }

    #[test]
    fn test_create_usage_entry() {
        let agent = StubAgent::new(vec![Ok(CompletionOutcome {
            text: "ok".to_string(),
            input_tokens: 0,
            output_tokens: 0,
        })]);
        let usage = create_usage_entry(&agent, false, false, 100, 50);
        assert_eq!(usage.input_tokens, 100);
        assert_eq!(usage.output_tokens, 50);
        assert!(!usage.is_retry);
        assert!(!usage.is_estimate);
        assert_eq!(usage.provider, "stub");
        assert_eq!(usage.model, "test");
        assert!(usage.timestamp > 0);
    }

    #[test]
    fn test_create_usage_entry_retry() {
        let agent = StubAgent::new(vec![Ok(CompletionOutcome {
            text: "ok".to_string(),
            input_tokens: 0,
            output_tokens: 0,
        })]);
        let usage = create_usage_entry(&agent, true, false, 100, 50);
        assert!(usage.is_retry);
        assert!(!usage.is_estimate);
    }

    #[test]
    fn test_create_usage_entry_estimate() {
        let agent = StubAgent::new(vec![Ok(CompletionOutcome {
            text: "ok".to_string(),
            input_tokens: 0,
            output_tokens: 0,
        })]);
        let usage = create_usage_entry(&agent, false, true, 100, 0);
        assert!(!usage.is_retry);
        assert!(usage.is_estimate);
        assert_eq!(usage.output_tokens, 0);
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
            content: rig::one_or_many::OneOrMany::one(UserContent::Text(Text {
                text: "First message".to_string(),
            })),
        }];
        let prompt = "Second";
        let tokens = estimate_input_tokens(preamble, &history, prompt);
        assert!(tokens > 0);
    }

    #[tokio::test]
    async fn test_retry_completion_call_returns_usage_on_error() {
        let agent = StubAgent::new(vec![
            Err(CompletionAgentError::retryable(anyhow!("fail 1"))),
            Err(CompletionAgentError::retryable(anyhow!("fail 2"))),
            Err(CompletionAgentError::fatal(anyhow!("fatal"))),
        ]);
        let request = CompletionRequest {
            preamble: "test",
            prompt: "prompt",
            history: &[],
            max_tokens: 10,
            temperature: 0.0,
        };
        let (result, usage) = retry_completion_call(&agent, &request, 3).await;

        assert!(result.is_err());
        assert!(!usage.is_empty());
        assert_eq!(usage.len(), 3);
        assert!(usage.iter().all(|u| u.is_estimate));
        assert!(usage.iter().all(|u| u.input_tokens > 0));
    }
}
