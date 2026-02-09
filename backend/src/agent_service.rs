use anyhow::{Context, Result, anyhow};
use dialect_coach_shared::AgentUsage;
use std::env;
use std::sync::Arc;

use crate::embedding_service::EmbeddingService;
use crate::qdrant_service::QdrantService;

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

use provider::{
    ANTHROPIC_PROVIDER, CompletionAgent, CompletionAgentFactory, OPENAI_PROVIDER,
    ProviderAgentConfig,
};
use response::ResponseContext;
use util::contains_illegal_characters;

fn load_channel_agent(
    channel: &dialect_coach_shared::config::ChannelConfig,
    channel_name: &str,
    llm_config: &dialect_coach_shared::config::LlmConfig,
) -> Result<(Arc<dyn CompletionAgent>, ProviderAgentConfig)> {
    let reasoning_budget = channel
        .reasoning_budget
        .unwrap_or(llm_config.openai.reasoning_budget);
    let channel_api_key = env::var(format!("{}_API_KEY", channel_name)).ok();
    match channel.provider.as_str() {
        ANTHROPIC_PROVIDER => {
            let api_key = channel_api_key
                .or_else(|| env::var("ANTHROPIC_API_KEY").ok())
                .ok_or_else(|| anyhow!("Missing API key for {} channel", channel_name))?;
            let model = channel
                .model
                .clone()
                .unwrap_or_else(|| llm_config.anthropic.model.clone());
            let config = ProviderAgentConfig::anthropic(api_key, model, reasoning_budget);
            Ok((
                Arc::from(CompletionAgentFactory::build(config.clone())?),
                config,
            ))
        }
        OPENAI_PROVIDER => {
            let api_key = channel_api_key
                .or_else(|| env::var("OPENAI_API_KEY").ok())
                .ok_or_else(|| anyhow!("Missing API key for {} channel", channel_name))?;
            let model = channel
                .model
                .clone()
                .unwrap_or_else(|| llm_config.openai.model.clone());
            let config = ProviderAgentConfig::openai(api_key, model, reasoning_budget);
            Ok((
                Arc::from(CompletionAgentFactory::build(config.clone())?),
                config,
            ))
        }
        other => Err(anyhow!(
            "Unsupported provider '{}' for {} channel",
            other,
            channel_name
        )),
    }
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
        llm_config: &dialect_coach_shared::config::LlmConfig,
        qdrant: Arc<QdrantService>,
        embeddings: Arc<EmbeddingService>,
    ) -> Result<Self> {
        let response_agent =
            load_channel_agent(&llm_config.channels.response, "RESPONSE", llm_config)?.0;
        let learning_agent =
            load_channel_agent(&llm_config.channels.learning, "LEARNING", llm_config)?.0;
        let analysis_agent =
            load_channel_agent(&llm_config.channels.analysis, "ANALYSIS", llm_config)?.0;
        let (planning_agent, planning_config) =
            load_channel_agent(&llm_config.channels.planning, "PLANNING", llm_config)?;
        let pronunciation_agent = load_channel_agent(
            &llm_config.channels.pronunciation,
            "PRONUNCIATION",
            llm_config,
        )?
        .0;

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
    use super::*;
    use crate::embedding_service::EmbeddingService;
    use crate::qdrant_service::QdrantService;
    use dialect_coach_shared::config::{
        AnthropicProviderConfig, ChannelConfig, ChannelsConfig, LlmConfig, OpenAiProviderConfig,
    };
    use serial_test::serial;

    fn make_llm_config() -> LlmConfig {
        LlmConfig {
            anthropic: AnthropicProviderConfig {
                model: "claude-haiku-4-5".to_string(),
                org_id: None,
            },
            openai: OpenAiProviderConfig {
                model: "gpt-4o".to_string(),
                reasoning_budget: 512,
            },
            channels: make_channels_config("anthropic"),
        }
    }

    fn make_channels_config(provider: &str) -> ChannelsConfig {
        let ch = || ChannelConfig {
            provider: provider.to_string(),
            model: None,
            reasoning_budget: None,
        };
        ChannelsConfig {
            response: ch(),
            learning: ch(),
            analysis: ch(),
            pronunciation: ch(),
            planning: ch(),
        }
    }

    #[tokio::test]
    #[ignore] // Requires API key and Qdrant
    async fn test_agent_initialization() {
        dotenvy::dotenv().ok();
        let config = dialect_coach_shared::config::load_config("config.yaml").unwrap();
        let qdrant_url = std::env::var("QDRANT_URL").unwrap();
        let qdrant_key = std::env::var("QDRANT_API_KEY").unwrap();
        let qdrant = QdrantService::new(&qdrant_url, &qdrant_key).await.unwrap();
        let embeddings = EmbeddingService::new().unwrap();
        let agent =
            AgentService::from_config(&config.llm, Arc::new(qdrant), Arc::new(embeddings)).unwrap();
        assert_eq!(agent.response_agent.provider(), ANTHROPIC_PROVIDER);
        assert!(!agent.response_agent.model().is_empty());
    }

    #[test]
    #[serial]
    fn test_load_channel_agent_anthropic() {
        unsafe {
            std::env::set_var("ANTHROPIC_API_KEY", "test-key");
            std::env::remove_var("RESPONSE_API_KEY");
        }
        let llm = make_llm_config();
        let result = load_channel_agent(&llm.channels.response, "RESPONSE", &llm);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().0.provider(), ANTHROPIC_PROVIDER);
        unsafe {
            std::env::remove_var("ANTHROPIC_API_KEY");
        }
    }

    #[test]
    #[serial]
    fn test_load_channel_agent_openai() {
        unsafe {
            std::env::set_var("OPENAI_API_KEY", "sk-test-key");
            std::env::remove_var("RESPONSE_API_KEY");
            std::env::remove_var("ANTHROPIC_API_KEY");
        }
        let mut llm = make_llm_config();
        llm.channels.response.provider = "openai".to_string();
        let result = load_channel_agent(&llm.channels.response, "RESPONSE", &llm);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().0.provider(), OPENAI_PROVIDER);
        unsafe {
            std::env::remove_var("OPENAI_API_KEY");
        }
    }

    #[test]
    #[serial]
    fn test_load_channel_agent_openai_custom_model() {
        unsafe {
            std::env::set_var("OPENAI_API_KEY", "sk-test-key");
            std::env::remove_var("RESPONSE_API_KEY");
            std::env::remove_var("ANTHROPIC_API_KEY");
        }
        let mut llm = make_llm_config();
        llm.channels.response.provider = "openai".to_string();
        llm.channels.response.model = Some("gpt-4-turbo".to_string());
        let result = load_channel_agent(&llm.channels.response, "RESPONSE", &llm);
        assert!(result.is_ok());
        let agent = result.unwrap().0;
        assert_eq!(agent.provider(), OPENAI_PROVIDER);
        assert_eq!(agent.model(), "gpt-4-turbo");
        unsafe {
            std::env::remove_var("OPENAI_API_KEY");
        }
    }

    #[test]
    fn test_channel_reasoning_budget_override() {
        let mut llm = make_llm_config();
        llm.channels.response.reasoning_budget = Some(300);
        assert_eq!(llm.channels.response.reasoning_budget, Some(300));
    }

    #[test]
    fn test_channel_reasoning_budget_default() {
        let llm = make_llm_config();
        let budget = llm
            .channels
            .response
            .reasoning_budget
            .unwrap_or(llm.openai.reasoning_budget);
        assert_eq!(budget, 512);
    }
}
