use anyhow::{Context, Result};
use dialect_coach_shared::{Dialect, DialectDocument};
use rig::providers::anthropic::{ClientBuilder, CLAUDE_3_5_SONNET};
use rig::completion::Prompt;
use std::sync::Arc;

use crate::qdrant_service::QdrantService;

/// Agent service for AI-powered dialect coaching
pub struct AgentService {
    client: rig::providers::anthropic::Client,
    model_name: String,
    qdrant: Arc<QdrantService>,
}

impl AgentService {
    /// Create new agent service from environment variables
    pub fn from_env(qdrant: Arc<QdrantService>) -> Result<Self> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .context("ANTHROPIC_API_KEY environment variable not set")?;
        let model_name = std::env::var("ANTHROPIC_MODEL")
            .unwrap_or_else(|_| CLAUDE_3_5_SONNET.to_string());

        let client = ClientBuilder::new(&api_key)
            .anthropic_version("2023-06-01")
            .build();

        tracing::info!("Initialized Anthropic client with model: {}", model_name);

        Ok(Self {
            client,
            model_name,
            qdrant,
        })
    }

    /// Generate response for user message with RAG context
    pub async fn generate_response(
        &self,
        user_message: &str,
        dialect: Dialect,
        conversation_history: &[String],
    ) -> Result<String> {
        // TODO: Generate embedding for user message
        // For now, we'll skip RAG and use direct agent response
        // In production, we need to:
        // 1. Generate embedding for user_message
        // 2. Call self.qdrant.search_dialect_examples(embedding, dialect, 5)
        // 3. Include examples in prompt

        // Build conversation context
        let history_context = if conversation_history.is_empty() {
            String::new()
        } else {
            format!(
                "\n\nPrevious conversation:\n{}",
                conversation_history.join("\n")
            )
        };

        // Build system prompt with dialect coaching instructions
        let system_content = format!(
            "You are a dialect coach specializing in {}. \
            Your role is to help users practice and improve their skills in this specific dialect. \
            Respond naturally in {}, providing corrections and suggestions when appropriate. \
            Keep responses conversational and encouraging.{}",
            dialect.name(),
            dialect.name(),
            history_context
        );

        // Create agent with preamble
        let agent = self
            .client
            .agent(&self.model_name)
            .preamble(&system_content)
            .max_tokens(1024)
            .build();

        // Generate response
        let response = agent
            .prompt(user_message)
            .await
            .context("Failed to get completion from Claude")?;

        tracing::info!(
            "Generated response for dialect {} ({} chars)",
            dialect.name(),
            response.len()
        );

        Ok(response)
    }

    /// Get dialect examples from RAG (utility function for testing)
    pub async fn get_dialect_examples(
        &self,
        _query_embedding: Vec<f32>,
        dialect: Dialect,
        limit: usize,
    ) -> Result<Vec<DialectDocument>> {
        // Placeholder - will be replaced with actual embedding generation
        self.qdrant
            .search_dialect_examples(vec![0.0; 768], dialect, limit)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires API key
    async fn test_agent_initialization() {
        dotenvy::dotenv().ok();

        let qdrant_url = std::env::var("QDRANT_URL").unwrap();
        let qdrant_key = std::env::var("QDRANT_API_KEY").unwrap();
        let qdrant = QdrantService::new(&qdrant_url, &qdrant_key)
            .await
            .unwrap();

        let agent = AgentService::from_env(Arc::new(qdrant)).unwrap();
        assert!(!agent.model_name.is_empty());
    }
}
