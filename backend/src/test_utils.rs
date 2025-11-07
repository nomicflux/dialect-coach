use anyhow::{Context, Result};
use dialect_coach_shared::{Dialect, Formality, TeachingMode};
use rig::completion::{
    Completion, Message as RigMessage, message::AssistantContent, message::Text,
    message::UserContent,
};
use rig::one_or_many::OneOrMany;
use serde::{Deserialize, Serialize};

use crate::agent_service::AgentService;
use crate::agent_service::response::GenerateResponseParams;
use crate::agent_service::retry::{estimate_input_tokens, retry_completion_call};
use crate::agent_service::util::contains_illegal_characters;
use crate::embedding_service::EmbeddingService;
use crate::qdrant_service::QdrantService;
use crate::rag_config::RAGConfig;
use crate::similarity::compute_corpus_similarity;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestStats {
    pub config: RAGConfig,
    pub dialect: String,
    pub cosine_mse_mean: f64,
    pub cosine_mse_median: f64,
    pub cosine_mse_variance: f64,
    pub l2_mse_mean: f64,
    pub l2_mse_median: f64,
    pub l2_mse_variance: f64,
}

/// Run self-chat test with given configuration
pub async fn run_self_chat_test(
    agent: &AgentService,
    qdrant: &QdrantService,
    embeddings: &EmbeddingService,
    config: RAGConfig,
    dialect: Dialect,
    formality: Formality,
    seed: &str,
) -> Result<TestStats> {
    let mut conversation_history: Vec<RigMessage> = Vec::new();
    let mut cosine_mse_values = Vec::new();
    let mut l2_mse_values = Vec::new();
    let mut current_message = seed.to_string();

    for _ in 0..10 {
        // Add user message to history BEFORE calling agent (matches app behavior)
        conversation_history.push(create_user_rig_message(&current_message));

        let params = GenerateResponseParams {
            user_message: &current_message,
            dialect,
            formality,
            teaching_mode: TeachingMode::Immersive,
            conversation_history: &conversation_history,
            learning_goals: &[],
            rag_config: &config,
        };
        let (result, _usage) = agent.generate_response(&params).await;
        let response = result?;

        let (cosine_mse, l2_mse) =
            compute_corpus_similarity(&response.response, dialect, qdrant, embeddings, 10).await?;
        cosine_mse_values.push(cosine_mse);
        l2_mse_values.push(l2_mse);

        // Add assistant response AFTER getting it (for next turn's context)
        conversation_history.push(create_assistant_rig_message(&response.response));

        // Generate next user message using agent
        current_message = generate_user_message(
            &agent.client,
            &agent.model_name,
            dialect,
            &conversation_history,
        )
        .await?;
    }

    let (cosine_mean, cosine_median, cosine_variance) = compute_stats(&cosine_mse_values);
    let (l2_mean, l2_median, l2_variance) = compute_stats(&l2_mse_values);

    Ok(TestStats {
        config,
        dialect: dialect.name().to_string(),
        cosine_mse_mean: cosine_mean,
        cosine_mse_median: cosine_median,
        cosine_mse_variance: cosine_variance,
        l2_mse_mean: l2_mean,
        l2_mse_median: l2_median,
        l2_mse_variance: l2_variance,
    })
}

fn create_user_rig_message(text: &str) -> RigMessage {
    RigMessage::User {
        content: OneOrMany::one(UserContent::Text(Text {
            text: text.to_string(),
        })),
    }
}

fn create_assistant_rig_message(text: &str) -> RigMessage {
    RigMessage::Assistant {
        content: OneOrMany::one(AssistantContent::Text(Text {
            text: text.to_string(),
        })),
    }
}

fn compute_stats(similarities: &[f64]) -> (f64, f64, f64) {
    let mean = calculate_mean(similarities);
    let median = calculate_median(similarities);
    let variance = calculate_variance(similarities, mean);
    (mean, median, variance)
}

fn calculate_mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

fn calculate_median(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mid = sorted.len() / 2;
    if sorted.len().is_multiple_of(2) {
        (sorted[mid - 1] + sorted[mid]) / 2.0
    } else {
        sorted[mid]
    }
}

fn calculate_variance(values: &[f64], mean: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64
}

pub fn build_user_message_retry_preamble(original_preamble: &str, failed_response: &str) -> String {
    let error_detail = if failed_response.trim().is_empty() {
        "Your previous response was EMPTY. The response must contain actual text content and cannot be empty."
    } else if contains_illegal_characters(failed_response) {
        "Your previous response contained ILLEGAL CHARACTERS (null bytes or control characters). The response must contain only valid text characters - no null bytes or control characters except newlines, tabs, and spaces."
    } else {
        "Your previous response was invalid. The response must be valid text content."
    };

    format!(
        "{}\n\n\
            # CRITICAL ERROR - SYSTEM CRASHED\n\
            {}\n\
            Previous response (INCORRECT): {}\n\
            You MUST respond with non-empty plain text only. You MUST NOT end the conversation.\n",
        original_preamble,
        error_detail,
        &failed_response.chars().take(200).collect::<String>()
    )
}

