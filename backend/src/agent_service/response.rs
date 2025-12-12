use anyhow::{Context, Result};
use dialect_coach_shared::{
    AgentUsage, Dialect, DialectDocument, DialectWithFeatures, Explained, Exploratory, Formality,
    Gender, LanguageLevel, LanguageOption, LearningGoal, Mistake, PastLearningItems, TeachingMode,
    Translated, UserGender,
};
use rig::completion::{Message as RigMessage, message::Text, message::UserContent};
use rig::one_or_many::OneOrMany;
use std::sync::Arc;

use crate::embedding_service::EmbeddingService;
use crate::qdrant_service::QdrantService;
use crate::rag_config::RAGConfig;

use super::language_instructions::build_language_instruction;
use super::learning::{LearningAgent, LearningAgentOutput, LearningAgentParams};
use super::provider::{CompletionAgent, CompletionRequest};
use super::retry::{RetryContext, build_retry_response_preamble, retry_completion_call};
use super::util::{
    JSON_OUTPUT_INSTRUCTION, clean_response, contains_illegal_characters,
    create_prefilled_assistant_message, format_learning_items_context, get_message_text,
    learning_goals_section, normalize_json_response,
};

fn temperature_for_mode(mode: &TeachingMode) -> f64 {
    match mode {
        TeachingMode::Immersive => 0.6,
        TeachingMode::Corrective => 0.4,
        TeachingMode::Explanatory => 0.5,
        TeachingMode::Interleaved => 0.4,
        TeachingMode::StoryTeller => 1.0,
        TeachingMode::Debug => 0.1,
    }
}

fn tokens_per_mode(teaching_mode: &TeachingMode) -> u64 {
    match teaching_mode {
        TeachingMode::Immersive => 256,
        TeachingMode::Corrective => 512,
        TeachingMode::Explanatory => 512,
        TeachingMode::Interleaved => 512,
        TeachingMode::StoryTeller => 1024,
        TeachingMode::Debug => 1024,
    }
}

fn speaker_desc(dialect: &Dialect, formality: &Formality, gender: &Gender) -> String {
    let gender_str = match gender {
        Gender::MalePresenting => "male-presenting",
        Gender::FemalePresenting => "female-presenting",
    };
    let base = format!("You are a {} native {} speaker", gender_str, dialect.name());
    match formality {
        Formality::Formal => format!(
            "{} communicating in a professional, polite manner in a formal setting.",
            base
        ),
        Formality::ProfessionalCasual => format!(
            "{} communicating in a professional manner amongst colleagues, using more standard forms than usual but not being rigid in speech.",
            base
        ),
        Formality::Informal => format!(
            "{} speaking conversationally, using dialectal forms when appropriate and natural, and more standard forms when those become difficult to understand.",
            base
        ),
        Formality::Slang => format!("{} using informal slang and colloquialisms.", base),
    }
}

