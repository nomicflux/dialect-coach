use anyhow::{Context, Result};
use dialect_coach_shared::{Dialect, DialectDocument, Formality, TeachingMode};
use rig::completion::{
    Chat, Message as RigMessage, Prompt, message::Text, message::UserContent,
};
use rig::one_or_many::OneOrMany;
use std::sync::Arc;

use crate::embedding_service::EmbeddingService;
use crate::qdrant_service::QdrantService;
use crate::rag_config::RAGConfig;

use super::retry::{RetryContext, build_retry_response_preamble, retry_chat_call};
use super::util::{
    contains_illegal_characters, create_prefilled_assistant_message, get_message_text,
    learning_goals_section, normalize_json_response, output_format_spec, speaker_desc,
    teaching_desc, temperature_for_mode, tokens_per_mode, CONTENT_FILTERING_DIRECTIVES,
    JSON_OUTPUT_INSTRUCTION,
};

pub fn format_examples_as_user_message(
    primary_examples: &[&DialectDocument],
    secondary_examples: &[&DialectDocument],
) -> String {
    if primary_examples.is_empty() && secondary_examples.is_empty() {
        return String::new();
    }

    let mut examples_text = "DIALECT EXAMPLES:\n".to_string();

    for doc in primary_examples {
        examples_text.push_str(&format!("\"{}\"\n", doc.content));
    }

    for doc in secondary_examples {
        examples_text.push_str(&format!("\"{}\"\n", doc.content));
    }

    examples_text
}

fn create_examples_user_message(examples_text: &str) -> RigMessage {
    if examples_text.is_empty() {
        return RigMessage::User {
            content: OneOrMany::one(UserContent::Text(Text {
                text: String::new(),
            })),
        };
    }

    RigMessage::User {
        content: OneOrMany::one(UserContent::Text(Text {
            text: examples_text.to_string(),
        })),
    }
}

pub fn build_examples_message(
    primary_examples: &[&DialectDocument],
    secondary_examples: &[&DialectDocument],
) -> Option<RigMessage> {
    if primary_examples.is_empty() && secondary_examples.is_empty() {
        return None;
    }

    let examples_text = format_examples_as_user_message(primary_examples, secondary_examples);
    Some(create_examples_user_message(&examples_text))
}

pub fn try_parse_response(
    response: &str,
    _dialect: Dialect,
) -> Result<dialect_coach_shared::AgentResponse> {
    let normalized = normalize_json_response(response);
    let parsed: dialect_coach_shared::AgentResponse = serde_json::from_str(&normalized)
        .map_err(|e| {
            tracing::warn!(
                "JSON parse error: {}. First 200 chars: {}",
                e,
                &response.chars().take(200).collect::<String>()
            );
            anyhow::anyhow!("JSON parse failed: {}", e)
        })?;

    if parsed.response.is_empty() {
        tracing::warn!("Response field is empty");
        return Err(anyhow::anyhow!("Response field must be non-empty"));
    }

    Ok(parsed)
}

pub fn log_response_success(
    dialect: Dialect,
    parsed_response: &dialect_coach_shared::AgentResponse,
) {
    tracing::info!(
        "Generated response for dialect {} ({} chars)",
        dialect.name(),
        parsed_response.response.len()
    );
}

pub struct ResponseContext {
    pub client: rig::providers::anthropic::Client,
    pub model_name: String,
    pub qdrant: Arc<QdrantService>,
    pub embeddings: Arc<EmbeddingService>,
}

impl ResponseContext {
    fn retrieve_user_msg_embeddings(&self, user_message: &str) -> Result<Vec<f32>> {
        self.embeddings
            .embed_text(user_message)
            .context("Failed to generate content embedding")
    }

