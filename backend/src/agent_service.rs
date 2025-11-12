use anyhow::{Result, anyhow};
use dialect_coach_shared::{AgentUsage, Dialect, Explained, Formality, Mistake};
use std::env;
use std::sync::Arc;

use crate::embedding_service::EmbeddingService;
use crate::qdrant_service::QdrantService;

pub mod analysis;
pub mod learning;
pub mod provider;
pub mod response;
pub mod retry;
pub mod util;

use provider::{ANTHROPIC_PROVIDER, CompletionAgent, CompletionAgentFactory, ProviderAgentConfig};
use response::ResponseContext;
use util::contains_illegal_characters;

fn channel_env(prefix: &str, suffix: &str) -> Option<String> {
    env::var(format!("{}_{}", prefix, suffix)).ok()
}

fn load_channel_agent(prefix: &str) -> Result<Arc<dyn CompletionAgent>> {
    let provider =
        channel_env(prefix, "PROVIDER").unwrap_or_else(|| ANTHROPIC_PROVIDER.to_string());
    match provider.as_str() {
        ANTHROPIC_PROVIDER => {
            let api_key = channel_env(prefix, "API_KEY")
                .or_else(|| env::var("ANTHROPIC_API_KEY").ok())
                .ok_or_else(|| {
                    anyhow!(
                        "Missing API key: set {}_API_KEY or ANTHROPIC_API_KEY",
                        prefix
                    )
                })?;
            let model = channel_env(prefix, "MODEL").or_else(|| env::var("ANTHROPIC_MODEL").ok());
            let config = ProviderAgentConfig::anthropic(api_key, model);
            let agent = CompletionAgentFactory::build(config)?;
            Ok(Arc::from(agent))
        }
        other => Err(anyhow!(
            "Unsupported provider '{}' configured for {} channel",
            other,
            prefix
        )),
    }
}

pub struct AgentService {
    pub(crate) response_agent: Arc<dyn CompletionAgent>,
    pub(crate) learning_agent: Arc<dyn CompletionAgent>,
    pub(crate) analysis_agent: Arc<dyn CompletionAgent>,
    qdrant: Arc<QdrantService>,
    embeddings: Arc<EmbeddingService>,
}

impl AgentService {
    /// Create new agent service from environment variables
    pub fn from_env(qdrant: Arc<QdrantService>, embeddings: Arc<EmbeddingService>) -> Result<Self> {
        let response_agent = load_channel_agent("RESPONSE")?;
        let learning_agent = load_channel_agent("LEARNING")?;
        let analysis_agent = load_channel_agent("ANALYSIS")?;

        tracing::info!(
            "Response agent configured: provider={}, model={}",
            response_agent.provider(),
            response_agent.model()
        );
        tracing::info!(
            "Learning agent configured: provider={}, model={}",
            learning_agent.provider(),
            learning_agent.model()
        );
        tracing::info!(
            "Analysis agent configured: provider={}, model={}",
            analysis_agent.provider(),
            analysis_agent.model()
        );

        Ok(Self {
            response_agent,
            learning_agent,
            analysis_agent,
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
            response_agent: self.response_agent.clone(),
            learning_agent: self.learning_agent.clone(),
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
            agent: self.analysis_agent.clone(),
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
            response_agent: self.response_agent.clone(),
            learning_agent: self.learning_agent.clone(),
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
        assert_eq!(agent.response_agent.provider(), ANTHROPIC_PROVIDER);
        assert!(!agent.response_agent.model().is_empty());
    }
}
