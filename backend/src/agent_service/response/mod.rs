use anyhow::{Context, Result};
use dialect_coach_shared::{
    AgentUsage, Dialect, DialectDocument, DialectWithFeatures, Explained, Exploratory, Formality,
    LanguageLevel, LanguageOption, LearningGoal, Mistake, PastLearningItems, TeachingMode,
    Translated, UserGender,
};
use rig::completion::Message as RigMessage;
use std::sync::Arc;

use crate::embedding_service::EmbeddingService;
use crate::qdrant_service::QdrantService;
use crate::rag_config::RAGConfig;

use super::learning::{LearningAgent, LearningAgentOutput, LearningAgentParams};
use super::provider::{CompletionAgent, CompletionRequest};
use super::retry::{RetryContext, build_retry_response_preamble, retry_completion_call};
use super::util::{
    clean_response, contains_illegal_characters, get_message_text, normalize_json_response,
};

mod config;
use config::{temperature_for_mode, tokens_per_mode};

mod speaker;

mod teaching;

mod examples;
use examples::{
    build_conversation_history_with_examples, deduplicate_examples, get_sample_formalities,
    group_examples_by_formality,
};

mod system_content;
use system_content::build_system_content;

fn apply_learning_output(
    mut base: dialect_coach_shared::AgentResponse,
    output: LearningAgentOutput,
) -> dialect_coach_shared::AgentResponse {
    base.mistakes = Some(output.mistakes);
    base.explained = Some(output.explained);
    base.translated = Some(output.translated);
    base.exploratory = Some(output.exploratory);
    base
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


fn build_simple_completion_request<'a>(
    system_preamble: &'a str,
    prompt: &'a str,
    history: &'a [RigMessage],
) -> CompletionRequest<'a> {
    CompletionRequest {
        preamble: system_preamble,
        prompt,
        history,
        max_tokens: 1024,
        temperature: 0.0,
    }
}

fn sanitize_simple_json_response(response_text: &str) -> Result<String> {
    let cleaned = clean_response(response_text);
    let trimmed = cleaned.trim();

    if trimmed.is_empty() {
        return Err(anyhow::anyhow!(
            "Simple response was empty; expected content"
        ));
    }

    Ok(trimmed.to_string())
}




/// Parameters for generating a response
pub struct GenerateResponseParams<'a> {
    pub user_message: &'a str,
    pub dialect: DialectWithFeatures,
    pub formality: Formality,
    pub teaching_mode: TeachingMode,
    pub conversation_history: &'a [RigMessage],
    pub learning_goals: &'a [LearningGoal],
    pub rag_config: &'a RAGConfig,
    pub past_mistakes: &'a [Mistake],
    pub past_explained: &'a [Explained],
    pub past_translated: &'a [Translated],
    pub past_exploratory: &'a [Exploratory],
    pub user_gender: UserGender,
    pub language_option: &'a Option<LanguageOption>,
    pub active_plan: Option<&'a dialect_coach_shared::LanguagePlan>,
    pub language_level: LanguageLevel,
}

