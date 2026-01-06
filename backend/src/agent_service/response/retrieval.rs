use anyhow::Result;
use dialect_coach_shared::{DialectDocument, DialectWithFeatures, Formality};
use rig::completion::Message as RigMessage;

use super::super::util::get_message_text;
use super::ResponseContext;
use super::examples::{deduplicate_excluding, merge_search_results};
use crate::rag_config::RAGConfig;

impl ResponseContext {
    fn build_context_string(conversation_history: &[RigMessage]) -> String {
        conversation_history
            .iter()
            .take(3)
            .map(get_message_text)
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub(super) async fn collect_examples(
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

        // 1. Generate content embedding
        let content_embedding = self.embeddings.embed_text(user_message)?;

        // 2. Generate context embedding from conversation history
        let context_text = Self::build_context_string(conversation_history);
        let context_embedding = self.embeddings.embed_text(&context_text)?;

        // 3. Extract keywords and generate embeddings
        let keywords = self
            .keyword_extractor
            .extract(user_message, dialect.dialect)
            .await
            .unwrap_or_default();
        let keyword_embeddings = if !keywords.is_empty() {
            self.embeddings.embed_batch(keywords)?
        } else {
            Vec::new()
        };

        // 4. Primary search (with formality filter) - parallel
        let (content_with_form, context_with_form, keyword_with_form) = tokio::join!(
            self.qdrant.search_by_content(
                &content_embedding,
                &dialect.dialect,
                Some(&formality),
                rag_config.content_with_formality_limit
            ),
            self.qdrant.search_by_context(
                &context_embedding,
                &dialect.dialect,
                Some(&formality),
                rag_config.context_with_formality_limit
            ),
            self.qdrant.search_by_keywords(
                &keyword_embeddings,
                &dialect.dialect,
                Some(&formality),
                rag_config.keyword_with_formality_limit
            ),
        );

        // 5. Merge primary results (with formality)
        let primary = merge_search_results(vec![
            content_with_form?,
            context_with_form?,
            keyword_with_form?,
        ]);

        // 6. Secondary search (no formality filter) - parallel
        let (content_without_form, context_without_form, keyword_without_form) = tokio::join!(
            self.qdrant.search_by_content(
                &content_embedding,
                &dialect.dialect,
                None,
                rag_config.content_without_formality_limit
            ),
            self.qdrant.search_by_context(
                &context_embedding,
                &dialect.dialect,
                None,
                rag_config.context_without_formality_limit
            ),
            self.qdrant.search_by_keywords(
                &keyword_embeddings,
                &dialect.dialect,
                None,
                rag_config.keyword_without_formality_limit
            ),
        );

        // 7. Merge secondary, excluding primary content
        let secondary = deduplicate_excluding(
            vec![
                content_without_form?,
                context_without_form?,
                keyword_without_form?,
            ],
            &primary,
        );

        Ok((primary, secondary))
    }
}
