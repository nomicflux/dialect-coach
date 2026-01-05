use anyhow::Result;
use dialect_coach_shared::Dialect;
use rig::prelude::*;
use rig::providers::openai::{
    self,
    responses_api::{AdditionalParameters, Reasoning, ReasoningEffort},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Deserialize, Serialize, Debug, Clone)]
pub enum Intent {
    Inquiry,
    Statement,
    Greeting,
    Command,
    Expression,
}

#[derive(JsonSchema, Deserialize, Serialize, Debug, Clone)]
pub enum Emotion {
    Neutral,
    Happy,
    Sad,
    Angry,
    Excited,
    Confused,
}

#[derive(JsonSchema, Deserialize, Serialize, Debug, Clone)]
pub enum Formality {
    Formal,
    ProfessionalCasual,
    Informal,
    Slang,
}

#[derive(JsonSchema, Deserialize, Serialize, Debug, Clone)]
pub struct EnrichedData {
    pub formality: Option<Formality>,
    pub intent: Option<Intent>,
    pub emotion: Option<Emotion>,
    pub topics: Vec<String>,
    pub context_triggers: Vec<String>,
    pub keywords: Vec<String>,
    pub is_safe: bool,
    pub is_target_dialect: bool,
}

pub struct LlmClient {
    client: openai::Client,
}

impl LlmClient {
    pub fn new() -> Result<Self> {
        let client = openai::Client::from_env();
        Ok(Self { client })
    }

    pub async fn enrich_chunk(
        &self,
        text_chunk: &str,
        dialect: Dialect,
    ) -> Result<Option<EnrichedData>> {
        let dialect_name = dialect.name();
        let dialect_id = dialect.id();

        let t_start = std::time::Instant::now();
        println!("  >> [LLM] Preparing request...");

        let prompt = format!(
            r#"Role: You are an expert linguist specializing in {dialect_name} ({dialect_id}).
Task: Analyze the provided text chunk for a RAG retrieval system.

1. Context Triggers: Generate 3 distinct conversational turns in {dialect_name} that would ELICIT this text as a response.
   - STRICTLY use {dialect_name} (with selected Code Switching if common in {dialect_name}).
   - Focus on the conversational flow: What did the OTHER person say immediately before this?

2. Keywords: Extract ALL substantive vocabulary terms in their dictionary form (lemma).
   - Exclude: Articles, particles, conjunctions, prepositions, common adjectives/adverbs (e.g., "very", "too").
   - Include: Nouns (dictionary form), Verbs (dictionary form), uncommon adjectives/adverbs.
   - Include slang terms if present.

3. Filtration (CRITICAL):
   - If the text is ALL-ENGLISH (no target dialect present) and NO Code-switching, mark `is_target_dialect` as false.
   - If the text contains URIs or Banned Subjects (Hate speech/Explicit), mark `is_safe` as false.
   - NOTE: Use the structured output fields `is_safe` and `is_target_dialect`.

4. Metadata:
   - Classify Formality, Intent, Emotion, Topics.

Input Text: "{text_chunk}"
"#,
            dialect_name = dialect_name,
            dialect_id = dialect_id,
        );
        println!(
            "  >> [LLM] Prompt prepared in {:.2}ms",
            t_start.elapsed().as_millis()
        );

        let t_build = std::time::Instant::now();

        // Create AdditionalParameters with low reasoning effort
        let additional_params = AdditionalParameters {
            reasoning: Some(Reasoning::new().with_effort(ReasoningEffort::Minimal)),
            ..Default::default()
        };

        // Use Extractor with additional_params for reasoning effort
        let extractor = self
            .client
            .extractor::<EnrichedData>("gpt-5-nano")
            .additional_params(additional_params.to_json())
            .build();

        println!(
            "  >> [LLM] Extractor built in {:.2}ms",
            t_build.elapsed().as_millis()
        );

        println!(
            "  >> [LLM] Sending request to 'gpt-5-nano' (Prompt len: {} chars)...",
            prompt.len()
        );
        let t_exec = std::time::Instant::now();

        let mut result = match extractor.extract(&prompt).await {
            Ok(data) => {
                println!(
                    "  >> [LLM] Network/Gen Success in {:.2}s. (Safe: {}, Target: {})",
                    t_exec.elapsed().as_secs_f64(),
                    data.is_safe,
                    data.is_target_dialect
                );
                data
            }
            Err(e) => {
                println!(
                    "  >> [LLM] Network/Gen FAILED in {:.2}s: {}",
                    t_exec.elapsed().as_secs_f64(),
                    e
                );
                return Err(e.into());
            }
        };

        // Sort keywords alphabetically for deterministic lookups (Unicode order)
        result.keywords.sort();

        // Filtration Logic
        if !result.is_safe || !result.is_target_dialect {
            return Ok(None);
        }

        Ok(Some(result))
    }
}
