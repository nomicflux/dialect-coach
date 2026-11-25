use anyhow::Result;
use dialect_coach_shared::Dialect;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::provider::CompletionAgent;

#[derive(JsonSchema, Deserialize, Serialize, Debug, Clone)]
pub struct ExtractedKeywords {
    pub keywords: Vec<String>,
}

pub struct KeywordExtractor {
    agent: Arc<dyn CompletionAgent>,
}

impl KeywordExtractor {
    pub fn new(agent: Arc<dyn CompletionAgent>) -> Self {
        Self { agent }
    }

    /// Extract keywords from text using the configured keyword model.
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

        let mut result = self.agent.extract_keywords(&prompt).await?;

        result.keywords.sort();

        Ok(result.keywords)
    }
}