fn response_teaching_desc(teaching_mode: &TeachingMode) -> String {
    let tokens = tokens_per_mode(teaching_mode);
    let desc = match *teaching_mode {
        TeachingMode::Immersive => {
            "3. IMMERSIVE MODE: Keep responses brief and conversational - just chat naturally without explanations or corrections."
        }
        TeachingMode::Corrective => {
            r#"3. CORRECTIVE MODE: Respond naturally, warmly but concisely (hard limit of 1-2 short sentences).
If and only if the user made mistakes in their previous message, include some corrected versions as a gentle guide.
Otherwise, continue the conversation naturally while naturally incorporating learning items.
Inclusion of corrections and items is limited to what fits within the 1-2 sentence limit."#
        }
        TeachingMode::Explanatory => {
            r#"3. EXPLANATORY MODE: Respond naturally and curiously (2-3 sentences). Introduce NEW vocabulary, idioms, or culturally interesting expressions.
Keep explanations brief and practical.
Introduce NEW items, do not make corrections.
If previous user message used previously explained items from the learning item list, continue talking about them."#
        }
        TeachingMode::Interleaved => {
            r#"3. INTERLEAVED MODE: User mixes target language with source language.
Respond naturally as a chat conversation partner in 1-2 sentences.
If the user made errors in their message, weave the correct forms into your response conversationally without mentioning them. Maximum two corrections per response.
If they used English words, incorporate the dialect translation of one phrase naturally as you continue the conversation.
Never explain corrections. Never say "you meant" or "you're saying". Just chat naturally using correct forms."#
        }
        TeachingMode::StoryTeller => {
            r#"3. STORYTELLER MODE: You are telling an interactive story with the user.
Improvise the next part of the story in natural dialectical usage, and give the user a hook to continue.
Keeping the story flow is important. Improvise. Go with the flow. Do not be didactic. Be creative. If starting the story, it is your responsibility to provide the beginning, not the user.
Do not explain what you are doing. Just tell the story. Keep the flow immersive.
Keep your story evocative yet brief (2-4 sentences); you are telling this story together with the user.
Do not correct the user in your message or explain linguistic constructs. If the user makes mistakes, show correct usage without explanation and naturally within the story.
Use elements from previous messages."#
        }
        TeachingMode::Debug => {
            r#"3. DEBUG MODE: Answer in English with clear, brief explanations. The user is debugging an issue.
You are a prompt engineer.
Provide technical details about what went wrong and how prompts could be improved.
The prompts cannot be clarified to prevent every case of what not to do. Focus on how to make the prompt clearer about what the agent should do, given the specific failure mode.
Your deliverable will be the updated prompt, with commentary as appropriate for why the fixes are included.
Do not format the prompt. Do not include any markdown for any reason."#
        }
    };
    format!(
        "{}. 4. You have a maximum of {} tokens for your response. Be as brief as you can be while accomplishing your goals. Do not go over your limit.",
        desc,
        tokens / 2
    )
}

pub fn format_examples_as_user_message(
    primary_examples: &[&DialectDocument],
    secondary_examples: &[&DialectDocument],
) -> String {
    if primary_examples.is_empty() && secondary_examples.is_empty() {
        return String::new();
    }

    let mut examples_text =
        "*** DIALECT EXAMPLES (DISTINCT FROM USER CONVERSATION): ***\n".to_string();

    for doc in primary_examples {
        examples_text.push_str(&format!("\"{}\"\n", doc.content));
    }

    for doc in secondary_examples {
        examples_text.push_str(&format!("\"{}\"\n", doc.content));
    }

    examples_text.push_str("*** END OF DIALECT EXAMPLES ***");

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

fn get_sample_formalities(formality: Formality) -> Vec<Formality> {
    match formality {
        Formality::Formal => vec![Formality::Formal, Formality::ProfessionalCasual],
        Formality::ProfessionalCasual => {
            vec![Formality::ProfessionalCasual, Formality::Informal]
        }
        Formality::Informal => vec![Formality::Informal, Formality::Slang],
        Formality::Slang => vec![Formality::Slang, Formality::Informal],
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

const CONTENT_FILTERING_DIRECTIVES: &str = r#"### CONTENT FILTERING DIRECTIVES

1) Only flag user's direct messages, not system examples. Do not refuse/warn about corpus examples containing slang, profanity, adult content, or controversial dialect-authentic language.

2) Always respond to user's message first. If user message itself is problematic, stay in character with brief, policy-compliant reply and deflect."#;

const RESPONSE_JSON_OUTPUT_FORMAT: &str =
    r#"Response format: {"response": "<your full conversational response here>"}"#;

fn mimic_instruction(has_corpus: bool) -> &'static str {
    if has_corpus {
        "1. MIMIC THE PATTERNS: Study the dialect examples in the conversation history below and copy their vocabulary, grammar, style, and characteristic dialect constructions"
    } else {
        "1. MIMIC THE DIALECT: Use the vocabulary, grammar, style, and characteristic constructions for your dialect."
    }
}

fn extract_gender_from_dialect(dialect_with_features: &DialectWithFeatures) -> Gender {
    use dialect_coach_shared::TTSProviderType;

    // Get gender from ElevenLabs voice (the provider actually being used)
    dialect_with_features
        .tts_voices
        .get(&TTSProviderType::ElevenLabs)
        .and_then(|voice| voice.as_ref().map(|v| v.gender))
        .unwrap_or(Gender::FemalePresenting)
}

