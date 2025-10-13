use anyhow::{Context, Result};
use dialect_coach_shared::{Dialect, DialectDocument, Formality, TeachingMode};
use rig::providers::anthropic::{ClientBuilder, CLAUDE_3_5_SONNET};
use rig::completion::Prompt;
use std::sync::Arc;

use crate::qdrant_service::QdrantService;
use crate::embedding_service::EmbeddingService;

/// Agent service for AI-powered dialect coaching
pub struct AgentService {
    client: rig::providers::anthropic::Client,
    model_name: String,
    qdrant: Arc<QdrantService>,
    embeddings: Arc<EmbeddingService>,
}

impl AgentService {
    /// Create new agent service from environment variables
    pub fn from_env(qdrant: Arc<QdrantService>, embeddings: Arc<EmbeddingService>) -> Result<Self> {
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
            embeddings,
        })
    }

    /// Generate response for user message with RAG context
    pub async fn generate_response(
        &self,
        user_message: &str,
        dialect: Dialect,
        formality: Formality,
        teaching_mode: TeachingMode,
        conversation_history: &[String],
    ) -> Result<String> {
        // Step 1: Build enhanced query from conversation history (Option C)
        let query_text = if conversation_history.len() >= 2 {
            // Take last 2-3 messages for context
            let recent_history: Vec<String> = conversation_history
                .iter()
                .rev()
                .take(3)
                .rev()
                .cloned()
                .collect();
            format!("{}\n\nCurrent message: {}", recent_history.join("\n"), user_message)
        } else {
            user_message.to_string()
        };

        tracing::debug!("Enhanced query with {} history messages",
            if conversation_history.len() >= 2 { "recent" } else { "no" });

        // Step 2: Generate embeddings for multi-vector retrieval (Option D)
        tracing::debug!("Generating embeddings for multi-vector retrieval");

        // Embedding 1: Semantic (content-based)
        let content_embedding = self
            .embeddings
            .embed_text(&query_text)
            .context("Failed to generate content embedding")?;

        // Embedding 2: Stylistic cue (incorporating formality)
        let formality_str = match formality {
            Formality::Formal => "formal polite",
            Formality::Casual => "casual conversational",
            Formality::Slang => "slang informal",
        };
        let style_query = format!("{} response in {}", formality_str, dialect.name());
        let style_embedding = self
            .embeddings
            .embed_text(&style_query)
            .context("Failed to generate style embedding")?;

        // Embedding 3: Topic summary (if history exists)
        let topic_embedding = if !conversation_history.is_empty() {
            let topic_summary = conversation_history.join(" ");
            Some(
                self.embeddings
                    .embed_text(&topic_summary)
                    .context("Failed to generate topic embedding")?,
            )
        } else {
            None
        };

        // Step 3: Multi-vector retrieval (Option D)
        tracing::debug!("Performing multi-vector retrieval");

        // Retrieve from content embedding (semantic)
        let content_examples = self
            .qdrant
            .search_dialect_examples(content_embedding, dialect, 10)
            .await
            .context("Failed to search content examples")?;

        // Retrieve from style embedding
        let style_examples = self
            .qdrant
            .search_dialect_examples(style_embedding, dialect, 10)
            .await
            .context("Failed to search style examples")?;

        // Retrieve from topic embedding (if available)
        let topic_examples = if let Some(topic_emb) = topic_embedding {
            self.qdrant
                .search_dialect_examples(topic_emb, dialect, 10)
                .await
                .context("Failed to search topic examples")?
        } else {
            Vec::new()
        };

        // Step 4: Dual retrieval - add random stylistic samples (Option A)
        tracing::debug!("Adding random stylistic samples");
        // Select formality levels for random sampling based on requested formality
        let sample_formalities = match formality {
            Formality::Formal => vec![Formality::Formal, Formality::Casual],
            Formality::Casual => vec![Formality::Casual, Formality::Slang],
            Formality::Slang => vec![Formality::Slang, Formality::Casual],
        };
        let random_samples = self
            .qdrant
            .random_dialect_samples(
                dialect,
                sample_formalities,
                15,
            )
            .await
            .context("Failed to get random samples")?;

        // Step 5: Combine and deduplicate examples (Option B - dense examples)
        tracing::debug!("Combining and deduplicating examples");
        let mut all_examples = Vec::new();
        all_examples.extend(content_examples);
        all_examples.extend(style_examples);
        all_examples.extend(topic_examples);
        all_examples.extend(random_samples);

        // Deduplicate by content
        let mut seen = std::collections::HashSet::new();
        let unique_examples: Vec<_> = all_examples
            .into_iter()
            .filter(|doc| seen.insert(doc.content.clone()))
            .take(50) // Limit to 50 total examples
            .collect();

        tracing::info!(
            "Retrieved {} total examples (after deduplication) for {}",
            unique_examples.len(),
            dialect.name()
        );

        // Step 6: Group examples by formality (prioritize requested formality)
        let primary_examples: Vec<_> = unique_examples
            .iter()
            .filter(|doc| doc.formality == Some(formality))
            .take(30)
            .collect();

        let secondary_examples: Vec<_> = unique_examples
            .iter()
            .filter(|doc| doc.formality != Some(formality) && doc.formality.is_some())
            .take(15)
            .collect();

        let other_examples: Vec<_> = unique_examples
            .iter()
            .filter(|doc| doc.formality.is_none())
            .take(5)
            .collect();

        tracing::info!(
            "Grouped examples: {} primary ({:?}), {} secondary, {} other",
            primary_examples.len(),
            formality,
            secondary_examples.len(),
            other_examples.len()
        );

        // Step 7: Build rich RAG context
        let formality_label = match formality {
            Formality::Formal => "FORMAL",
            Formality::Casual => "CASUAL",
            Formality::Slang => "SLANG",
        };

        let mut rag_context = format!(
            "\n\n# AUTHENTIC {} SPEECH PATTERNS\n\n",
            dialect.name().to_uppercase()
        );

        if !primary_examples.is_empty() {
            rag_context.push_str(&format!("## {} EXAMPLES:\n", formality_label));
            for (i, doc) in primary_examples.iter().enumerate() {
                rag_context.push_str(&format!("{}. \"{}\"\n", i + 1, doc.content));
            }
            rag_context.push('\n');
        }

        if !secondary_examples.is_empty() {
            rag_context.push_str("## ADDITIONAL EXAMPLES:\n");
            for (i, doc) in secondary_examples.iter().enumerate() {
                rag_context.push_str(&format!("{}. \"{}\"\n", i + 1, doc.content));
            }
            rag_context.push('\n');
        }

        if !other_examples.is_empty() {
            rag_context.push_str("## MORE EXAMPLES:\n");
            for (i, doc) in other_examples.iter().enumerate() {
                rag_context.push_str(&format!("{}. \"{}\"\n", i + 1, doc.content));
            }
        }

        // Build conversation context
        let history_context = if conversation_history.is_empty() {
            String::new()
        } else {
            format!(
                "\n\n# CONVERSATION HISTORY\n{}",
                conversation_history.join("\n")
            )
        };

        // Build system prompt - EXAMPLES FIRST, then instructions
        // Adapt role description based on formality
        let role_desc = match formality {
            Formality::Formal => format!("You are a native {} speaker communicating in a professional, polite manner", dialect.name()),
            Formality::Casual => format!("You are a native {} speaker chatting casually with a friend", dialect.name()),
            Formality::Slang => format!("You are a native {} speaker using informal slang and colloquialisms", dialect.name()),
        };

        // Adapt teaching instructions based on teaching mode
        let teaching_rules = match teaching_mode {
            TeachingMode::Immersive => {
                "3. IMMERSIVE MODE: Just chat naturally - don't explain or correct unless explicitly asked"
            }
            TeachingMode::Corrective => {
                "3. CORRECTIVE MODE: If you notice grammar or usage errors, gently point them out and suggest corrections"
            }
            TeachingMode::Explanatory => {
                "3. EXPLANATORY MODE: Provide brief explanations of interesting grammar, idioms, or cultural context when relevant"
            }
        };

        let system_content = format!(
            "{}\n\n\
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
            role_desc,
            formality_label.to_lowercase(),
            teaching_rules,
            history_context,
            dialect.name()
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