pub async fn attempt_user_message_retry(
    client: &rig::providers::anthropic::Client,
    model_name: &str,
    original_preamble: &str,
    failed_response: &str,
    conversation_history: &[RigMessage],
) -> Result<String> {
    let retry_preamble = build_user_message_retry_preamble(original_preamble, failed_response);
    let agent = client
        .agent(model_name)
        .preamble(&retry_preamble)
        .max_tokens(64)
        .temperature(0.3)
        .build();
    let prompt = "Continue the conversation naturally.";
    let estimate_fn = || estimate_input_tokens(&retry_preamble, conversation_history, prompt);
    let (result, _) = retry_completion_call(
        || async {
            agent
                .completion(prompt, conversation_history.to_vec())
                .await?
                .send()
                .await
        },
        estimate_fn,
        3,
    )
    .await;
    let response = result.context("Failed to get retry completion from Claude")?;
    Ok(response)
}

pub async fn retry_user_message_with_feedback(
    client: &rig::providers::anthropic::Client,
    model_name: &str,
    original_preamble: &str,
    failed_response: &str,
    conversation_history: &[RigMessage],
    dialect: Dialect,
) -> Result<String> {
    let max_retries = 3;
    let mut last_failed_response = failed_response.to_string();

    for attempt in 1..=max_retries {
        tracing::warn!(
            "Retrying user message generation for dialect {} (attempt {}/{})",
            dialect.name(),
            attempt,
            max_retries
        );

        let response = attempt_user_message_retry(
            client,
            model_name,
            original_preamble,
            &last_failed_response,
            conversation_history,
        )
        .await?;

        if !response.trim().is_empty() && !contains_illegal_characters(&response) {
            return Ok(response);
        }

        if response.trim().is_empty() {
            tracing::error!("Retry attempt {} returned empty response", attempt);
        }
        if contains_illegal_characters(&response) {
            tracing::error!(
                "Retry attempt {} returned response with illegal characters",
                attempt
            );
        }
        if attempt == max_retries {
            return Err(anyhow::anyhow!(
                "Generated user message is invalid after {} retry attempts",
                max_retries
            ));
        }
        last_failed_response = response;
    }

    Err(anyhow::anyhow!(
        "Failed to generate non-empty user message after {} retry attempts",
        max_retries
    ))
}

/// Generate a user message for self-chat testing
/// Returns a simple response as a native dialect speaker would say
pub async fn generate_user_message(
    client: &rig::providers::anthropic::Client,
    model_name: &str,
    dialect: Dialect,
    conversation_history: &[RigMessage],
) -> Result<String> {
    let dialect_name = dialect.name();
    let preamble = format!(
        "You are a native {} speaker having a casual conversation. \
            You MUST ALWAYS continue the conversation - NEVER indicate it has ended. If goodbyes were exchanged, ask a follow-up question or introduce a new topic. \
            Respond naturally and briefly (1-2 sentences). Your response must be non-empty. Respond with plain text only.",
        dialect_name
    );

    let agent = client
        .agent(model_name)
        .preamble(&preamble)
        .max_tokens(64)
        .temperature(0.3)
        .build();

    let prompt = "Continue the conversation naturally.";
    let estimate_fn = || estimate_input_tokens(&preamble, conversation_history, prompt);
    let (result, _) = retry_completion_call(
        || async {
            agent
                .completion(prompt, conversation_history.to_vec())
                .await?
                .send()
                .await
        },
        estimate_fn,
        3,
    )
    .await;
    let response = result.context("Failed to generate user message")?;

    if response.trim().is_empty() || contains_illegal_characters(&response) {
        return retry_user_message_with_feedback(
            client,
            model_name,
            &preamble,
            &response,
            conversation_history,
            dialect,
        )
        .await;
    }

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_stats() {
        let values = vec![0.5, 0.7, 0.9];
        let (mean, median, variance) = compute_stats(&values);
        assert!((mean - 0.7).abs() < 1e-6);
        assert!((median - 0.7).abs() < 1e-6);
        assert!(variance > 0.0);
    }

    #[test]
    fn test_calculate_mean() {
        assert_eq!(calculate_mean(&[]), 0.0);
        assert!((calculate_mean(&[1.0, 2.0, 3.0]) - 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_calculate_median() {
        assert_eq!(calculate_median(&[]), 0.0);
        assert!((calculate_median(&[1.0, 2.0, 3.0]) - 2.0).abs() < 1e-6);
        assert!((calculate_median(&[1.0, 2.0, 3.0, 4.0]) - 2.5).abs() < 1e-6);
    }

    #[test]
    fn test_calculate_variance() {
        assert_eq!(calculate_variance(&[], 0.0), 0.0);
        let values = vec![1.0, 2.0, 3.0];
        let mean = calculate_mean(&values);
        let variance = calculate_variance(&values, mean);
        assert!((variance - 0.6666666666666666).abs() < 1e-6);
    }
}