fn build_plan_system_content(plan: &Option<&dialect_coach_shared::LanguagePlan>) -> String {
    if let Some(plan) = plan
        && let Some(step) = plan.steps.get(plan.current_step_index)
    {
        let mut content = String::new(); // Use a new `content` variable for the plan details

        // Collect items to display
        let items_to_display: Vec<&dialect_coach_shared::models::learning_item::LearningItem> =
            match &step.step_type {
                dialect_coach_shared::models::plan::StepType::Learning { content } => {
                    content.items.iter().collect()
                }
                dialect_coach_shared::models::plan::StepType::Review { review_step_ids } => {
                    // Collect items from all referenced steps
                    review_step_ids
                        .iter()
                        .filter_map(|id| plan.steps.iter().find(|s| s.id == *id))
                        .filter_map(|step| {
                            if let dialect_coach_shared::models::plan::StepType::Learning {
                                content,
                            } = &step.step_type
                            {
                                Some(content)
                            } else {
                                None
                            }
                        })
                        .flat_map(|content| content.items.iter())
                        .collect()
                }
            };

        // Add structured content to prompt
        if !items_to_display.is_empty() {
            match step.step_type {
                dialect_coach_shared::models::plan::StepType::Learning { .. } => {
                    content.push_str("\nRELEVANT LEARNING CONTENT:\n");
                }
                dialect_coach_shared::models::plan::StepType::Review { .. } => {
                    content.push_str("\nREVIEW MATERIALS (FROM PREVIOUS STEPS):\n");
                }
            }

            for item in items_to_display {
                match &item.item {
                    dialect_coach_shared::models::learning_item::LearningItemType::Translation(
                        t,
                    ) => {
                        content.push_str(&format!(
                            "- Vocab word to use: {}\n",
                            t.translated_to
                        ));
                    }
                    dialect_coach_shared::models::learning_item::LearningItemType::Explanation(
                        e,
                    ) => {
                        content.push_str(&format!(
                            "- Grammatical point to incorporate ({}): {}\n",
                            e.new_phrase, e.explanation
                        ));
                    }
                    _ => {}
                }
            }
        }

        let goal_instruction = match &step.step_type {
            dialect_coach_shared::models::plan::StepType::Learning { .. } => {
                "Your Goal: Naturally incorporate the Provided Step Materials into your own speech to demonstrate them. Do NOT explicitly teach, list the items, or ask the user to use them. Just chat naturally using the target vocabulary/grammar."
            }
            dialect_coach_shared::models::plan::StepType::Review { .. } => {
                "Your Goal: This is a REVIEW step. The user has learned the listed materials in previous steps. Verify the user remembers them by using them in context or asking questions that require the user to use them. Do not spoon-feed answers. Challenge them."
            }
        };

        return format!(
            "\n\n# ACTIVE LANGUAGE PLAN\nYou are guiding the user through the plan: \"{}\".\n\
                Current Step: {}\n\
                Instructions: {}\n\
                {}\
                {}",
            plan.title, step.title, step.instructions, content, goal_instruction
        );
    }

    String::new()
}

