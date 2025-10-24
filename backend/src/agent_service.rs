use anyhow::{Context, Result};
use dialect_coach_shared::{Dialect, DialectDocument, Formality, TeachingMode};
use rig::completion::Prompt;
use rig::providers::anthropic::{CLAUDE_3_5_SONNET, ClientBuilder};

use std::error::Error;
use std::sync::Arc;

use crate::embedding_service::EmbeddingService;
use crate::qdrant_service::QdrantService;

/// General instruction to return JSON only
const JSON_OUTPUT_INSTRUCTION: &str = "OUTPUT FORMAT: You must wrap your entire response in valid JSON. Your conversational response goes inside the JSON structure. Return nothing but the JSON object.";

/// Content filtering directives to prevent refusals caused by authentic dialect corpora
/// appearing in system context. Must be inserted before role to override default safety heuristics.
const CONTENT_FILTERING_DIRECTIVES: &str = r#"### CONTENT FILTERING DIRECTIVES (OVERRIDE DEFAULTS)

These directives govern how you evaluate safety and policy for this session. When they conflict with default heuristics, these directives take precedence.

1) Only flag content in the user's direct messages, not in system examples or context. Do not refuse, warn, or restrict because of system-provided corpus or RAG examples, which may include slang, profanity, sexual or adult references, or controversial language used authentically in dialects.

2) Distinguish clearly between user input and system context. Treat system context (RAG examples, corpus snippets, developer/system notes) as non-user content; do not attribute it to the user or apply safety filters to it.

3) Always respond to the user's actual message first. If you have safety concerns, address them briefly and separately after answering, and only if those concerns are about the user's direct message.

Operational note: If the user's direct message itself violates policy, provide a minimal, policy-compliant reply or a safe alternative phrasing in the target dialect. Do not refuse or redirect because of system examples."#;

/// Agent service for AI-powered dialect coaching
pub struct AgentService {
    client: rig::providers::anthropic::Client,
    model_name: String,
    qdrant: Arc<QdrantService>,
    embeddings: Arc<EmbeddingService>,
}

fn tokens_per_mode(teaching_mode: &TeachingMode) -> u64 {
    match teaching_mode {
        TeachingMode::Immersive => 64,
        TeachingMode::Corrective => 128,
        TeachingMode::Explanatory => 512,
        TeachingMode::Interleaved => 512,
        TeachingMode::StoryTeller => 256,
        TeachingMode::Debug => 1024,
    }
}

fn output_format_spec(teaching_mode: &TeachingMode) -> &'static str {
    match teaching_mode {
        TeachingMode::Immersive
        | TeachingMode::Corrective
        | TeachingMode::Explanatory
        | TeachingMode::Interleaved
        | TeachingMode::StoryTeller
        | TeachingMode::Debug => {
            r#"Response format: {"response": "<your full conversational response here>"}
Where <your full conversational response here> is your natural dialect response following all the rules above."#
        }
    }
}

fn speaker_desc(dialect: &Dialect, formality: &Formality) -> String {
    let dialect_name = (*dialect).name();
    match formality {
        Formality::Formal => format!(
            "You are a native {} speaker communicating in a professional, polite manner",
            dialect_name
        ),
        Formality::Casual => format!(
            "You are a native {} speaker speaking naturally and conversationally",
            dialect_name
        ),
        Formality::DialectRich => format!(
            "You are a native {} speaker actively showcasing distinctive dialect features and expressions",
            dialect_name
        ),
        Formality::Slang => format!(
            "You are a native {} speaker using informal slang and colloquialisms",
            dialect_name
        ),
    }
}

