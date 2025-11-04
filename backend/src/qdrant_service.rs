use anyhow::{Context, Result};
use dialect_coach_shared::{Dialect, DialectDocument};
use qdrant_client::Qdrant;
use qdrant_client::QdrantError;
use qdrant_client::qdrant::r#match::MatchValue;
use qdrant_client::qdrant::{
    Condition, CreateFieldIndexCollectionBuilder, FieldType, Filter, SearchPointsBuilder,
};
use rand::seq::SliceRandom;

const COLLECTION_NAME: &str = "dialect_documents";

/// Qdrant service for RAG retrieval
pub struct QdrantService {
    client: Qdrant,
}

impl QdrantService {
    /// Connect to Qdrant Cloud instance
    pub async fn new(url: &str, api_key: &str) -> Result<Self> {
        let client = Qdrant::from_url(url)
            .api_key(api_key)
            .build()
            .context("Failed to connect to Qdrant")?;

        tracing::info!("Connected to Qdrant at {}", url);

        Ok(Self { client })
    }

    /// Get reference to the Qdrant client
    pub fn client(&self) -> &Qdrant {
        &self.client
    }

    /// Create from environment variables
    pub async fn from_env() -> Result<Self> {
        let url = std::env::var("QDRANT_URL").context("QDRANT_URL environment variable not set")?;
        let api_key = std::env::var("QDRANT_API_KEY")
            .context("QDRANT_API_KEY environment variable not set")?;

        Self::new(&url, &api_key).await
    }

    fn is_retryable_error(error: &QdrantError) -> bool {
        match error {
            QdrantError::Io(_) => true,
            QdrantError::Reqwest(_) => true,
            _ => false,
        }
    }

    fn calculate_backoff_delay(attempt: usize) -> tokio::time::Duration {
        tokio::time::Duration::from_secs(2_u64.pow(attempt as u32))
    }

    fn log_retry_attempt(
        attempt: usize,
        max_attempts: usize,
        error: &QdrantError,
        delay: tokio::time::Duration,
    ) {
        tracing::warn!(
            "Qdrant operation failed (attempt {}/{}): {}. Retrying in {:?}...",
            attempt,
            max_attempts,
            error,
            delay
        );
    }

    async fn handle_retry_delay(attempt: usize, max_attempts: usize, error: &QdrantError) {
        let delay = Self::calculate_backoff_delay(attempt);
        Self::log_retry_attempt(attempt, max_attempts, error, delay);
        tokio::time::sleep(delay).await;
    }