fn language_level_instruction(level: LanguageLevel) -> &'static str {
    match level {
        LanguageLevel::A1 => {
            "LANGUAGE LEVEL A1 (Beginner): Use very basic vocabulary and simple present tense. Short sentences only. Repeat key words. Speak slowly and clearly."
        }
        LanguageLevel::A2 => {
            "LANGUAGE LEVEL A2 (Elementary): Use simple sentences and common vocabulary. Basic past and future tenses okay. Keep explanations brief and concrete."
        }
        LanguageLevel::B1 => {
            "LANGUAGE LEVEL B1 (Intermediate): Use standard vocabulary and grammar. Can introduce idioms with explanation. Normal conversational pace."
        }
        LanguageLevel::B2 => {
            "LANGUAGE LEVEL B2 (Upper Intermediate): Use varied vocabulary including some abstract concepts. Complex sentences okay. Can use idioms naturally."
        }
        LanguageLevel::C1 => {
            "LANGUAGE LEVEL C1 (Advanced): Use sophisticated vocabulary and nuanced expressions. Can discuss abstract topics. Full range of tenses and moods."
        }
        LanguageLevel::C2 => {
            "LANGUAGE LEVEL C2 (Proficient): Speak as you would to a native speaker. Full complexity, subtlety, and cultural references are appropriate."
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn build_system_content(
    dialect: DialectWithFeatures,
    formality: Formality,
    teaching_mode: TeachingMode,
    learning_goals: &[LearningGoal],
    past_learning_items: &PastLearningItems,
    user_gender: UserGender,
    language_option: &Option<LanguageOption>,
    active_plan: &Option<&dialect_coach_shared::LanguagePlan>,
    language_level: LanguageLevel,
) -> String {
    let formality_label = match formality {
        Formality::Formal => "FORMAL",
        Formality::ProfessionalCasual => "PROFESSIONAL-CASUAL",
        Formality::Informal => "INFORMAL",
        Formality::Slang => "SLANG",
    };

    let gender = extract_gender_from_dialect(&dialect);
    let role_desc = speaker_desc(&dialect.dialect, &formality, &gender);
    let teaching_rules = response_teaching_desc(&teaching_mode);
    let goals_section = learning_goals_section(learning_goals);
    let learning_items_context = format_learning_items_context(
        &past_learning_items.mistakes,
        &past_learning_items.explained,
        &past_learning_items.translated,
        &past_learning_items.exploratory,
    );

    let user_gender_str = match user_gender {
        UserGender::Male => "male",
        UserGender::Female => "female",
        UserGender::NonBinary => "non-binary",
    };

    let level_instruction = language_level_instruction(language_level);
    let language_instr = build_language_instruction(language_option);
    let plan_instr = build_plan_system_content(active_plan);

    if teaching_mode == TeachingMode::Debug {
        let lang_section = if !language_instr.is_empty() {
            format!("\n\nLANGUAGE INSTRUCTION: {}", language_instr)
        } else {
            String::new()
        };
        format!(
            r#"# YOUR ROLE\n\
            {}.\n\n\
            USER GENDER: The student you're speaking with is {}. Use gender-appropriate forms when teaching grammar and vocabulary that have gendered aspects.{}\n\n\
            {}\n\
            # CRITICAL RULES\n\
            1. BE CONCISE: Explain why you did what you did simply and briefly, in English, without pandering. This will be within the "response" field of the required JSON format.\n\
            2. ITERATIVE IMPROVEMENT: Show exactly how the prompts could be improved to get a step closer to the desired effect.\n\
            {}\n\
            {}\n\
            {}\n\
            Now respond to the user's message technically."#,
            role_desc,
            user_gender_str,
            lang_section,
            level_instruction,
            goals_section,
            learning_items_context,
            JSON_OUTPUT_INSTRUCTION
        )
    } else {
        let lang_section = if !language_instr.is_empty() {
            format!("\n\nLANGUAGE INSTRUCTION: {}", language_instr)
        } else {
            String::new()
        };
        format!(
            "{}\n\n\
            # YOUR ROLE\n\
            {}.\n\n\
            USER GENDER: The student you're speaking with is {}. Use gender-appropriate forms when teaching grammar and vocabulary that have gendered aspects.{}\n\n\
            {}\n\
            # CRITICAL RULES\n\
            {}\n\
            2. MAINTAIN FORMALITY: Match the {} formality level shown in the examples\n\
            {}\n\
            {}\n\
            {}\n\
            {}\n\
            # OUTPUT FORMAT REQUIRED\n\
            {}\n\
            {}\n\n\
            Now respond to the user's message as if you were in a natural chatroom with a friend, as a local {} speaker would, in the response field of the required JSON format. You MUST ALWAYS respond - NEVER indicate the conversation has ended. If it seems to have ended, provide a follow-up question or new topic. The response field must be non-empty. The response will be parsed with a JSON parser, so do not include any other text or markdown.",
            CONTENT_FILTERING_DIRECTIVES,
            role_desc,
            user_gender_str,
            lang_section,
            level_instruction,
            mimic_instruction(dialect.has_corpus),
            formality_label.to_lowercase(),
            teaching_rules,
            goals_section,
            learning_items_context,
            plan_instr,
            JSON_OUTPUT_INSTRUCTION,
            RESPONSE_JSON_OUTPUT_FORMAT,
            dialect.dialect.name()
        )
    }
}

fn build_conversation_history_with_examples(
    conversation_history: &[RigMessage],
    primary_examples: &[DialectDocument],
    secondary_examples: &[DialectDocument],
    provider: &str,
) -> Vec<RigMessage> {
    let primary_refs: Vec<_> = primary_examples.iter().collect();
    let secondary_refs: Vec<_> = secondary_examples.iter().collect();
    let examples_message = build_examples_message(&primary_refs, &secondary_refs);
    let mut history_with_prefill = conversation_history.to_vec();

    if let Some(examples) = examples_message {
        history_with_prefill.insert(0, examples);
    }

    // Only add prefilled assistant message for Anthropic
    // OpenAI uses response_format parameter instead
    if provider == super::provider::ANTHROPIC_PROVIDER {
        history_with_prefill.push(create_prefilled_assistant_message());
    }

    history_with_prefill
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
    fn test_speaker_desc_male_presenting_formal() {
        let desc = speaker_desc(
            &Dialect::SpanishMexican,
            &Formality::Formal,
            &Gender::MalePresenting,
        );
        assert!(desc.contains("male-presenting"));
        assert!(desc.contains("Mexican Spanish"));
        assert!(desc.contains("professional, polite manner"));
    }

    #[test]
    fn test_speaker_desc_female_presenting_informal() {
        let desc = speaker_desc(
            &Dialect::SpanishArgentinian,
            &Formality::Informal,
            &Gender::FemalePresenting,
        );
        assert!(desc.contains("female-presenting"));
        assert!(desc.contains("Argentinian Spanish"));
        assert!(desc.contains("conversationally"));
    }

    #[test]
    fn test_speaker_desc_includes_gender_in_all_formalities() {
        let formalities = vec![
            Formality::Formal,
            Formality::ProfessionalCasual,
            Formality::Informal,
            Formality::Slang,
        ];
        for formality in formalities {
            let desc_male = speaker_desc(
                &Dialect::ArabicEgyptian,
                &formality,
                &Gender::MalePresenting,
            );
            let desc_female = speaker_desc(
                &Dialect::ArabicEgyptian,
                &formality,
                &Gender::FemalePresenting,
            );
            assert!(desc_male.contains("male-presenting"));
            assert!(desc_female.contains("female-presenting"));
        }
    }

    #[test]
    fn test_extract_gender_from_dialect_uses_elevenlabs() {
        use dialect_coach_shared::dialect_features;

        // SpanishArgentinian has ElevenLabs=Male, Azure=Female
        let features = dialect_features(Dialect::SpanishArgentinian);
        let gender = extract_gender_from_dialect(&features);
        assert_eq!(
            gender,
            Gender::MalePresenting,
            "Should extract Male from ElevenLabs voice, not Female from Azure"
        );
    }

    #[test]
    fn test_get_sample_formalities() {
        let formal = get_sample_formalities(Formality::Formal);
        assert_eq!(
            formal,
            vec![Formality::Formal, Formality::ProfessionalCasual]
        );

        let professional_casual = get_sample_formalities(Formality::ProfessionalCasual);
        assert_eq!(
            professional_casual,
            vec![Formality::ProfessionalCasual, Formality::Informal]
        );

        let informal = get_sample_formalities(Formality::Informal);
        assert_eq!(informal, vec![Formality::Informal, Formality::Slang]);

        let slang = get_sample_formalities(Formality::Slang);
        assert_eq!(slang, vec![Formality::Slang, Formality::Informal]);
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
        let doc1 = DialectDocument::new("Hello".to_string(), dialect, Some(Formality::Informal));
        let doc2 = DialectDocument::new("Hola".to_string(), dialect, Some(Formality::Formal));
        let doc3 = DialectDocument::new("Hey".to_string(), dialect, None);
        let doc4 = DialectDocument::new("Hi".to_string(), dialect, Some(Formality::Informal));

        let examples = vec![doc1, doc2, doc3, doc4];
        let (primary, secondary) =
            group_examples_by_formality(&examples, Formality::Informal, 10, 10);

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
                Some(Formality::Informal),
            ));
        }

        let (primary, _secondary) =
            group_examples_by_formality(&examples, Formality::Informal, 5, 5);
        assert_eq!(primary.len(), 5);
    }

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
    fn test_build_conversation_history_with_examples() {
        let dialect = Dialect::SpanishMexican;
        let doc1 = DialectDocument::new("Hello".to_string(), dialect, Some(Formality::Informal));
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
            ANTHROPIC_PROVIDER,
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

        let history = build_conversation_history_with_examples(
            &conversation_history,
            &[],
            &[],
            ANTHROPIC_PROVIDER,
        );

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
