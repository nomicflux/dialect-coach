use anyhow::{Context, Result};
use dialect_coach_shared::{Dialect, DialectDocument, Formality, TeachingMode};
use rig::completion::{Chat, Message as RigMessage, Prompt, message::Text, message::UserContent};
use rig::one_or_many::OneOrMany;
use std::sync::Arc;

use crate::embedding_service::EmbeddingService;
use crate::qdrant_service::QdrantService;
use crate::rag_config::RAGConfig;

use super::retry::{RetryContext, build_retry_response_preamble, retry_chat_call};
use super::util::{
    CONTENT_FILTERING_DIRECTIVES, JSON_OUTPUT_INSTRUCTION, contains_illegal_characters,
    create_prefilled_assistant_message, get_message_text, learning_goals_section,
    normalize_json_response, output_format_spec, speaker_desc, teaching_desc, temperature_for_mode,
    tokens_per_mode,
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
    let parsed: dialect_coach_shared::AgentResponse =
        serde_json::from_str(&normalized).map_err(|e| {
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

fn get_sample_formalities(formality: Formality) -> Vec<Formality> {
    match formality {
        Formality::Formal => vec![Formality::Formal, Formality::Casual],
        Formality::Casual => vec![Formality::Casual, Formality::DialectRich],
        Formality::DialectRich => {
            vec![Formality::Casual, Formality::DialectRich, Formality::Slang]
        }
        Formality::Slang => vec![Formality::Slang, Formality::DialectRich],
    }
}

fn deduplicate_examples(
    examples: Vec<DialectDocument>,
    random_samples: Vec<DialectDocument>,
) -> Vec<DialectDocument> {
    let mut all_examples = Vec::new();
    all_examples.extend(examples);
    all_examples.extend(random_samples);

    let mut seen = std::collections::HashSet::new();
    all_examples
        .into_iter()
        .filter(|doc| seen.insert(doc.content.clone()))
        .take(50)
        .collect()
}

fn group_examples_by_formality(
    examples: &[DialectDocument],
    formality: Formality,
    primary_limit: usize,
    secondary_limit: usize,
) -> (Vec<DialectDocument>, Vec<DialectDocument>) {
    let primary_examples: Vec<_> = examples
        .iter()
        .filter(|doc| doc.formality.is_none() || doc.formality == Some(formality))
        .take(primary_limit)
        .cloned()
        .collect();

    let secondary_examples: Vec<_> = examples
        .iter()
        .filter(|doc| {
            doc.formality.is_some() && doc.formality != Some(formality) && doc.formality.is_some()
        })
        .take(secondary_limit)
        .cloned()
        .collect();

    (primary_examples, secondary_examples)
}

fn build_system_content(
    dialect: Dialect,
    formality: Formality,
    teaching_mode: TeachingMode,
    learning_goals: &[String],
) -> String {
    let formality_label = match formality {
        Formality::Formal => "FORMAL",
        Formality::Casual => "CASUAL",
        Formality::DialectRich => "DIALECT-RICH",
        Formality::Slang => "SLANG",
    };

    let role_desc = speaker_desc(&dialect, &formality);
    let teaching_rules = teaching_desc(&teaching_mode);
    let goals_section = learning_goals_section(learning_goals);

    if teaching_mode == TeachingMode::Debug {
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
    }
}

fn build_conversation_history_with_examples(
    conversation_history: &[RigMessage],
    primary_examples: &[DialectDocument],
    secondary_examples: &[DialectDocument],
) -> Vec<RigMessage> {
    let primary_refs: Vec<_> = primary_examples.iter().collect();
    let secondary_refs: Vec<_> = secondary_examples.iter().collect();
    let examples_message = build_examples_message(&primary_refs, &secondary_refs);
    let mut history_with_prefill = conversation_history.to_vec();

    if let Some(examples) = examples_message {
        history_with_prefill.insert(0, examples);
    }

    history_with_prefill.push(create_prefilled_assistant_message());
    history_with_prefill
}

/// Parameters for generating a response
pub struct GenerateResponseParams<'a> {
    pub user_message: &'a str,
    pub dialect: Dialect,
    pub formality: Formality,
    pub teaching_mode: TeachingMode,
    pub conversation_history: &'a [RigMessage],
    pub learning_goals: &'a [String],
    pub rag_config: &'a RAGConfig,
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

    async fn retrieve_random_samples(
        &self,
        dialect: Dialect,
        formalities: Vec<Formality>,
        num_samples: usize,
    ) -> Result<Vec<DialectDocument>> {
        if num_samples == 0 {
            return Ok(Vec::new());
        }
        self.qdrant
            .random_dialect_samples(dialect, formalities.clone(), num_samples)
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to get random samples for dialect {} with formalities {:?}: {}",
                    dialect.name(),
                    formalities,
                    e
                )
            })
    }

    async fn collect_examples(
        &self,
        user_message: &str,
        conversation_history: &[RigMessage],
        dialect: Dialect,
        formality: Formality,
        rag_config: &RAGConfig,
    ) -> Result<(Vec<DialectDocument>, Vec<DialectDocument>)> {
        let history_text = conversation_history
            .iter()
            .map(get_message_text)
            .collect::<Vec<String>>();
        let embeddings = self.retrieve_embeddings(user_message, &history_text)?;
        let examples = self
            .retrieve_examples(&dialect, embeddings, rag_config.num_conversation_documents)
            .await?;
        let sample_formalities = get_sample_formalities(formality);
        let random_samples = self
            .retrieve_random_samples(dialect, sample_formalities, rag_config.num_random_documents)
            .await?;
        let unique_examples = deduplicate_examples(examples, random_samples);
        let (primary, secondary) = group_examples_by_formality(
            &unique_examples,
            formality,
            rag_config.num_conversation_documents,
            rag_config.num_random_documents,
        );
        Ok((primary, secondary))
    }

    fn create_agent(&self, system_content: &str, teaching_mode: TeachingMode) -> impl Chat {
        let max_tokens = tokens_per_mode(&teaching_mode);
        self.client
            .agent(&self.model_name)
            .preamble(system_content)
            .max_tokens(max_tokens)
            .temperature(temperature_for_mode(&teaching_mode))
            .build()
    }

    async fn handle_response_parsing(
        &self,
        response: String,
        params: &GenerateResponseParams<'_>,
        system_content: &str,
        history_with_prefill: Vec<RigMessage>,
    ) -> Result<(dialect_coach_shared::AgentResponse, u32)> {
        match try_parse_response(&response, params.dialect) {
            Ok(parsed_response) => {
                if contains_illegal_characters(&parsed_response.response) {
                    tracing::error!(
                        "Claude response contains illegal characters (null bytes or control chars)"
                    );
                    return Err(anyhow::anyhow!("Response contains illegal characters"));
                }
                log_response_success(params.dialect, &parsed_response);
                Ok((parsed_response, 0))
            }
            Err(_) => {
                let dialect = params.dialect;
                let teaching_mode = params.teaching_mode;
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
                let max_tokens = tokens_per_mode(&teaching_mode);
                let prompt_params = super::retry::RetryPromptParams {
                    original_preamble: system_content,
                    failed_response: &response,
                    prompt: params.user_message,
                    preamble_builder: &preamble_builder,
                };
                let config = super::util::GenerationConfig {
                    max_tokens,
                    temperature: temperature_for_mode(&teaching_mode),
                };
                match retry_ctx
                    .retry_with_error_feedback_tracked(
                        &prompt_params,
                        &history_with_prefill,
                        &config,
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

    pub async fn generate_response(
        &self,
        params: &GenerateResponseParams<'_>,
    ) -> Result<(dialect_coach_shared::AgentResponse, u32)> {
        if contains_illegal_characters(params.user_message) {
            return Err(anyhow::anyhow!("User message contains illegal characters"));
        }

        let (primary_examples, secondary_examples) = self
            .collect_examples(
                params.user_message,
                params.conversation_history,
                params.dialect,
                params.formality,
                params.rag_config,
            )
            .await?;

        let system_content = build_system_content(
            params.dialect,
            params.formality,
            params.teaching_mode,
            params.learning_goals,
        );
        let agent = self.create_agent(&system_content, params.teaching_mode);
        let history_with_prefill = build_conversation_history_with_examples(
            params.conversation_history,
            &primary_examples,
            &secondary_examples,
        );

        let response = retry_chat_call(
            || agent.chat(params.user_message, history_with_prefill.clone()),
            3,
        )
        .await
        .context("Failed to get completion from Claude")?;

        self.handle_response_parsing(response, params, &system_content, history_with_prefill)
            .await
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

#[cfg(test)]
mod tests {
    use super::*;
    use dialect_coach_shared::{Dialect, Formality, TeachingMode};
    use rig::completion::{Message as RigMessage, message::Text, message::UserContent};
    use rig::one_or_many::OneOrMany;

    #[test]
    fn test_get_sample_formalities() {
        let formal = get_sample_formalities(Formality::Formal);
        assert_eq!(formal, vec![Formality::Formal, Formality::Casual]);

        let casual = get_sample_formalities(Formality::Casual);
        assert_eq!(casual, vec![Formality::Casual, Formality::DialectRich]);

        let dialect_rich = get_sample_formalities(Formality::DialectRich);
        assert_eq!(
            dialect_rich,
            vec![Formality::Casual, Formality::DialectRich, Formality::Slang]
        );

        let slang = get_sample_formalities(Formality::Slang);
        assert_eq!(slang, vec![Formality::Slang, Formality::DialectRich]);
    }

    #[test]
    fn test_deduplicate_examples() {
        let doc1 = DialectDocument::new("Hello".to_string(), Dialect::SpanishMexican, None);
        let doc2 = DialectDocument::new("Hola".to_string(), Dialect::SpanishMexican, None);
        let doc3 = DialectDocument::new("Hello".to_string(), Dialect::SpanishMexican, None);

        let examples = vec![doc1.clone(), doc2.clone()];
        let random_samples = vec![doc3];

        let result = deduplicate_examples(examples, random_samples);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].content, "Hello");
        assert_eq!(result[1].content, "Hola");
    }

    #[test]
    fn test_deduplicate_examples_respects_limit() {
        let mut examples = Vec::new();
        for i in 0..60 {
            examples.push(DialectDocument::new(
                format!("Example {}", i),
                Dialect::SpanishMexican,
                None,
            ));
        }

        let result = deduplicate_examples(examples, Vec::new());
        assert_eq!(result.len(), 50);
    }

    #[test]
    fn test_group_examples_by_formality() {
        let dialect = Dialect::SpanishMexican;
        let doc1 = DialectDocument::new("Hello".to_string(), dialect, Some(Formality::Casual));
        let doc2 = DialectDocument::new("Hola".to_string(), dialect, Some(Formality::Formal));
        let doc3 = DialectDocument::new("Hey".to_string(), dialect, None);
        let doc4 = DialectDocument::new("Hi".to_string(), dialect, Some(Formality::Casual));

        let examples = vec![doc1, doc2, doc3, doc4];
        let (primary, secondary) =
            group_examples_by_formality(&examples, Formality::Casual, 10, 10);

        assert_eq!(primary.len(), 3);
        assert_eq!(secondary.len(), 1);
        assert_eq!(secondary[0].formality, Some(Formality::Formal));
    }

    #[test]
    fn test_group_examples_by_formality_respects_limits() {
        let dialect = Dialect::SpanishMexican;
        let mut examples = Vec::new();
        for i in 0..15 {
            examples.push(DialectDocument::new(
                format!("Example {}", i),
                dialect,
                Some(Formality::Casual),
            ));
        }

        let (primary, _secondary) = group_examples_by_formality(&examples, Formality::Casual, 5, 5);
        assert_eq!(primary.len(), 5);
    }

    #[test]
    fn test_build_system_content_debug() {
        let dialect = Dialect::SpanishMexican;
        let formality = Formality::Casual;
        let teaching_mode = TeachingMode::Debug;
        let learning_goals = vec![];

        let content = build_system_content(dialect, formality, teaching_mode, &learning_goals);

        assert!(content.contains("# YOUR ROLE"));
        assert!(content.contains("BE CONCISE"));
        assert!(content.contains("ITERATIVE IMPROVEMENT"));
        assert!(content.contains("technically"));
    }

    #[test]
    fn test_build_system_content_normal() {
        let dialect = Dialect::SpanishMexican;
        let formality = Formality::Casual;
        let teaching_mode = TeachingMode::Immersive;
        let learning_goals = vec!["Goal 1".to_string()];

        let content = build_system_content(dialect, formality, teaching_mode, &learning_goals);

        assert!(content.contains("# YOUR ROLE"));
        assert!(content.contains("MIMIC THE PATTERNS"));
        assert!(content.contains("MAINTAIN FORMALITY"));
        assert!(content.contains("casual"));
        assert!(content.contains("# LEARNING GOALS"));
        assert!(content.contains("Goal 1"));
    }

    #[test]
    fn test_build_conversation_history_with_examples() {
        let dialect = Dialect::SpanishMexican;
        let doc1 = DialectDocument::new("Hello".to_string(), dialect, Some(Formality::Casual));
        let doc2 = DialectDocument::new("Hola".to_string(), dialect, Some(Formality::Formal));

        let conversation_history = vec![RigMessage::User {
            content: OneOrMany::one(UserContent::Text(Text {
                text: "Test".to_string(),
            })),
        }];

        let primary_examples = vec![doc1];
        let secondary_examples = vec![doc2];

        let history = build_conversation_history_with_examples(
            &conversation_history,
            &primary_examples,
            &secondary_examples,
        );

        assert_eq!(history.len(), 3);
        match &history[0] {
            RigMessage::User { .. } => {}
            _ => panic!("First message should be examples user message"),
        }
        match &history[1] {
            RigMessage::User { .. } => {}
            _ => panic!("Second message should be original history"),
        }
        match &history[2] {
            RigMessage::Assistant { .. } => {}
            _ => panic!("Last message should be prefilled assistant message"),
        }
    }

    #[test]
    fn test_build_conversation_history_without_examples() {
        let conversation_history = vec![RigMessage::User {
            content: OneOrMany::one(UserContent::Text(Text {
                text: "Test".to_string(),
            })),
        }];

        let history = build_conversation_history_with_examples(&conversation_history, &[], &[]);

        assert_eq!(history.len(), 2);
        match &history[0] {
            RigMessage::User { .. } => {}
            _ => panic!("First message should be original history"),
        }
        match &history[1] {
            RigMessage::Assistant { .. } => {}
            _ => panic!("Last message should be prefilled assistant message"),
        }
    }
}
