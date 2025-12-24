use super::*;
use crate::agent_service::translation::{
    TranslationAgent, TranslationAgentOutput, TranslationAgentParams,
};
use anyhow::{Context, Result};
use dialect_coach_shared::{AgentUsage, PastLearningItems};
use rig::completion::Message as RigMessage;

use dialect_coach_shared::DialectDocument;

use super::config::{temperature_for_mode, tokens_per_mode};
use super::examples::build_conversation_history_with_examples;
use super::parsing::{
    apply_learning_output, build_simple_completion_request, log_response_success,
    sanitize_simple_json_response, try_parse_response,
};
use super::system_content::build_system_content;

use crate::agent_service::learning::{LearningAgent, LearningAgentOutput, LearningAgentParams};
use crate::agent_service::provider::CompletionRequest;
use crate::agent_service::retry::{
    RetryContext, build_retry_response_preamble, retry_completion_call,
};
use crate::agent_service::util::{GenerationConfig, contains_illegal_characters};

impl ResponseContext {
    pub(super) async fn attach_learning_items(
        &self,
        params: &GenerateResponseParams<'_>,
        parsed_response: dialect_coach_shared::AgentResponse,
    ) -> Result<(dialect_coach_shared::AgentResponse, Vec<AgentUsage>)> {
        let assistant_response = match Self::get_learning_analysis_target(params, &parsed_response)
        {
            Some(response) => response,
            None => return Ok((parsed_response, Vec::new())),
        };

        // Generate learning items
        let (learning_result, learning_usage) = self
            .generate_learning_content(params, &assistant_response)
            .await;

        // Generate translations
        let (translation_result, translation_usage) =
            self.generate_translation_content(params).await;

        // Merge results
        let mut all_usage = learning_usage;
        all_usage.extend(translation_usage);

        match (learning_result, translation_result) {
            (Ok(learning_output), Ok(translation_output)) => {
                let merged = merge_learning_outputs(learning_output, translation_output);
                Ok((apply_learning_output(parsed_response, merged), all_usage))
            }
            (Err(e), _) | (_, Err(e)) => Err(e),
        }
    }

    fn get_learning_analysis_target(
        params: &GenerateResponseParams<'_>,
        parsed_response: &dialect_coach_shared::AgentResponse,
    ) -> Option<String> {
        if params.teaching_mode == TeachingMode::ErrorFinding {
            params
                .conversation_history
                .iter()
                .rev()
                .find(|m| matches!(m, RigMessage::Assistant { .. }))
                .map(crate::agent_service::util::get_message_text)
        } else {
            Some(parsed_response.response.clone())
        }
    }

    async fn generate_learning_content(
        &self,
        params: &GenerateResponseParams<'_>,
        assistant_response: &str,
    ) -> (Result<LearningAgentOutput>, Vec<AgentUsage>) {
        let learning_agent = LearningAgent::new(self.learning_agent.clone());
        let learning_params = build_learning_params(params, assistant_response);
        learning_agent
            .generate_learning_items(&learning_params)
            .await
    }

    async fn generate_translation_content(
        &self,
        params: &GenerateResponseParams<'_>,
    ) -> (Result<TranslationAgentOutput>, Vec<AgentUsage>) {
        if params.teaching_mode == TeachingMode::Debug {
            return (Ok(TranslationAgentOutput::empty()), Vec::new());
        }

        let translation_agent = TranslationAgent::new(self.learning_agent.clone());
        let translation_params = TranslationAgentParams {
            user_message: params.user_message,
            dialect: params.dialect.dialect,
            formality: params.formality,
            past_translated: params.past_translated,
            language_option: params.language_option,
        };
        translation_agent
            .generate_translations(&translation_params)
            .await
    }

    async fn handle_successful_completion(
        &self,
        response: String,
        usage: Vec<AgentUsage>,
        params: &GenerateResponseParams<'_>,
        system_content: &str,
        history_with_prefill: Vec<RigMessage>,
        skip_learning: bool,
    ) -> (
        Result<dialect_coach_shared::AgentResponse, anyhow::Error>,
        Vec<AgentUsage>,
        Vec<AgentUsage>,
    ) {
        match self
            .handle_response_parsing(
                response,
                usage.clone(),
                params,
                system_content,
                history_with_prefill,
                skip_learning,
            )
            .await
        {
            Ok((agent_response, response_usage, learning_usage)) => {
                (Ok(agent_response), response_usage, learning_usage)
            }
            Err(e) => {
                log_processing_error(&e);
                (Err(e), usage, Vec::new())
            }
        }
    }