    async fn retry_qdrant_operation<F, Fut, T>(operation: F, max_attempts: usize) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, QdrantError>>,
    {
        let mut last_error = None;
        for attempt in 1..=max_attempts {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) if Self::is_retryable_error(&e) && attempt < max_attempts => {
                    Self::handle_retry_delay(attempt, max_attempts, &e).await;
                    last_error = Some(e);
                }
                Err(e) => return Err(anyhow::anyhow!("Qdrant operation failed: {}", e)),
            }
        }
        Err(anyhow::anyhow!(
            "Qdrant operation failed after {} attempts: {}",
            max_attempts,
            last_error.unwrap()
        ))
    }

    /// Search for relevant dialect examples
    pub async fn search_dialect_examples(
        &self,
        query_embedding: &[f32],
        dialect: &Dialect,
        limit: usize,
    ) -> Result<Vec<(DialectDocument, f32)>> {
        let dialect_id = dialect.id().to_string();
        tracing::info!("Searching for {}", dialect_id);
        let filter = Filter::must([Condition::matches(
            "dialect",
            MatchValue::Keyword(dialect_id),
        )]);

        let search_result = Self::retry_qdrant_operation(
            || {
                self.client.search_points(
                    SearchPointsBuilder::new(
                        COLLECTION_NAME,
                        query_embedding.to_owned(),
                        limit as u64,
                    )
                    .filter(filter.clone())
                    .with_payload(true),
                )
            },
            3,
        )
        .await
        .map_err(|e| {
            anyhow::anyhow!(
                "Failed to search Qdrant collection '{}' for dialect {} (limit: {}): {}",
                COLLECTION_NAME,
                dialect.name(),
                limit,
                e
            )
        })?;

        let documents = self.parse_search_results(search_result.result, dialect)?;

        tracing::info!(
            "Found {} dialect examples for {}",
            documents.len(),
            dialect.name()
        );

        Ok(documents)
    }

    /// Get random dialect samples filtered by formality
    pub async fn random_dialect_samples(
        &self,
        dialect: Dialect,
        formality_levels: Vec<dialect_coach_shared::Formality>,
        limit: usize,
    ) -> Result<Vec<DialectDocument>> {
        use qdrant_client::qdrant::ScrollPointsBuilder;

        // Build filter - just dialect for now (formality filtering can be added later)
        let filter = Filter::must([Condition::matches("dialect", dialect.id().to_string())]);

        // Use scroll to get random samples
        let scroll_result = Self::retry_qdrant_operation(
            || {
                self.client.scroll(
                    ScrollPointsBuilder::new(COLLECTION_NAME)
                        .filter(filter.clone())
                        .limit(limit as u32)
                        .with_payload(true),
                )
            },
            3,
        )
        .await
        .map_err(|e| {
            anyhow::anyhow!(
                "Failed to scroll Qdrant collection '{}' for dialect {} (limit: {}): {}",
                COLLECTION_NAME,
                dialect.name(),
                limit,
                e
            )
        })?;

        // Parse retrieved points
        let mut documents = Vec::new();
        for point in scroll_result.result {
            let payload = point.payload;

            let content = payload
                .get("content")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_default();

            let formality = payload
                .get("formality")
                .and_then(|v| v.as_str())
                .and_then(|s| {
                    let s = s.as_ref();
                    match s {
                        "Formal" => Some(dialect_coach_shared::Formality::Formal),
                        "Casual" => Some(dialect_coach_shared::Formality::Casual),
                        "DialectRich" => Some(dialect_coach_shared::Formality::DialectRich),
                        "Slang" => Some(dialect_coach_shared::Formality::Slang),
                        _ => None,
                    }
                });

            // Filter by formality if specified
            if !formality_levels.is_empty()
                && let Some(f) = formality
                && !formality_levels.contains(&f)
            {
                continue;
            }

            documents.push(DialectDocument {
                content,
                dialect,
                formality,
                embedding: Vec::new(),
            });
        }

        // Shuffle to make results actually random
        documents.shuffle(&mut rand::thread_rng());

        tracing::info!(
            "Retrieved {} random dialect samples for {} with formality filters",
            documents.len(),
            dialect.name()
        );

        Ok(documents)
    }

    /// Parse Qdrant search results into DialectDocuments with scores
    fn parse_search_results(
        &self,
        results: Vec<qdrant_client::qdrant::ScoredPoint>,
        dialect: &Dialect,
    ) -> Result<Vec<(DialectDocument, f32)>> {
        let mut documents = Vec::new();

        tracing::info!("Found {} results", results.len());

        for point in results {
            let payload = point.payload;
            let score = point.score;

            // Extract fields from payload
            let content = payload
                .get("content")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_default();

            // Formality is optional
            let formality = payload
                .get("formality")
                .and_then(|v| v.as_str())
                .and_then(|s| {
                    let s = s.as_ref();
                    match s {
                        "Formal" => Some(dialect_coach_shared::Formality::Formal),
                        "Casual" => Some(dialect_coach_shared::Formality::Casual),
                        "DialectRich" => Some(dialect_coach_shared::Formality::DialectRich),
                        "Slang" => Some(dialect_coach_shared::Formality::Slang),
                        _ => None,
                    }
                });

            documents.push((
                DialectDocument {
                    content,
                    dialect: *dialect,
                    formality,
                    embedding: Vec::new(),
                },
                score,
            ));
        }

        Ok(documents)
    }

    /// Get collection info for debugging
    pub async fn get_collection_info(&self) -> Result<()> {
        let collection_info = self
            .client
            .collection_info(COLLECTION_NAME)
            .await
            .context("Failed to get collection info")?;

        if let Some(result) = collection_info.result {
            tracing::info!(
                "Collection '{}' has {:?} points",
                COLLECTION_NAME,
                result.points_count
            );
        }

        Ok(())
    }

    /// Create field index for filtering
    pub async fn create_field_index(&self, field_name: &str) -> Result<()> {
        tracing::info!("Creating field index for '{}'", field_name);

        self.client
            .create_field_index(CreateFieldIndexCollectionBuilder::new(
                COLLECTION_NAME,
                field_name,
                FieldType::Keyword,
            ))
            .await
            .context("Failed to create field index")?;

        tracing::info!("Successfully created field index for '{}'", field_name);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dialect_filter() {
        let dialect = Dialect::SpanishArgentinian;
        let _filter = Filter::must([Condition::matches("dialect", dialect.name().to_string())]);
        // If this compiles, the filter is correctly constructed
    }

    #[tokio::test]
    #[ignore] // Only run with real credentials
    async fn test_connection() {
        // This test requires QDRANT_URL and QDRANT_API_KEY env vars
        if let Ok(service) = QdrantService::from_env().await {
            service.get_collection_info().await.unwrap();
        }
    }
}
