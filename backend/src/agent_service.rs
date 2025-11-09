use anyhow::{Context, Result};
use dialect_coach_shared::{AgentUsage, Dialect, Explained, Formality, Mistake};
use rig::providers::anthropic::{CLAUDE_3_5_SONNET, ClientBuilder};
use std::sync::Arc;

use crate::embedding_service::EmbeddingService;
use crate::qdrant_service::QdrantService;

pub mod analysis;
pub mod learning;
pub mod response;
pub mod retry;
pub mod util;

use response::ResponseContext;
use util::contains_illegal_characters;

pub struct AgentService {
    pub(crate) client: rig::providers::anthropic::Client,
    pub(crate) model_name: String,
    qdrant: Arc<QdrantService>,
    embeddings: Arc<EmbeddingService>,
}

impl AgentService {
    /// Create new agent service from environment variables
    pub fn from_env(qdrant: Arc<QdrantService>, embeddings: Arc<EmbeddingService>) -> Result<Self> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .context("ANTHROPIC_API_KEY environment variable not set")?;
        let model_name =
            std::env::var("ANTHROPIC_MODEL").unwrap_or_else(|_| CLAUDE_3_5_SONNET.to_string());

        let client = ClientBuilder::new(&api_key)
            .anthropic_version("2023-06-01")
            .build();

        tracing::info!("Initialized Anthropic client with model: {}", model_name);

        Ok(Self {
            client,
            model_name,
            qdrant,
            embeddings,
        })
    }

    pub async fn generate_response(
        &self,
        params: &response::GenerateResponseParams<'_>,
    ) -> (
        Result<dialect_coach_shared::AgentResponse, anyhow::Error>,
        Vec<AgentUsage>,
        Vec<AgentUsage>,
    ) {
        let ctx = ResponseContext {
            client: self.client.clone(),
            model_name: self.model_name.clone(),
            qdrant: self.qdrant.clone(),
            embeddings: self.embeddings.clone(),
        };
        response::ResponseContext::generate_response(&ctx, params).await
    }

    pub async fn generate_analysis(
        &self,
        dialect: Dialect,
        msg: &String,
        mistakes: &[Mistake],
        explained: &[Explained],
        translated: &[dialect_coach_shared::Translated],
        exploratory: &[dialect_coach_shared::Exploratory],
    ) -> (
        Result<dialect_coach_shared::AgentAnalysis, anyhow::Error>,
        Vec<AgentUsage>,
    ) {
        let retry_ctx = retry::RetryContext {
            client: &self.client,
            model_name: &self.model_name,
        };
        crate::agent_service::analysis::generate_analysis(
            &retry_ctx,
            dialect,
            msg,
            mistakes,
            explained,
            translated,
            exploratory,
        )
        .await
    }

    /// Simple translation without RAG - for fast prompt translation
    pub async fn generate_simple_translation(
        &self,
        prompt: &str,
        dialect: Dialect,
        formality: Formality,
    ) -> Result<dialect_coach_shared::AgentResponse> {
        let ctx = ResponseContext {
            client: self.client.clone(),
            model_name: self.model_name.clone(),
            qdrant: self.qdrant.clone(),
            embeddings: self.embeddings.clone(),
        };
        response::ResponseContext::generate_simple_translation(&ctx, prompt, dialect, formality)
            .await
    }

    pub fn contains_illegal_characters(text: &str) -> bool {
        contains_illegal_characters(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedding_service::EmbeddingService;
    use crate::qdrant_service::QdrantService;

    #[tokio::test]
    #[ignore] // Requires API key
    async fn test_agent_initialization() {
        dotenvy::dotenv().ok();

        let qdrant_url = std::env::var("QDRANT_URL").unwrap();
        let qdrant_key = std::env::var("QDRANT_API_KEY").unwrap();
        let qdrant = QdrantService::new(&qdrant_url, &qdrant_key).await.unwrap();
        let embeddings = EmbeddingService::new().unwrap();

        let agent = AgentService::from_env(Arc::new(qdrant), Arc::new(embeddings)).unwrap();
        assert!(!agent.model_name.is_empty());
    }
}
