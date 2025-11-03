use anyhow::Result;
use dialect_coach_shared::{Dialect, Formality, TeachingMode};
use rig::completion::{
    Message as RigMessage, message::AssistantContent, message::Text, message::UserContent,
};
use rig::one_or_many::OneOrMany;
use serde::{Deserialize, Serialize};

use crate::agent_service::AgentService;
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

        let response = agent
            .generate_response(
                &current_message,
                dialect,
                formality,
                TeachingMode::Immersive,
                &conversation_history,
                &[],
                &config,
            )
            .await?;

        let (cosine_mse, l2_mse) =
            compute_corpus_similarity(&response.response, dialect, qdrant, embeddings, 10).await?;
        cosine_mse_values.push(cosine_mse);
        l2_mse_values.push(l2_mse);

        // Add assistant response AFTER getting it (for next turn's context)
        conversation_history.push(create_assistant_rig_message(&response.response));

        // Generate next user message using agent
        current_message = agent
            .generate_user_message(dialect, &conversation_history)
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
    if sorted.len() % 2 == 0 {
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