    async fn handle_response_parsing(
        &self,
        response: String,
        initial_usage: Vec<AgentUsage>,
        params: &GenerateResponseParams<'_>,
        system_content: &str,
        history_with_prefill: Vec<RigMessage>,
        skip_learning: bool,
    ) -> Result<(
        dialect_coach_shared::AgentResponse,
        Vec<AgentUsage>,
        Vec<AgentUsage>,
    )> {
        tracing::debug!(
            dialect = %params.dialect.dialect.name(),
            "Raw provider response: {}",
            response
        );
        match try_parse_response(&response, params.dialect.dialect) {
            Ok(parsed_response) => {
                handle_parse_success(self, parsed_response, initial_usage, params, skip_learning)
                    .await
            }
            Err(e) => {
                let initial_error = format!("{}", e);
                handle_parse_failure_with_retry(RetryHandlingParams {
                    ctx: self,
                    response,
                    initial_usage,
                    params,
                    system_content,
                    history_with_prefill,
                    skip_learning,
                    initial_error: &initial_error,
                })
                .await
            }
        }
    }

    pub async fn generate_response(
        &self,
        params: &GenerateResponseParams<'_>,
        skip_learning: bool,
    ) -> (
        Result<dialect_coach_shared::AgentResponse, anyhow::Error>,
        Vec<AgentUsage>,
        Vec<AgentUsage>,
    ) {
        if let Err(e) = validate_user_message(params.user_message, params.dialect.dialect) {
            return (Err(e), Vec::new(), Vec::new());
        }

        let (system_content, history_with_prefill, _primary_examples, _secondary_examples) =
            match prepare_generation_context(self, params).await {
                Ok(context) => context,
                Err(e) => return (Err(e), Vec::new(), Vec::new()),
            };

        let request = build_completion_request(params, &system_content, &history_with_prefill);
        let (result, usage) = execute_provider_completion(self, &request).await;

        match result {
            Ok(response) => {
                self.handle_successful_completion(
                    response,
                    usage,
                    params,
                    &system_content,
                    history_with_prefill.to_vec(),
                    skip_learning,
                )
                .await
            }
            Err(e) => {
                log_provider_error(self, &e);
                (
                    Err(e.context(format!(
                        "Failed to get completion from provider {} model {}",
                        self.response_agent.provider(),
                        self.response_agent.model()
                    ))),
                    usage,
                    Vec::new(),
                )
            }
        }
    }

    pub async fn generate_simple_response(
        &self,
        system_preamble: &str,
        prompt: &str,
        history: Vec<RigMessage>,
    ) -> Result<dialect_coach_shared::AgentResponse> {
        let request = build_simple_completion_request(system_preamble, prompt, &history);
        let (result, _) = retry_completion_call(self.response_agent.as_ref(), &request, 1).await;
        let text = result.context(format!(
            "Failed to get translation from provider {} model {}",
            self.response_agent.provider(),
            self.response_agent.model()
        ))?;
        let sanitized = sanitize_simple_json_response(&text)?;
        Ok(dialect_coach_shared::AgentResponse::from(sanitized))
    }
}

fn build_learning_params<'a>(
    params: &'a GenerateResponseParams<'_>,
    assistant_response: &'a str,
) -> LearningAgentParams<'a> {
    LearningAgentParams {
        user_message: params.user_message,
        assistant_response,
        dialect: params.dialect.dialect,
        formality: params.formality,
        teaching_mode: params.teaching_mode,
        past_mistakes: params.past_mistakes,
        past_explained: params.past_explained,
        past_translated: params.past_translated,
        past_exploratory: params.past_exploratory,
        language_option: params.language_option,
    }
}

fn log_processing_error(e: &anyhow::Error) {
    tracing::error!(
        error = %e,
        "Response processing failed after provider completion"
    );
}

