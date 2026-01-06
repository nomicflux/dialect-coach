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
pub struct ExtractedKeywords {
    pub keywords: Vec<String>,
}

pub struct KeywordExtractor {
    client: openai::Client,
}

impl KeywordExtractor {
    pub fn new() -> Result<Self> {
        let client = openai::Client::from_env();
        Ok(Self { client })
    }

    /// Extract keywords from text using GPT-5-nano
    /// Returns dictionary forms of: nouns, verbs, slang, proper nouns,
    /// unusual adverbs/adjectives - sorted by unicode order
    pub async fn extract(&self, text: &str, dialect: Dialect) -> Result<Vec<String>> {
        let dialect_name = dialect.name();

        let prompt = format!(
            r#"Role: Extract searchable keywords from user input for {dialect_name} dialect retrieval.

Task: Extract ALL substantive vocabulary terms in their dictionary form (lemma).

Include:
- Nouns (dictionary/singular form)
- Verbs (dictionary/infinitive form)
- Slang terms
- Proper nouns
- Unusual adjectives/adverbs

Exclude:
- Articles, particles, conjunctions, prepositions
- Common adjectives/adverbs (very, too, really, etc.)

Input: "{text}"

Return keywords sorted alphabetically."#,
            dialect_name = dialect_name,
            text = text
        );

        let additional_params = AdditionalParameters {
            reasoning: Some(Reasoning::new().with_effort(ReasoningEffort::Minimal)),
            ..Default::default()
        };

        let extractor = self
            .client
            .extractor::<ExtractedKeywords>("gpt-5-nano")
            .additional_params(additional_params.to_json())
            .build();

        let mut result = extractor.extract(&prompt).await?;

        result.keywords.sort();

        Ok(result.keywords)
    }
}
