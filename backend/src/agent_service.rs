use anyhow::{Context, Result};
use dialect_coach_shared::{AgentUsage, config::LlmConfig};
use std::env;
use std::sync::Arc;

use crate::embedding_service::EmbeddingService;
use crate::qdrant_service::QdrantService;

pub mod activity;
pub mod analysis;
pub mod dialect_pronunciation_guides;
pub mod enrichment;
pub mod error_finding;
pub mod keyword_extraction;
pub mod language_instructions;
pub mod learning;
pub mod ocr;
pub mod planning;
pub mod pronunciation;
pub mod provider;
pub mod response;
pub mod retry;
pub mod translation;
pub mod util;

use provider::{CompletionAgent, CompletionAgentFactory, ProviderAgentConfig};
use response::ResponseContext;
use util::contains_illegal_characters;

fn load_channel_agent(
    llm_config: &LlmConfig,
    channel_name: &str,
    read_env: impl Fn(&str) -> Option<String>,
) -> Result<(Arc<dyn CompletionAgent>, ProviderAgentConfig)> {
    let channel = llm_config.resolve_channel(channel_name)?;
    let api_key = channel.api_key(read_env)?;
    let agent = CompletionAgentFactory::build(&channel, api_key.as_deref())
        .with_context(|| format!("Failed to initialize {channel_name} channel"))?;
    let config = ProviderAgentConfig {
        provider: agent.provider().to_owned(),
        model: channel.model,
        base_url: Some(channel.base_url),
        api_key: api_key.unwrap_or_default(),
        reasoning_budget: channel
            .reasoning
            .as_ref()
            .map_or(0, |options| options.budget),
        reasoning_effort: channel
            .reasoning
            .map_or_else(|| "low".to_owned(), |options| options.effort),
    };
    Ok((agent, config))
}

#[derive(Clone)]
pub struct AgentService {
    pub response_agent: Arc<dyn CompletionAgent>,
    pub learning_agent: Arc<dyn CompletionAgent>,
    pub analysis_agent: Arc<dyn CompletionAgent>,
    pub planning_agent: Arc<dyn CompletionAgent>,
    pub planning_config: ProviderAgentConfig,
    pub pronunciation_agent: Arc<dyn CompletionAgent>,
    pub qdrant: Arc<QdrantService>,
    pub embeddings: Arc<EmbeddingService>,
    pub keyword_extractor: Arc<keyword_extraction::KeywordExtractor>,
}

impl AgentService {
    pub fn from_config(
        llm_config: &LlmConfig,
        qdrant: Arc<QdrantService>,
        embeddings: Arc<EmbeddingService>,
    ) -> Result<Self> {
        let load_agent = |name| load_channel_agent(llm_config, name, |key| env::var(key).ok());
        let response_agent = load_agent("response")?.0;
        let learning_agent = load_agent("learning")?.0;
        let analysis_agent = load_agent("analysis")?.0;
        let (planning_agent, planning_config) = load_agent("planning")?;
        let pronunciation_agent = load_agent("pronunciation")?.0;

        let keyword_extractor = Arc::new(
            keyword_extraction::KeywordExtractor::new()
                .context("Failed to create keyword extractor")?,
        );

        Ok(Self {
            response_agent,
            learning_agent,
            analysis_agent,
            planning_agent,
            planning_config,
            pronunciation_agent,
            qdrant,
            embeddings,
            keyword_extractor,
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
            pronunciation_agent: self.pronunciation_agent.clone(),
            qdrant: self.qdrant.clone(),
            embeddings: self.embeddings.clone(),
            keyword_extractor: self.keyword_extractor.clone(),
        };
        response::ResponseContext::generate_response(&ctx, params, false).await
    }

    pub async fn generate_response_for_action(
        &self,
        params: &response::GenerateResponseParams<'_>,
    ) -> (
        Result<dialect_coach_shared::AgentResponse, anyhow::Error>,
        Vec<AgentUsage>,
    ) {
        let ctx = ResponseContext {
            response_agent: self.response_agent.clone(),
            learning_agent: self.learning_agent.clone(),
            pronunciation_agent: self.pronunciation_agent.clone(),
            qdrant: self.qdrant.clone(),
            embeddings: self.embeddings.clone(),
            keyword_extractor: self.keyword_extractor.clone(),
        };
        let (result, response_usage, _learning_usage) =
            response::ResponseContext::generate_response(&ctx, params, true).await;
        (result, response_usage)
    }

    pub async fn generate_analysis(
        &self,
        params: analysis::AnalysisRequestParams<'_>,
    ) -> (
        Result<dialect_coach_shared::AgentAnalysis, anyhow::Error>,
        Vec<AgentUsage>,
    ) {
        let retry_ctx = retry::RetryContext {
            agent: self.analysis_agent.clone(),
        };
        crate::agent_service::analysis::generate_analysis(
            crate::agent_service::analysis::AnalysisParams {
                retry_ctx: &retry_ctx,
                activity: params.activity,
                allowed: params.allowed,
                dialect: params.dialect,
                msg: params.msg,
                mistakes: params.mistakes,
                explained: params.explained,
                translated: params.translated,
                exploratory: params.exploratory,
                language_option: params.language_option,
            },
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
            pronunciation_agent: self.pronunciation_agent.clone(),
            qdrant: self.qdrant.clone(),
            embeddings: self.embeddings.clone(),
            keyword_extractor: self.keyword_extractor.clone(),
        };
        response::ResponseContext::generate_simple_response(
            &ctx,
            system_preamble,
            prompt,
            Vec::new(),
        )
        .await
    }

    pub fn contains_illegal_characters(text: &str) -> bool {
        contains_illegal_characters(text)
    }

    pub async fn enrich_learning_item(
        &self,
        req: dialect_coach_shared::EnrichRequest,
    ) -> anyhow::Result<dialect_coach_shared::EnrichResponse> {
        let retry_ctx = retry::RetryContext {
            agent: self.learning_agent.clone(),
        };
        enrichment::enrich_learning_item(&retry_ctx, &req).await
    }

    pub async fn generate_learning_plan(
        &self,
        text: &str,
        dialect: dialect_coach_shared::Dialect,
    ) -> Result<dialect_coach_shared::models::plan::import::SimpleImportLanguagePlan> {
        let generator =
            planning::PlanGenerator::new(self.planning_agent.clone(), self.planning_config.clone());
        generator.generate_plan(text, dialect).await
    }
}

#[cfg(test)]
mod tests {
    use super::provider::{ANTHROPIC_PROVIDER, OLLAMA_PROVIDER, OPENAI_PROVIDER};
    use super::*;
    use crate::embedding_service::EmbeddingService;
    use crate::qdrant_service::QdrantService;