pub struct ResponseContext {
    pub response_agent: Arc<dyn CompletionAgent>,
    pub learning_agent: Arc<dyn CompletionAgent>,
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
        dialect: DialectWithFeatures,
        formality: Formality,
        rag_config: &RAGConfig,
    ) -> Result<(Vec<DialectDocument>, Vec<DialectDocument>)> {
        if !dialect.has_corpus {
            return Ok((Vec::new(), Vec::new()));
        }
        let history_text = conversation_history
            .iter()
            .map(get_message_text)
            .collect::<Vec<String>>();
        let embeddings = self.retrieve_embeddings(user_message, &history_text)?;
        let examples = self
            .retrieve_examples(
                &dialect.dialect,
                embeddings,
                rag_config.num_conversation_documents,
            )
            .await?;
        let sample_formalities = get_sample_formalities(formality);
        let random_samples = self
            .retrieve_random_samples(
                dialect.dialect,
                sample_formalities,
                rag_config.num_random_documents,
            )
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

    async fn attach_learning_items(
        &self,
        params: &GenerateResponseParams<'_>,
        parsed_response: dialect_coach_shared::AgentResponse,
    ) -> Result<(dialect_coach_shared::AgentResponse, Vec<AgentUsage>)> {
        let assistant_response = parsed_response.response.clone();
        let learning_agent = LearningAgent::new(self.learning_agent.clone());
        let learning_params = LearningAgentParams {
            user_message: params.user_message,
            assistant_response: &assistant_response,
            dialect: params.dialect.dialect,
            formality: params.formality,
            teaching_mode: params.teaching_mode,
            learning_goals: params.learning_goals,
            past_mistakes: params.past_mistakes,
            past_explained: params.past_explained,
            past_translated: params.past_translated,
            past_exploratory: params.past_exploratory,
            language_option: params.language_option,
        };
        let (result, usage) = learning_agent
            .generate_learning_items(&learning_params)
            .await;
        match result {
            Ok(output) => Ok((apply_learning_output(parsed_response, output), usage)),
            Err(e) => Err(e),
        }
    }

    async fn handle_successful_completion(
        &self,
        response: String,
        usage: Vec<AgentUsage>,
        params: &GenerateResponseParams<'_>,
        system_content: &str,
        history_with_prefill: Vec<RigMessage>,
        skip_learning: bool,
    ) -> (
        Result<dialect_coach_shared::AgentResponse, anyhow::Error>,
        Vec<AgentUsage>,
        Vec<AgentUsage>,
    ) {
        match self
            .handle_response_parsing(
                response,
                usage.clone(),
                params,
                system_content,
                history_with_prefill,
                skip_learning,
            )
            .await
        {
            Ok((agent_response, response_usage, learning_usage)) => {
                (Ok(agent_response), response_usage, learning_usage)
            }
            Err(e) => {
                tracing::error!(
                    error = %e,
                    "Response processing failed after provider completion"
                );
                (Err(e), usage, Vec::new())
            }
        }
    }

    async fn handle_response_parsing(
        &self,
        response: String,
        initial_usage: Vec<AgentUsage>,
        params: &GenerateResponseParams<'_>,
        system_content: &str,
        history_with_prefill: Vec<RigMessage>,
        skip_learning: bool,
    ) -> Result<(
        dialect_coach_shared::AgentResponse,
        Vec<AgentUsage>,
        Vec<AgentUsage>,
    )> {
        tracing::debug!(
            dialect = %params.dialect.dialect.name(),
            "Raw provider response: {}",
            response
        );
        match try_parse_response(&response, params.dialect.dialect) {
            Ok(parsed_response) => {
                if contains_illegal_characters(&parsed_response.response) {
                    tracing::error!(
                        "Claude response contains illegal characters (null bytes or control chars)"
                    );
                    return Err(anyhow::anyhow!("Response contains illegal characters"));
                }
                log_response_success(params.dialect.dialect, &parsed_response);
                if skip_learning {
                    Ok((parsed_response, initial_usage, Vec::new()))
                } else {
                    match self.attach_learning_items(params, parsed_response).await {
                        Ok((final_response, learning_usage)) => {
                            Ok((final_response, initial_usage, learning_usage))
                        }
                        Err(e) => {
                            tracing::error!(
                                dialect = %params.dialect.dialect.name(),
                                error = %e,
                                "Failed to attach learning items to parsed response"
                            );
                            Err(e)
                        }
                    }
                }
            }
            Err(_) => {
                let dialect = params.dialect.dialect;
                let teaching_mode = params.teaching_mode;
                let retry_ctx = RetryContext {
                    agent: self.response_agent.clone(),
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
                    Ok((parsed_response, retry_usage)) => {
                        let mut all_response_usage = initial_usage;
                        all_response_usage.extend(retry_usage);
                        if skip_learning {
                            Ok((parsed_response, all_response_usage, Vec::new()))
                        } else {
                            match self.attach_learning_items(params, parsed_response).await {
                                Ok((final_response, learning_usage)) => {
                                    Ok((final_response, all_response_usage, learning_usage))
                                }
                                Err(e) => Err(e),
                            }
                        }
                    }
                    Err(e) => Err(e),
                }
            }
        }
    }

    pub async fn generate_response(
        &self,
        params: &GenerateResponseParams<'_>,
        skip_learning: bool,
    ) -> (
        Result<dialect_coach_shared::AgentResponse, anyhow::Error>,
        Vec<AgentUsage>,
        Vec<AgentUsage>,
    ) {
        if contains_illegal_characters(params.user_message) {
            tracing::error!(
                dialect = %params.dialect.dialect.name(),
                "User message contains illegal control characters; aborting response generation"
            );
            return (
                Err(anyhow::anyhow!("User message contains illegal characters")),
                Vec::new(),
                Vec::new(),
            );
        }

        let (primary_examples, secondary_examples) = match self
            .collect_examples(
                params.user_message,
                params.conversation_history,
                params.dialect.clone(),
                params.formality,
                params.rag_config,
            )
            .await
        {
            Ok(examples) => examples,
            Err(e) => {
                tracing::error!(
                    dialect = %params.dialect.dialect.name(),
                    error = %e,
                    "Failed to collect RAG examples for response generation"
                );
                return (Err(e), Vec::new(), Vec::new());
            }
        };

        let past_learning_items = PastLearningItems {
            mistakes: params.past_mistakes.to_vec(),
            explained: params.past_explained.to_vec(),
            translated: params.past_translated.to_vec(),
            exploratory: params.past_exploratory.to_vec(),
        };
        let system_content = build_system_content(
            params.dialect.clone(),
            params.formality,
            params.teaching_mode,
            params.learning_goals,
            &past_learning_items,
            params.user_gender,
            params.language_option,
            &params.active_plan,
            params.language_level,
        );
        tracing::debug!("System content sent to Claude:\n{}", system_content);
        let history_with_prefill = build_conversation_history_with_examples(
            params.conversation_history,
            &primary_examples,
            &secondary_examples,
            self.response_agent.provider(),
        );
        let request = CompletionRequest {
            preamble: &system_content,
            prompt: params.user_message,
            history: &history_with_prefill,
            max_tokens: tokens_per_mode(&params.teaching_mode),
            temperature: temperature_for_mode(&params.teaching_mode),
        };

        let (result, usage) =
            retry_completion_call(self.response_agent.as_ref(), &request, 3).await;
        match result {
            Ok(response) => {
                self.handle_successful_completion(
                    response,
                    usage,
                    params,
                    &system_content,
                    history_with_prefill,
                    skip_learning,
                )
                .await
            }
            Err(e) => {
                let provider = self.response_agent.provider();
                let model = self.response_agent.model();
                let error_text = format!("{}", e);
                tracing::error!(
                    provider = %provider,
                    model = %model,
                    error = %error_text,
                    "Provider completion failed before response parsing"
                );
                (
                    Err(e.context(format!(
                        "Failed to get completion from provider {} model {}: {}",
                        provider, model, error_text
                    ))),
                    usage,
                    Vec::new(),
                )
            }
        }
    }

    pub async fn generate_simple_response(
        &self,
        system_preamble: &str,
        prompt: &str,
        history: Vec<RigMessage>,
    ) -> Result<dialect_coach_shared::AgentResponse> {
        let request = build_simple_completion_request(system_preamble, prompt, &history);
        let (result, _) = retry_completion_call(self.response_agent.as_ref(), &request, 1).await;
        let text = result.context(format!(
            "Failed to get translation from provider {} model {}",
            self.response_agent.provider(),
            self.response_agent.model()
        ))?;
        let sanitized = sanitize_simple_json_response(&text)?;
        Ok(dialect_coach_shared::AgentResponse::from(sanitized))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_service::provider::ANTHROPIC_PROVIDER;
    use dialect_coach_shared::models::dialect::dialect_features;
    use dialect_coach_shared::{Dialect, Formality, TeachingMode};
    use rig::completion::{Message as RigMessage, message::Text, message::UserContent};
    use rig::one_or_many::OneOrMany;


    #[test]
    fn test_build_system_content_debug() {
        let dialect = Dialect::SpanishMexican;
        let formality = Formality::Informal;
        let teaching_mode = TeachingMode::Debug;
        let learning_goals = vec![];
        let past_learning_items = PastLearningItems::default();

        let content = build_system_content(
            dialect_features(dialect),
            formality,
            teaching_mode,
            &learning_goals,
            &past_learning_items,
            UserGender::NonBinary,
            &None,
            &None,
            LanguageLevel::B1,
        );

        assert!(content.contains("# YOUR ROLE"));
        assert!(content.contains("BE CONCISE"));
        assert!(content.contains("ITERATIVE IMPROVEMENT"));
        assert!(content.contains("LANGUAGE LEVEL"));
        assert!(content.contains("technically"));
    }

    #[test]
    fn test_build_system_content_normal() {
        let dialect = Dialect::SpanishArgentinian;
        let formality = Formality::Informal;
        let teaching_mode = TeachingMode::Immersive;
        let learning_goals = vec![LearningGoal {
            goal: "Goal 1".to_string(),
            dialect: Dialect::SpanishArgentinian,
        }];
        let past_learning_items = PastLearningItems::default();

        let content = build_system_content(
            dialect_features(dialect),
            formality,
            teaching_mode,
            &learning_goals,
            &past_learning_items,
            UserGender::NonBinary,
            &None,
            &None,
            LanguageLevel::A2,
        );

        assert!(content.contains("# YOUR ROLE"));
        assert!(content.contains("MIMIC THE PATTERNS"));
        assert!(content.contains("MAINTAIN FORMALITY"));
        assert!(content.contains("informal"));
        assert!(content.contains("# LEARNING GOALS"));
        assert!(content.contains("Goal 1"));
        assert!(content.contains("LANGUAGE LEVEL"));
    }


    #[test]
    fn test_build_simple_completion_request_uses_zero_temperature() {
        let request = build_simple_completion_request("sys", "prompt", &[]);
        assert_eq!(request.temperature, 0.0);
        assert_eq!(request.max_tokens, 1024);
        assert_eq!(request.preamble, "sys");
        assert_eq!(request.prompt, "prompt");
    }

    #[test]
    fn test_sanitize_simple_json_response_strips_markdown_and_validates() {
        let wrapped = "```json\n{\"response\":\"hola\"}\n```";
        let sanitized = sanitize_simple_json_response(wrapped).unwrap();
        let value: serde_json::Value = serde_json::from_str(&sanitized).unwrap();
        assert_eq!(value["response"], "hola");
        assert!(!sanitized.contains("```"));
    }

    #[test]
    fn test_build_system_content_with_active_plan() {
        use crate::agent_service::response::system_content::build_plan_system_content;
        use dialect_coach_shared::models::Dialect;
        use dialect_coach_shared::models::UserState;
        use dialect_coach_shared::models::learning_item::{LearningItem, LearningItemType};
        use dialect_coach_shared::models::{
            LanguagePlan, PlanContent, PlanStep, StepType, Translated,
        };
        use uuid::Uuid;

        let mut content = PlanContent::default();
        content.items.push(LearningItem::new(
            LearningItemType::Translation(Translated::new(
                "hola".to_string(),
                "hello".to_string(),
                None,
            )),
            Dialect::SpanishMexican,
        ));

        let current_step = PlanStep::new(
            1,
            "Intro".to_string(),
            StepType::Learning { content },
            "Learn basic greetings".to_string(),
        );

        let plan = LanguagePlan::new(
            "Test Plan".to_string(),
            Dialect::SpanishMexican,
            None,
            vec![current_step],
        );

        let mut state = UserState::new(Uuid::new_v4());
        state.language_plans.push(plan.clone());
        state.active_plan_id = Some(plan.id);

        // Act
        let system_content = build_plan_system_content(&Some(&plan));

        // Assert
        assert!(system_content.contains("RELEVANT LEARNING CONTENT"));
        assert!(system_content.contains("hello"));
        assert!(system_content.contains("Naturally incorporate"));
    }

    #[test]
    fn test_build_plan_system_content() {
        use crate::agent_service::response::system_content::build_plan_system_content;
        use dialect_coach_shared::models::learning_item::{LearningItem, LearningItemType};
        use dialect_coach_shared::models::plan::{PlanContent, PlanStep, StepType};
        use dialect_coach_shared::models::{Dialect, Explained, Translated};

        let mut content = PlanContent::default();

        content.items.push(LearningItem::new(
            LearningItemType::Translation(Translated::new(
                "hola".to_string(),
                "hello".to_string(),
                None,
            )),
            Dialect::SpanishMexican,
        ));
        content.items.push(LearningItem::new(
            LearningItemType::Explanation(Explained::new(
                "que onda".to_string(),
                "what's up".to_string(),
            )),
            Dialect::SpanishMexican,
        ));

        let step = PlanStep::new(
            1,
            "Test Step".to_string(),
            StepType::Learning { content },
            "Use these words".to_string(),
        );

        let plan = dialect_coach_shared::LanguagePlan::new(
            "Test Plan".to_string(),
            Dialect::SpanishMexican,
            None,
            vec![step],
        );

        let prompt = build_plan_system_content(&Some(&plan));

        // Check for key prompt elements
        assert!(
            prompt.contains("ACTIVE LANGUAGE PLAN"),
            "Should identify as active plan"
        );
        assert!(prompt.contains("Test Step"), "Should contain step title");
        assert!(
            prompt.contains("Use these words"),
            "Should contain instructions"
        );

        // Check content rendering
        assert!(
            prompt.contains("hello"),
            "Should contain translation target"
        );
        assert!(
            prompt.contains("que onda"),
            "Should contain explanation phrase"
        );

        // Check new goal instruction
        assert!(
            prompt.contains("Naturally incorporate"),
            "Should contain new goal instruction"
        );
        assert!(
            !prompt.contains("correct them gently"),
            "Should NOT contain old goal instruction"
        );
    }

    #[test]
    fn test_language_level_instruction_covers_all_levels() {
        use crate::agent_service::response::teaching::language_level_instruction;
        let levels = [
            LanguageLevel::A1,
            LanguageLevel::A2,
            LanguageLevel::B1,
            LanguageLevel::B2,
            LanguageLevel::C1,
            LanguageLevel::C2,
        ];
        for level in levels {
            let instruction = language_level_instruction(level);
            assert!(!instruction.is_empty());
            assert!(instruction.contains("LANGUAGE LEVEL"));
        }
    }

    #[test]
    fn test_build_plan_system_content_review_step() {
        use crate::agent_service::response::system_content::build_plan_system_content;
        use dialect_coach_shared::models::learning_item::{LearningItem, LearningItemType};
        use dialect_coach_shared::models::plan::{PlanContent, PlanStep, StepType};
        use dialect_coach_shared::models::{Dialect, Translated};

        // Create a learning step with content
        let mut content1 = PlanContent::default();
        content1.items.push(LearningItem::new(
            LearningItemType::Translation(Translated::new(
                "gracias".to_string(),
                "thanks".to_string(),
                None,
            )),
            Dialect::SpanishMexican,
        ));

        let step1 = PlanStep::new(
            1,
            "Learning Step".to_string(),
            StepType::Learning { content: content1 },
            "Learn this".to_string(),
        );

        // Create a review step referencing step1
        let step2 = PlanStep::new(
            2,
            "Review Step".to_string(),
            StepType::Review {
                review_step_ids: vec![step1.id],
            },
            "Review this".to_string(),
        );

        let plan = dialect_coach_shared::LanguagePlan {
            id: uuid::Uuid::new_v4(),
            title: "Test Plan".to_string(),
            dialect: Dialect::SpanishMexican,
            description: None,
            steps: vec![step1, step2],
            current_step_index: 1, // Set to Review step
            status: dialect_coach_shared::models::plan::PlanStatus::InProgress,
            created_at: 0,
        };

        let prompt = build_plan_system_content(&Some(&plan));

        assert!(prompt.contains("Current Step: Review Step"));
        assert!(
            prompt.contains("REVIEW MATERIALS"),
            "Should identify materials as review materials"
        );
        assert!(
            prompt.contains("thanks"),
            "Should contain content from referenced step"
        );
        assert!(
            prompt.contains("The user has learned the listed materials"),
            "Should contain review-specific goal instruction"
        );
    }
}
