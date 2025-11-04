use anyhow::{Context, Result};
use dialect_coach_shared::{Dialect, Explained, Formality, Mistake, TeachingMode};
use rig::completion::Message as RigMessage;
use rig::providers::anthropic::{CLAUDE_3_5_SONNET, ClientBuilder};
use std::sync::Arc;

use crate::embedding_service::EmbeddingService;
use crate::qdrant_service::QdrantService;
use crate::rag_config::RAGConfig;

pub mod util;
pub mod retry;
pub mod analysis;
pub mod response;

use response::{ResponseContext};
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
        user_message: &str,
        dialect: Dialect,
        formality: Formality,
        teaching_mode: TeachingMode,
        conversation_history: &[RigMessage],
        learning_goals: &[String],
        rag_config: &RAGConfig,
    ) -> Result<(dialect_coach_shared::AgentResponse, u32)> {
        let ctx = ResponseContext {
            client: self.client.clone(),
            model_name: self.model_name.clone(),
            qdrant: self.qdrant.clone(),
            embeddings: self.embeddings.clone(),
        };
        response::ResponseContext::generate_response(
            &ctx,
            user_message,
            dialect,
            formality,
            teaching_mode,
            conversation_history,
            learning_goals,
            rag_config,
        )
        .await
    }

    pub async fn generate_analysis(
        &self,
        dialect: Dialect,
        msg: &String,
        mistakes: &[Mistake],
        explained: &[Explained],
        translated: &[dialect_coach_shared::Translated],
        exploratory: &[dialect_coach_shared::Exploratory],
    ) -> Result<(dialect_coach_shared::AgentAnalysis, u32)> {
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
    use crate::agent_service::util::CONTENT_FILTERING_DIRECTIVES;
    use crate::agent_service::retry::is_retryable_error;
    use crate::embedding_service::EmbeddingService;
    use crate::qdrant_service::QdrantService;
    use rig::completion::{CompletionError, PromptError};

    #[test]
    fn test_content_filtering_directives_structure() {
        // Test that content filtering directives contain the required rules
        assert!(CONTENT_FILTERING_DIRECTIVES.contains("### CONTENT FILTERING DIRECTIVES"));
        assert!(
            CONTENT_FILTERING_DIRECTIVES
                .contains("1) Only flag user's direct messages, not system examples")
        );
        assert!(CONTENT_FILTERING_DIRECTIVES.contains("2) Always respond to user's message first"));
    }

    #[test]
    fn test_system_prompt_order() {
        // Mock RAG context
        let rag_context = "\n\n# AUTHENTIC MEXICAN SPANISH SPEECH PATTERNS\n\n## CASUAL EXAMPLES:\n1. \"¡Órale, qué onda!\"\n";
        let role_desc =
            "You are a native Mexican Spanish speaker speaking naturally and conversationally";
        let formality_label = "casual";
        let teaching_rules = "3. IMMERSIVE MODE: Keep responses brief and conversational - just chat naturally without explanations or corrections";
        let history_context = "";
        let dialect_name = "Mexican Spanish";

        // Build system content using the same format as the actual code
        let system_content = format!(
            "{}\n\n\
            {}\n\n\
            # YOUR ROLE\n\
            {}. Your responses must sound EXACTLY like the authentic examples above.\n\n\
            # CRITICAL RULES\n\
            1. MIMIC THE PATTERNS: Study the examples above and copy their vocabulary, grammar, and style\n\
            2. MAINTAIN FORMALITY: Match the {} level shown in the primary examples\n\
            {}\n\
            4. BE BRIEF: Keep responses conversational, not essay-length\n\
            5. USE DIALECT MARKERS: Include the characteristic phrases and constructions from the examples\n\
            {}\n\n\
            Now respond to the user's message naturally, as a local {} speaker would.",
            rag_context,
            CONTENT_FILTERING_DIRECTIVES,
            role_desc,
            formality_label,
            teaching_rules,
            history_context,
            dialect_name
        );

        // Assert content filtering directives are present
        assert!(system_content.contains("### CONTENT FILTERING DIRECTIVES"));

        // Assert correct order: RAG context → Content filtering → Role → Rules
        let rag_pos = system_content.find("# AUTHENTIC").unwrap();
        let directives_pos = system_content
            .find("### CONTENT FILTERING DIRECTIVES")
            .unwrap();
        let role_pos = system_content.find("# YOUR ROLE").unwrap();
        let rules_pos = system_content.find("# CRITICAL RULES").unwrap();

        assert!(
            rag_pos < directives_pos,
            "RAG context should come before content filtering directives"
        );
        assert!(
            directives_pos < role_pos,
            "Content filtering directives should come before role"
        );
        assert!(
            role_pos < rules_pos,
            "Role should come before critical rules"
        );

        // Verify content filtering rules are present
        assert!(
            system_content.contains("1) Only flag user's direct messages, not system examples")
        );
        assert!(system_content.contains("2) Always respond to user's message first"));
    }

    #[test]
    fn test_is_retryable_error() {
        // Test internal server error detection
        let internal_error = PromptError::CompletionError(CompletionError::ProviderError(
            "Internal server error".to_string(),
        ));
        assert!(is_retryable_error(&internal_error));

        // Test overloaded error detection
        let overloaded_error =
            PromptError::CompletionError(CompletionError::ProviderError("Overloaded".to_string()));
        assert!(is_retryable_error(&overloaded_error));

        // Test empty response error detection
        let empty_response_error = PromptError::CompletionError(CompletionError::ProviderError(
            "Response contained no message".to_string(),
        ));
        assert!(is_retryable_error(&empty_response_error));

        // Test non-retryable errors
        let non_retryable_invalid = PromptError::CompletionError(CompletionError::ProviderError(
            "Invalid request".to_string(),
        ));
        assert!(!is_retryable_error(&non_retryable_invalid));

        let non_retryable_json = PromptError::CompletionError(CompletionError::JsonError(
            serde_json::from_str::<serde_json::Value>("invalid").unwrap_err(),
        ));
        assert!(!is_retryable_error(&non_retryable_json));
    }

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