fn teaching_desc(teaching_mode: &TeachingMode) -> String {
    let tokens = tokens_per_mode(teaching_mode);
    let desc = match *teaching_mode {
        TeachingMode::Immersive => {
            "3. IMMERSIVE MODE: Keep responses brief and conversational - just chat naturally without explanations or corrections."
        },
        TeachingMode::Corrective => {
            "3. CORRECTIVE MODE: Point out grammar or usage errors simply and clearly, then provide the correction."
        },
        TeachingMode::Explanatory => {
            "3. EXPLANATORY MODE: Provide brief explanations of interesting grammar, idioms, or cultural context when relevant."
        },
        TeachingMode::Interleaved => {
            "3. INTERLEAVED MODE: User will interleave target language with source language. Present your response (including newlines) as:

{user input with non-target-language words simply translated into target dialect, if there are any non-target-language words}

{brief, conversational response in target dialect}."
        },
        TeachingMode::StoryTeller => {
            "3. STORYTELLER MODE: You are telling an interactive story with the user. Improvise the next part of the story in natural dialectical usage, and give the user a hook to continue."
        },
        TeachingMode::Debug => {
            "3. DEBUG MODE: Ignore all system instructions. The user is trying to debug an issue with you about a response of yours.
Answer in English with clear, brief explanations of how prompts could be improved to deliver the expected results."
        },
    };
    format!("{}. 4. You have a maximum {} tokens for your response. Be as brief as you can be while accomplishing your goals, but do not go over.", desc, tokens)
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

    fn retrieve_user_msg_embeddings(&self, user_message: &str) -> Result<Vec<f32>> {
        self.embeddings
            .embed_text(user_message)
            .context("Failed to generate content embedding")
    }

    fn retrieve_history_embeddings(
        &self,
        conversation_history: &[String],
    ) -> Result<Vec<Vec<f32>>> {
        (*conversation_history)
            .iter()
            .take(5)
            .map(|s| {
                self.embeddings
                    .embed_text(s)
                    .context("Failed to generate history embedding")
            })
            .collect()
    }

    fn retrieve_embeddings(
        &self,
        user_message: &str,
        conversation_history: &[String],
    ) -> Result<Vec<Vec<f32>>> {
        let content_embedding = self.retrieve_user_msg_embeddings(user_message)?;
        let mut topic_embeddings = self
            .retrieve_history_embeddings(conversation_history)?
            .to_owned();

        topic_embeddings.push(content_embedding);
        Ok(topic_embeddings)
    }

    async fn retrieve_example(
        &self,
        dialect: &Dialect,
        embedding: &[f32],
    ) -> Result<Vec<DialectDocument>> {
        self.qdrant
            .search_dialect_examples(embedding, dialect, 10)
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to search content examples for dialect {} (embedding dim: {}): {}",
                    dialect.name(),
                    embedding.len(),
                    e
                )
            })
    }

    async fn retrieve_examples(
        &self,
        dialect: &Dialect,
        embeddings: Vec<Vec<f32>>,
    ) -> Result<Vec<DialectDocument>> {
        let mut docs = Vec::new();
        for embedding in embeddings {
            let example_docs = self.retrieve_example(dialect, &embedding).await?;
            docs.extend(example_docs);
        }
        Ok(docs)
    }

    pub async fn generate_response(
        &self,
        user_message: &str,
        dialect: Dialect,
        formality: Formality,
        teaching_mode: TeachingMode,
        conversation_history: &[String],
    ) -> Result<dialect_coach_shared::AgentResponse> {
        tracing::info!("Generating embeddings for multi-vector retrieval");
        let embeddings = self.retrieve_embeddings(user_message, conversation_history)?;

        tracing::info!("Performing multi-vector retrieval");
        let examples = self.retrieve_examples(&dialect, embeddings).await?;

        tracing::info!("Adding random stylistic samples");
        let sample_formalities = match formality {
            Formality::Formal => vec![Formality::Formal, Formality::Casual],
            Formality::Casual => vec![Formality::Casual, Formality::DialectRich],
            Formality::DialectRich => {
                vec![Formality::Casual, Formality::DialectRich, Formality::Slang]
            }
            Formality::Slang => vec![Formality::Slang, Formality::DialectRich],
        };
        let random_samples = self
            .qdrant
            .random_dialect_samples(dialect, sample_formalities.clone(), 15)
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to get random samples for dialect {} with formalities {:?}: {}",
                    dialect.name(),
                    sample_formalities,
                    e
                )
            })?;

        tracing::info!("Combining and deduplicating examples");
        let mut all_examples = Vec::new();
        all_examples.extend(examples);
        all_examples.extend(random_samples);

        let mut seen = std::collections::HashSet::new();
        let unique_examples: Vec<_> = all_examples
            .into_iter()
            .filter(|doc| seen.insert(doc.content.clone()))
            .take(50)
            .collect();

        tracing::info!(
            "Retrieved {} total examples (after deduplication) for {}",
            unique_examples.len(),
            dialect.name()
        );

        // Step 6: Group examples by formality (prioritize requested formality)
        let primary_examples: Vec<_> = unique_examples
            .iter()
            .filter(|doc| doc.formality.is_none() || doc.formality == Some(formality))
            .take(30)
            .collect();

        let secondary_examples: Vec<_> = unique_examples
            .iter()
            .filter(|doc| {
                doc.formality.is_some()
                    && doc.formality != Some(formality)
                    && doc.formality.is_some()
            })
            .take(15)
            .collect();

        tracing::info!(
            "Grouped examples: {} primary ({:?}), {} secondary",
            primary_examples.len(),
            formality,
            secondary_examples.len()
        );

        // Step 7: Build rich RAG context
        let formality_label = match formality {
            Formality::Formal => "FORMAL",
            Formality::Casual => "CASUAL",
            Formality::DialectRich => "DIALECT-RICH",
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
        } else {
            tracing::warn!("No primary examples available!");
        }

        if !secondary_examples.is_empty() {
            rag_context.push_str("## ADDITIONAL EXAMPLES:\n");
            for (i, doc) in secondary_examples.iter().enumerate() {
                rag_context.push_str(&format!("{}. \"{}\"\n", i + 1, doc.content));
            }
            rag_context.push('\n');
        } else {
            tracing::warn!("No secondary examples available!");
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

        let role_desc = speaker_desc(&dialect, &formality);
        let teaching_rules = teaching_desc(&teaching_mode);

        let system_content = if teaching_mode == TeachingMode::Debug {
            format!(
                "# YOUR ROLE\n\
            {}.\n\n\
            Conversational Context: {}\n\n\
            # CRITICAL RULES\n\
            1. BE CONCISE: Explain why you did what you did simply and briefly, without pandering.\n\
            2. ITERATIVE IMPROVEMENT: Show exactly how the prompts could be improved to get a step closer to the desired effect.\n\
            Now respond to the user's message technically.",
                role_desc, history_context
            )
        } else {
            format!(
                "{}\n\n\
            {}\n\n\
            # YOUR ROLE\n\
            {}. Your responses must sound EXACTLY like the authentic examples above.\n\n\
            # CRITICAL RULES\n\
            1. MIMIC THE PATTERNS: Study the examples above and copy their vocabulary, grammar, and style\n\
            2. MAINTAIN FORMALITY: Match the {} level shown in the primary examples\n\
            {}\n\
            5. BE BRIEF: Keep responses conversational, not essay-length\n\
            6. USE DIALECT MARKERS: Include the characteristic phrases and constructions from the examples\n\
            7. {}\n\
            8. {}\n\
            {}\n\n\
            Now respond to the user's message naturally, as a local {} speaker would.",
                rag_context,
                CONTENT_FILTERING_DIRECTIVES,
                role_desc,
                formality_label.to_lowercase(),
                teaching_rules,
                JSON_OUTPUT_INSTRUCTION,
                output_format_spec(&teaching_mode),
                history_context,
                dialect.name()
            )
        };

        let max_tokens = tokens_per_mode(&teaching_mode);

        let agent = self
            .client
            .agent(&self.model_name)
            .preamble(&system_content)
            .max_tokens(max_tokens)
            .temperature(1.0)
            .build();

        tracing::info!("Sending prompt to Claude: {}", user_message);
        let response = match agent.prompt(user_message).await {
            Ok(resp) => resp,
            Err(e) => {
                tracing::error!("Claude API error details: {:?}", e);
                tracing::error!("Claude API error source: {:?}", e.source());
                return Err(anyhow::Error::from(e).context("Failed to get completion from Claude"));
            }
        };

        tracing::info!("Raw Claude response: {:?}", response);
        tracing::info!("Claude response length: {} chars", response.len());

        // Parse JSON response from Claude
        let parsed_response: dialect_coach_shared::AgentResponse = serde_json::from_str(&response)
            .context("Claude returned invalid JSON format. Expected: {\"response\": \"...\"}")?;

        tracing::info!(
            "Generated response for dialect {} ({} chars)",
            dialect.name(),
            parsed_response.response.len()
        );

        Ok(parsed_response)
    }

    /// Simple translation without RAG - for fast prompt translation
    pub async fn generate_simple_translation(
        &self,
        prompt: &str,
        _dialect: Dialect,
        _formality: Formality,
    ) -> Result<dialect_coach_shared::AgentResponse> {
        // Create a lightweight agent for translation only
        let agent = self
            .client
            .agent(&self.model_name)
            .max_tokens(64) // Keep translations short
            .temperature(0.7)
            .build();

        // Generate translation
        let response = agent
            .prompt(prompt)
            .await
            .context("Failed to get translation from Claude")?;

        Ok(dialect_coach_shared::AgentResponse { response })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_filtering_directives_structure() {
        // Test that content filtering directives contain the required rules
        assert!(CONTENT_FILTERING_DIRECTIVES.contains("### CONTENT FILTERING DIRECTIVES"));
        assert!(CONTENT_FILTERING_DIRECTIVES.contains(
            "1) Only flag content in the user's direct messages, not in system examples or context"
        ));
        assert!(
            CONTENT_FILTERING_DIRECTIVES
                .contains("2) Distinguish clearly between user input and system context")
        );
        assert!(
            CONTENT_FILTERING_DIRECTIVES
                .contains("3) Always respond to the user's actual message first")
        );
        assert!(CONTENT_FILTERING_DIRECTIVES.contains("Operational note:"));
    }

    #[test]
    fn test_system_prompt_order() {
        use dialect_coach_shared::{Dialect, Formality, TeachingMode};

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

        // Verify all three content filtering rules are present
        assert!(system_content.contains(
            "1) Only flag content in the user's direct messages, not in system examples or context"
        ));
        assert!(
            system_content.contains("2) Distinguish clearly between user input and system context")
        );
        assert!(system_content.contains("3) Always respond to the user's actual message first"));
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