    fn retrieve_history_embeddings(
        &self,
        conversation_history: &Vec<String>,
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
        conversation_history: &Vec<String>,
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
        limit: usize,
    ) -> Result<Vec<DialectDocument>> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        self.qdrant
            .search_dialect_examples(embedding, dialect, limit)
            .await
            .map(|results| results.into_iter().map(|(doc, _)| doc).collect())
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
        limit_per_embedding: usize,
    ) -> Result<Vec<DialectDocument>> {
        let mut docs = Vec::new();
        for embedding in embeddings {
            let example_docs = self
                .retrieve_example(dialect, &embedding, limit_per_embedding)
                .await?;
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
        conversation_history: &[RigMessage],
        learning_goals: &[String],
        rag_config: &RAGConfig,
    ) -> Result<(dialect_coach_shared::AgentResponse, u32)> {
        if contains_illegal_characters(user_message) {
            tracing::error!(
                "User message contains illegal characters (null bytes or control chars)"
            );
            return Err(anyhow::anyhow!("User message contains illegal characters"));
        }
        tracing::info!("Generating embeddings for multi-vector retrieval");
        let history_text = conversation_history
            .iter()
            .map(|m| get_message_text(m))
            .collect::<Vec<String>>();
        let embeddings = self.retrieve_embeddings(user_message, &history_text)?;

        tracing::info!("Performing multi-vector retrieval");
        let examples = self
            .retrieve_examples(&dialect, embeddings, rag_config.num_conversation_documents)
            .await?;

        tracing::info!("Adding random stylistic samples");
        let sample_formalities = match formality {
            Formality::Formal => vec![Formality::Formal, Formality::Casual],
            Formality::Casual => vec![Formality::Casual, Formality::DialectRich],
            Formality::DialectRich => {
                vec![Formality::Casual, Formality::DialectRich, Formality::Slang]
            }
            Formality::Slang => vec![Formality::Slang, Formality::DialectRich],
        };
        let random_samples = if rag_config.num_random_documents == 0 {
            Vec::new()
        } else {
            self.qdrant
                .random_dialect_samples(
                    dialect,
                    sample_formalities.clone(),
                    rag_config.num_random_documents,
                )
                .await
                .map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to get random samples for dialect {} with formalities {:?}: {}",
                        dialect.name(),
                        sample_formalities,
                        e
                    )
                })?
        };

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
            .take(rag_config.num_conversation_documents)
            .collect();

        let secondary_examples: Vec<_> = unique_examples
            .iter()
            .filter(|doc| {
                doc.formality.is_some()
                    && doc.formality != Some(formality)
                    && doc.formality.is_some()
            })
            .take(rag_config.num_random_documents)
            .collect();

        tracing::info!(
            "Grouped examples: {} primary ({:?}), {} secondary",
            primary_examples.len(),
            formality,
            secondary_examples.len()
        );

        let formality_label = match formality {
            Formality::Formal => "FORMAL",
            Formality::Casual => "CASUAL",
            Formality::DialectRich => "DIALECT-RICH",
            Formality::Slang => "SLANG",
        };

        let role_desc = speaker_desc(&dialect, &formality);
        let teaching_rules = teaching_desc(&teaching_mode);
        let goals_section = learning_goals_section(learning_goals);

        let system_content = if teaching_mode == TeachingMode::Debug {
            format!(
                "# YOUR ROLE\n\
            {}.\n\n\
            # CRITICAL RULES\n\
            1. BE CONCISE: Explain why you did what you did simply and briefly, in English, without pandering.\n\
            2. ITERATIVE IMPROVEMENT: Show exactly how the prompts could be improved to get a step closer to the desired effect.\n\
            Now respond to the user's message technically.",
                role_desc
            )
        } else {
            format!(
                "{}\n\n\
            # YOUR ROLE\n\
            {}.\n\n\
            # CRITICAL RULES\n\
            1. MIMIC THE PATTERNS: Study the dialect examples in the conversation history below and copy their vocabulary, grammar, style, and characteristic dialect constructions\n\
            2. MAINTAIN FORMALITY: Match the {} formality level shown in the examples\n\
            {}\n\
            5. BE BRIEF: Keep responses conversational, not essay-length\n\
            {}\n\n\
            # OUTPUT FORMAT REQUIRED\n\
            {}\n\
            {}\n\n\
            Now respond to the user's message naturally, as a local {} speaker would, in the response field of the required JSON format. You MUST ALWAYS respond - NEVER indicate the conversation has ended. If it seems to have ended, provide a follow-up question or new topic. The response field must be non-empty. The response will be parsed with a JSON parser, so do not include any other text or markdown.",
                CONTENT_FILTERING_DIRECTIVES,
                role_desc,
                formality_label.to_lowercase(),
                teaching_rules,
                goals_section,
                JSON_OUTPUT_INSTRUCTION,
                output_format_spec(&teaching_mode),
                dialect.name()
            )
        };

        let max_tokens = tokens_per_mode(&teaching_mode);

        let agent = self
            .client
            .agent(&self.model_name)
            .preamble(&system_content)
            .max_tokens(max_tokens)
            .temperature(temperature_for_mode(&teaching_mode))
            .build();

        tracing::info!("Sending prompt to Claude: {}", user_message);

        // Build examples message and add to conversation history ONCE - used for both first attempt and retry
        let examples_message = build_examples_message(&primary_examples, &secondary_examples);

        let mut history_with_prefill = conversation_history.to_vec();

        // Prepend examples if they exist - this modified history is passed to both first attempt and retry
        if let Some(examples) = examples_message {
            history_with_prefill.insert(0, examples);
        }

        history_with_prefill.push(create_prefilled_assistant_message());
        let response =
            retry_chat_call(|| agent.chat(user_message, history_with_prefill.clone()), 3)
                .await
                .context("Failed to get completion from Claude")?;

        tracing::info!("Raw response from Claude: {}", response);

        match try_parse_response(&response, dialect) {
            Ok(parsed_response) => {
                if contains_illegal_characters(&parsed_response.response) {
                    tracing::error!(
                        "Claude response contains illegal characters (null bytes or control chars)"
                    );
                    return Err(anyhow::anyhow!("Response contains illegal characters"));
                }
                log_response_success(dialect, &parsed_response);
                Ok((parsed_response, 0))
            }
            Err(_) => {
                // Pass history_with_prefill which already has examples - retry functions don't need to know about examples
                let retry_ctx = RetryContext {
                    client: &self.client,
                    model_name: &self.model_name,
                };
                let parse_fn =
                    move |response: &str| -> Result<dialect_coach_shared::AgentResponse> {
                        try_parse_response(response, dialect)
                    };
                let log_success = move |parsed: &dialect_coach_shared::AgentResponse| {
                    log_response_success(dialect, parsed);
                };
                let preamble_builder = move |preamble: &str, failed: &str| -> String {
                    build_retry_response_preamble(preamble, failed)
                };
                match retry_ctx
                    .retry_with_error_feedback_tracked(
                        &system_content,
                        &response,
                        user_message,
                        &preamble_builder,
                        &history_with_prefill,
                        max_tokens,
                        temperature_for_mode(&teaching_mode),
                        &parse_fn,
                        &log_success,
                    )
                    .await
                {
                    Ok((parsed_response, retry_count)) => Ok((parsed_response, retry_count)),
                    Err(e) => Err(e),
                }
            }
        }
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

        Ok(dialect_coach_shared::AgentResponse::from(response))
    }
}