    fn hosted_config() -> LlmConfig {
        dialect_coach_shared::config::load_config(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../config.yaml"
        ))
        .unwrap()
        .llm
    }

    #[tokio::test]
    #[ignore] // Requires API key and Qdrant
    async fn test_agent_initialization() {
        dotenvy::dotenv().ok();
        let config = hosted_config();
        let qdrant_url = std::env::var("QDRANT_URL").unwrap();
        let qdrant_key = std::env::var("QDRANT_API_KEY").unwrap();
        let qdrant = QdrantService::new(&qdrant_url, &qdrant_key).await.unwrap();
        let embeddings = EmbeddingService::new().unwrap();
        let agent =
            AgentService::from_config(&config, Arc::new(qdrant), Arc::new(embeddings)).unwrap();
        assert_eq!(agent.response_agent.provider(), OPENAI_PROVIDER);
        assert!(!agent.response_agent.model().is_empty());
    }

    #[test]
    fn test_load_channel_agent_anthropic() {
        let (agent, config) = load_channel_agent(&hosted_config(), "learning", |name| {
            (name == "ANTHROPIC_API_KEY").then(|| "test-key".into())
        })
        .unwrap();
        assert_eq!(agent.provider(), ANTHROPIC_PROVIDER);
        assert_eq!(agent.model(), "claude-sonnet-4-6");
        assert_eq!(
            config.base_url.as_deref(),
            Some("https://api.anthropic.com")
        );
    }

    #[test]
    fn test_load_channel_agent_openai() {
        let llm = hosted_config();
        let read_key = |name: &str| (name == "OPENAI_API_KEY").then(|| "sk-test-key".into());
        let (agent, config) = load_channel_agent(&llm, "response", read_key).unwrap();
        assert_eq!(agent.provider(), OPENAI_PROVIDER);
        assert_eq!(agent.model(), "gpt-5.4");
        assert_eq!(config.reasoning_budget, 512);
        assert_eq!(config.reasoning_effort, "low");

        let (_, config) = load_channel_agent(&llm, "pronunciation", read_key).unwrap();
        assert_eq!(config.reasoning_effort, "none");
    }

    #[test]
    fn test_load_channel_agent_ollama_without_cloud_credentials() {
        let config = serde_yaml::from_str(
            "
default_provider: local
providers:
  local: { driver: ollama, model: custom-local:42 }
",
        )
        .unwrap();
        let (agent, _) = load_channel_agent(&config, "response", |name| {
            panic!("Unexpected credential lookup for {name}")
        })
        .unwrap();
        assert_eq!(agent.provider(), OLLAMA_PROVIDER);
        assert_eq!(agent.model(), "custom-local:42");
    }
}
