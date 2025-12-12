use anyhow::Result;
use dialect_coach_shared::{AgentResponse, Dialect};
use rig::completion::Message as RigMessage;

use crate::agent_service::learning::LearningAgentOutput;
use crate::agent_service::provider::CompletionRequest;
use crate::agent_service::util::{clean_response, normalize_json_response};

pub fn apply_learning_output(
    mut base: AgentResponse,
    output: LearningAgentOutput,
) -> AgentResponse {
    base.mistakes = Some(output.mistakes);
    base.explained = Some(output.explained);
    base.translated = Some(output.translated);
    base.exploratory = Some(output.exploratory);
    base
}

pub fn try_parse_response(response: &str, _dialect: Dialect) -> Result<AgentResponse> {
    let normalized = normalize_json_response(response);
    let parsed: AgentResponse = serde_json::from_str(&normalized).map_err(|e| {
        tracing::warn!(
            "JSON parse error: {}. First 200 chars: {}",
            e,
            &response.chars().take(200).collect::<String>()
        );
        anyhow::anyhow!("JSON parse failed: {}", e)
    })?;

    if parsed.response.is_empty() {
        tracing::warn!("Response field is empty");
        return Err(anyhow::anyhow!("Response field must be non-empty"));
    }

    Ok(parsed)
}

pub fn log_response_success(dialect: Dialect, parsed_response: &AgentResponse) {
    tracing::info!(
        "Generated response for dialect {} ({} chars)",
        dialect.name(),
        parsed_response.response.len()
    );
}

pub(super) fn build_simple_completion_request<'a>(
    system_preamble: &'a str,
    prompt: &'a str,
    history: &'a [RigMessage],
) -> CompletionRequest<'a> {
    CompletionRequest {
        preamble: system_preamble,
        prompt,
        history,
        max_tokens: 1024,
        temperature: 0.0,
    }
}

pub(super) fn sanitize_simple_json_response(response_text: &str) -> Result<String> {
    let cleaned = clean_response(response_text);
    let trimmed = cleaned.trim();

    if trimmed.is_empty() {
        return Err(anyhow::anyhow!(
            "Simple response was empty; expected content"
        ));
    }

    Ok(trimmed.to_string())
}
