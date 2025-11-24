use anyhow::{Result, anyhow};
use dialect_coach_shared::{AgentUsage, Dialect, Explained, LanguageOption, Mistake};
use std::env;
use std::sync::Arc;

use crate::embedding_service::EmbeddingService;
use crate::qdrant_service::QdrantService;

pub mod analysis;
pub mod language_instructions;
pub mod learning;
pub mod provider;
pub mod response;
pub mod retry;
pub mod util;

use provider::{
    ANTHROPIC_PROVIDER, CompletionAgent, CompletionAgentFactory, OPENAI_PROVIDER,
    ProviderAgentConfig,
};
use response::ResponseContext;
use util::contains_illegal_characters;

fn channel_env(prefix: &str, suffix: &str) -> Option<String> {
    env::var(format!("{}_{}", prefix, suffix)).ok()
}

fn load_reasoning_budget(channel_prefix: &str) -> u32 {
    channel_env(channel_prefix, "REASONING_BUDGET")
        .and_then(|v| v.parse::<u32>().ok())
        .or_else(|| {
            env::var("OPENAI_REASONING_BUDGET")
                .ok()
                .and_then(|v| v.parse::<u32>().ok())
        })
        .unwrap_or(200)
}

fn load_channel_agent(prefix: &str) -> Result<Arc<dyn CompletionAgent>> {
    let provider =
        channel_env(prefix, "PROVIDER").unwrap_or_else(|| ANTHROPIC_PROVIDER.to_string());
    let reasoning_budget = load_reasoning_budget(prefix);
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
            let config = ProviderAgentConfig::anthropic(api_key, model, reasoning_budget);
            let agent = CompletionAgentFactory::build(config)?;
            Ok(Arc::from(agent))
        }
        OPENAI_PROVIDER => {
            let api_key = channel_env(prefix, "API_KEY")
                .or_else(|| env::var("OPENAI_API_KEY").ok())
                .ok_or_else(|| {
                    anyhow!("Missing API key: set {}_API_KEY or OPENAI_API_KEY", prefix)
                })?;
            let model = channel_env(prefix, "MODEL").or_else(|| env::var("OPENAI_MODEL").ok());
            let config = ProviderAgentConfig::openai(api_key, model, reasoning_budget);
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
        language_option: &Option<LanguageOption>,
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
            language_option,
        )
        .await
    }

    pub async fn generate_simple_response(
        &self,
        system_preamble: &str,
        prompt: &str,
    ) -> Result<dialect_coach_shared::AgentResponse> {
        let ctx = ResponseContext {
            response_agent: self.response_agent.clone(),
            learning_agent: self.learning_agent.clone(),
            qdrant: self.qdrant.clone(),
            embeddings: self.embeddings.clone(),
        };
        response::ResponseContext::generate_simple_response(&ctx, system_preamble, prompt, Vec::new()).await
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
    use serial_test::serial;

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

    #[test]
    #[serial]
    fn test_load_channel_agent_anthropic_uses_default_fallback() {
        unsafe {
            // Clean up any env vars from previous tests first
            std::env::remove_var("RESPONSE_PROVIDER");
            std::env::remove_var("RESPONSE_API_KEY");
            std::env::remove_var("RESPONSE_MODEL");
            std::env::remove_var("OPENAI_API_KEY");

            // Now set up for this test
            std::env::set_var("ANTHROPIC_API_KEY", "test-key");
        }

        let result = load_channel_agent("RESPONSE");
        if let Err(ref e) = result {
            eprintln!("load_channel_agent failed: {}", e);
        }
        assert!(result.is_ok(), "Failed to load agent: {:?}", result.as_ref().err());
        let agent = result.unwrap();
        assert_eq!(agent.provider(), ANTHROPIC_PROVIDER);

        unsafe {
            // Comprehensive cleanup
            std::env::remove_var("ANTHROPIC_API_KEY");
            std::env::remove_var("RESPONSE_PROVIDER");
            std::env::remove_var("RESPONSE_API_KEY");
            std::env::remove_var("RESPONSE_MODEL");
            std::env::remove_var("OPENAI_API_KEY");
        }
    }

    #[test]
    #[serial]
    fn test_load_channel_agent_openai_with_channel_vars() {
        unsafe {
            std::env::set_var("OPENAI_API_KEY", "sk-test-key");
            std::env::set_var("RESPONSE_PROVIDER", "openai");
            std::env::remove_var("RESPONSE_API_KEY");
            std::env::remove_var("ANTHROPIC_API_KEY");
        }

        let result = load_channel_agent("RESPONSE");
        assert!(result.is_ok());
        let agent = result.unwrap();
        assert_eq!(agent.provider(), OPENAI_PROVIDER);

        unsafe {
            // Comprehensive cleanup
            std::env::remove_var("OPENAI_API_KEY");
            std::env::remove_var("RESPONSE_PROVIDER");
            std::env::remove_var("RESPONSE_API_KEY");
            std::env::remove_var("RESPONSE_MODEL");
            std::env::remove_var("ANTHROPIC_API_KEY");
        }
    }

    #[test]
    #[serial]
    fn test_load_channel_agent_openai_custom_model() {
        unsafe {
            std::env::set_var("OPENAI_API_KEY", "sk-test-key");
            std::env::set_var("RESPONSE_PROVIDER", "openai");
            std::env::set_var("RESPONSE_MODEL", "gpt-4-turbo");
            std::env::remove_var("ANTHROPIC_API_KEY");
        }

        let result = load_channel_agent("RESPONSE");
        assert!(result.is_ok());
        let agent = result.unwrap();
        assert_eq!(agent.provider(), OPENAI_PROVIDER);
        assert_eq!(agent.model(), "gpt-4-turbo");

        unsafe {
            // Comprehensive cleanup
            std::env::remove_var("OPENAI_API_KEY");
            std::env::remove_var("RESPONSE_PROVIDER");
            std::env::remove_var("RESPONSE_MODEL");
            std::env::remove_var("RESPONSE_API_KEY");
            std::env::remove_var("ANTHROPIC_API_KEY");
        }
    }

    #[test]
    #[serial]
    fn test_load_reasoning_budget_channel_specific() {
        unsafe {
            std::env::set_var("RESPONSE_REASONING_BUDGET", "300");
        }
        let budget = load_reasoning_budget("RESPONSE");
        assert_eq!(budget, 300);
        unsafe {
            std::env::remove_var("RESPONSE_REASONING_BUDGET");
        }
    }

    #[test]
    #[serial]
    fn test_load_reasoning_budget_global_fallback() {
        unsafe {
            std::env::set_var("OPENAI_REASONING_BUDGET", "250");
            std::env::remove_var("LEARNING_REASONING_BUDGET");
        }
        let budget = load_reasoning_budget("LEARNING");
        assert_eq!(budget, 250);
        unsafe {
            std::env::remove_var("OPENAI_REASONING_BUDGET");
        }
    }

    #[test]
    #[serial]
    fn test_load_reasoning_budget_default_fallback() {
        unsafe {
            std::env::remove_var("ANALYSIS_REASONING_BUDGET");
            std::env::remove_var("OPENAI_REASONING_BUDGET");
        }
        let budget = load_reasoning_budget("ANALYSIS");
        assert_eq!(budget, 200);
    }

    #[test]
    #[serial]
    fn test_load_reasoning_budget_precedence() {
        unsafe {
            std::env::set_var("OPENAI_REASONING_BUDGET", "100");
            std::env::set_var("RESPONSE_REASONING_BUDGET", "400");
        }
        let budget = load_reasoning_budget("RESPONSE");
        assert_eq!(budget, 400);
        unsafe {
            std::env::remove_var("OPENAI_REASONING_BUDGET");
            std::env::remove_var("RESPONSE_REASONING_BUDGET");
        }
    }
}