async fn handle_parse_success(
    ctx: &ResponseContext,
    parsed_response: dialect_coach_shared::AgentResponse,
    initial_usage: Vec<AgentUsage>,
    params: &GenerateResponseParams<'_>,
    skip_learning: bool,
) -> Result<(
    dialect_coach_shared::AgentResponse,
    Vec<AgentUsage>,
    Vec<AgentUsage>,
)> {
    if contains_illegal_characters(&parsed_response.response) {
        tracing::error!(
            "Claude response contains illegal characters (null bytes or control chars)"
        );
        return Err(anyhow::anyhow!("Response contains illegal characters"));
    }
    log_response_success(params.dialect.dialect, &parsed_response);
    if skip_learning {
        Ok((parsed_response, initial_usage, Vec::new()))
    } else {
        attach_learning_with_error_handling(ctx, params, parsed_response, initial_usage).await
    }
}

async fn attach_learning_with_error_handling(
    ctx: &ResponseContext,
    params: &GenerateResponseParams<'_>,
    parsed_response: dialect_coach_shared::AgentResponse,
    initial_usage: Vec<AgentUsage>,
) -> Result<(
    dialect_coach_shared::AgentResponse,
    Vec<AgentUsage>,
    Vec<AgentUsage>,
)> {
    match ctx.attach_learning_items(params, parsed_response).await {
        Ok((final_response, learning_usage)) => Ok((final_response, initial_usage, learning_usage)),
        Err(e) => {
            tracing::error!(
                dialect = %params.dialect.dialect.name(),
                error = %e,
                "Failed to attach learning items to parsed response"
            );
            Err(e)
        }
    }
}
struct RetryHandlingParams<'a> {
    ctx: &'a ResponseContext,
    response: String,
    initial_usage: Vec<AgentUsage>,
    params: &'a GenerateResponseParams<'a>,
    system_content: &'a str,
    history_with_prefill: Vec<RigMessage>,
    skip_learning: bool,
    initial_error: &'a str,
}
async fn handle_parse_failure_with_retry(
    input: RetryHandlingParams<'_>,
) -> Result<(
    dialect_coach_shared::AgentResponse,
    Vec<AgentUsage>,
    Vec<AgentUsage>,
)> {
    let dialect = input.params.dialect.dialect;
    let teaching_mode = input.params.teaching_mode;
    let retry_ctx = RetryContext {
        agent: input.ctx.response_agent.clone(),
    };
    let parse_fn = move |response: &str| -> Result<dialect_coach_shared::AgentResponse> {
        try_parse_response(response, dialect)
    };
    let log_success = move |parsed: &dialect_coach_shared::AgentResponse| {
        log_response_success(dialect, parsed);
    };
    let preamble_builder = move |preamble: &str, failed: &str, error: &str| -> String {
        build_retry_response_preamble(preamble, failed, error)
    };
    let max_tokens = tokens_per_mode(&teaching_mode);
    let prompt_params = crate::agent_service::retry::RetryPromptParams {
        original_preamble: input.system_content,
        failed_response: &input.response,
        error_message: input.initial_error,
        prompt: input.params.user_message,
        preamble_builder: &preamble_builder,
    };
    let config = GenerationConfig {
        max_tokens,
        temperature: temperature_for_mode(&teaching_mode),
    };
    execute_retry_with_learning(
        input.ctx,
        retry_ctx,
        prompt_params,
        input.history_with_prefill,
        config,
        parse_fn,
        log_success,
        input.initial_usage,
        input.params,
        input.skip_learning,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn execute_retry_with_learning<F>(
    ctx: &ResponseContext,
    retry_ctx: RetryContext,
    prompt_params: crate::agent_service::retry::RetryPromptParams<'_, F>,
    history_with_prefill: Vec<RigMessage>,
    config: GenerationConfig,
    parse_fn: impl Fn(&str) -> Result<dialect_coach_shared::AgentResponse>,
    log_success: impl Fn(&dialect_coach_shared::AgentResponse),
    initial_usage: Vec<AgentUsage>,
    params: &GenerateResponseParams<'_>,
    skip_learning: bool,
) -> Result<(
    dialect_coach_shared::AgentResponse,
    Vec<AgentUsage>,
    Vec<AgentUsage>,
)>
where
    F: Fn(&str, &str, &str) -> String + Sync + Send,
{
    match retry_ctx
        .retry_with_error_feedback_tracked(
            &prompt_params,
            &history_with_prefill,
            &config,
            &parse_fn,
            &log_success,
        )
        .await
    {
        Ok((parsed_response, retry_usage)) => {
            let mut all_response_usage = initial_usage;
            all_response_usage.extend(retry_usage);
            if skip_learning {
                Ok((parsed_response, all_response_usage, Vec::new()))
            } else {
                attach_learning_after_retry(ctx, params, parsed_response, all_response_usage).await
            }
        }
        Err(e) => Err(e),
    }
}

async fn attach_learning_after_retry(
    ctx: &ResponseContext,
    params: &GenerateResponseParams<'_>,
    parsed_response: dialect_coach_shared::AgentResponse,
    all_response_usage: Vec<AgentUsage>,
) -> Result<(
    dialect_coach_shared::AgentResponse,
    Vec<AgentUsage>,
    Vec<AgentUsage>,
)> {
    match ctx.attach_learning_items(params, parsed_response).await {
        Ok((final_response, learning_usage)) => {
            Ok((final_response, all_response_usage, learning_usage))
        }
        Err(e) => Err(e),
    }
}

fn validate_user_message(message: &str, dialect: dialect_coach_shared::Dialect) -> Result<()> {
    if contains_illegal_characters(message) {
        tracing::error!(
            dialect = %dialect.name(),
            "User message contains illegal control characters; aborting response generation"
        );
        return Err(anyhow::anyhow!("User message contains illegal characters"));
    }
    Ok(())
}

async fn prepare_generation_context(
    ctx: &ResponseContext,
    params: &GenerateResponseParams<'_>,
) -> Result<(
    String,
    Vec<RigMessage>,
    Vec<DialectDocument>,
    Vec<DialectDocument>,
)> {
    let (primary_examples, secondary_examples) = ctx
        .collect_examples(
            params.user_message,
            params.conversation_history,
            params.dialect.clone(),
            params.formality,
            params.rag_config,
        )
        .await
        .map_err(|e| {
            tracing::error!(
                dialect = %params.dialect.dialect.name(),
                error = %e,
                "Failed to collect RAG examples for response generation"
            );
            e
        })?;

    let past_learning_items = PastLearningItems {
        mistakes: params.past_mistakes.to_vec(),
        explained: params.past_explained.to_vec(),
        translated: params.past_translated.to_vec(),
        exploratory: params.past_exploratory.to_vec(),
    };
    let system_content = build_system_content(
        params.dialect.clone(),
        params.formality,
        params.teaching_mode,
        params.learning_goals,
        &past_learning_items,
        params.user_gender,
        params.language_option,
        &params.active_plan,
        params.language_level,
    );
    tracing::debug!("System content sent to Claude:\n{}", system_content);
    let history_with_prefill = build_conversation_history_with_examples(
        params.conversation_history,
        &primary_examples,
        &secondary_examples,
        ctx.response_agent.provider(),
    );
    Ok((
        system_content,
        history_with_prefill,
        primary_examples,
        secondary_examples,
    ))
}

fn build_completion_request<'a>(
    params: &'a GenerateResponseParams<'_>,
    system_content: &'a str,
    history_with_prefill: &'a [RigMessage],
) -> CompletionRequest<'a> {
    CompletionRequest {
        preamble: system_content,
        prompt: params.user_message,
        history: history_with_prefill,
        max_tokens: tokens_per_mode(&params.teaching_mode),
        temperature: temperature_for_mode(&params.teaching_mode),
    }
}

async fn execute_provider_completion(
    ctx: &ResponseContext,
    request: &CompletionRequest<'_>,
) -> (Result<String>, Vec<AgentUsage>) {
    retry_completion_call(ctx.response_agent.as_ref(), request, 3).await
}

fn log_provider_error(ctx: &ResponseContext, e: &anyhow::Error) {
    let provider = ctx.response_agent.provider();
    let model = ctx.response_agent.model();
    let error_text = format!("{}", e);
    tracing::error!(
        provider = %provider,
        model = %model,
        error = %error_text,
        "Provider completion failed before response parsing"
    );
}

fn merge_learning_outputs(
    learning: LearningAgentOutput,
    translation: TranslationAgentOutput,
) -> LearningAgentOutput {
    LearningAgentOutput {
        mistakes: learning.mistakes,
        explained: learning.explained,
        translated: translation.translated,
        exploratory: learning.exploratory,
    }
}
