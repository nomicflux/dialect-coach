use anyhow::{Context, Result};
use dialect_coach_shared::{Dialect, DialectDocument, DialectWithFeatures, Formality};
use rig::completion::Message as RigMessage;

use super::examples::{deduplicate_examples, get_sample_formalities, group_examples_by_formality};
use super::super::util::get_message_text;
use super::ResponseContext;
use crate::rag_config::RAGConfig;

impl ResponseContext {
    pub(super) fn retrieve_user_msg_embeddings(&self, user_message: &str) -> Result<Vec<f32>> {
        self.embeddings
            .embed_text(user_message)
            .context("Failed to generate content embedding")
    }

    pub(super) fn retrieve_history_embeddings(
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

    pub(super) fn retrieve_embeddings(
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

    pub(super) async fn retrieve_example(
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

    pub(super) async fn retrieve_examples(
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

    pub(super) async fn retrieve_random_samples(
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

    async fn retrieve_all_rag_examples(
        &self,
        user_message: &str,
        conversation_history: &[RigMessage],
        dialect: &Dialect,
        config: &RAGConfig,
    ) -> Result<Vec<DialectDocument>> {
        let history_text = conversation_history
            .iter()
            .map(get_message_text)
            .collect::<Vec<String>>();
        let embeddings = self.retrieve_embeddings(user_message, &history_text)?;
        self.retrieve_examples(dialect, embeddings, config.num_conversation_documents)
            .await
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
        let examples = self
            .retrieve_all_rag_examples(
                user_message,
                conversation_history,
                &dialect.dialect,
                rag_config,
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
}
