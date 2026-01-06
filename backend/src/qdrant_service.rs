use anyhow::{Context, Result};
use dialect_coach_shared::{Dialect, DialectDocument};
use qdrant_client::Qdrant;
use qdrant_client::QdrantError;
use qdrant_client::qdrant::r#match::MatchValue;
use qdrant_client::qdrant::{
    Condition, CreateFieldIndexCollectionBuilder, FieldType, Filter, QueryPointsBuilder,
};

const COLLECTION_NAME: &str = "dialect_documents_v2";

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
        matches!(error, QdrantError::Io(_) | QdrantError::Reqwest(_))
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

    fn formality_to_filter_string(formality: &dialect_coach_shared::Formality) -> String {
        match formality {
            dialect_coach_shared::Formality::Formal => "Formal",
            dialect_coach_shared::Formality::ProfessionalCasual => "ProfessionalCasual",
            dialect_coach_shared::Formality::Informal => "Informal",
            dialect_coach_shared::Formality::Slang => "Slang",
        }
        .to_string()
    }

    fn build_dialect_filter(
        dialect: &Dialect,
        formality: Option<&dialect_coach_shared::Formality>,
    ) -> Filter {
        let mut conditions = vec![Condition::matches(
            "dialect",
            MatchValue::Keyword(dialect.id().to_string()),
        )];

        if let Some(f) = formality {
            conditions.push(Condition::matches(
                "formality",
                MatchValue::Keyword(Self::formality_to_filter_string(f)),
            ));
        }

        Filter::must(conditions)
    }

    async fn search_named_vector(
        &self,
        vector_name: &str,
        embedding: &[f32],
        filter: Filter,
        limit: usize,
    ) -> Result<Vec<(DialectDocument, f32)>, QdrantError> {
        let search_result = self
            .client
            .query(
                QueryPointsBuilder::new(COLLECTION_NAME)
                    .query(embedding.to_owned())
                    .using(vector_name)
                    .filter(filter)
                    .limit(limit as u64)
                    .with_payload(true),
            )
            .await?;

        Ok(search_result
            .result
            .into_iter()
            .map(|point| {
                let payload = point.payload;
                let score = point.score;

                let content = payload
                    .get("content")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_default();

                let formality = payload
                    .get("formality")
                    .and_then(|v| v.as_str())
                    .and_then(|s| match s.as_ref() {
                        "Formal" => Some(dialect_coach_shared::Formality::Formal),
                        "ProfessionalCasual" => {
                            Some(dialect_coach_shared::Formality::ProfessionalCasual)
                        }
                        "Informal" => Some(dialect_coach_shared::Formality::Informal),
                        "Slang" => Some(dialect_coach_shared::Formality::Slang),
                        _ => None,
                    });

                let dialect_str = payload
                    .get("dialect")
                    .and_then(|v| v.as_str())
                    .map_or("", |v| v);

                let dialect = Dialect::from_id(dialect_str).unwrap_or(Dialect::SpanishMexican);

                (
                    DialectDocument {
                        content,
                        dialect,
                        formality,
                        embedding: Vec::new(),
                    },
                    score,
                )
            })
            .collect())
    }

    /// Search by content vector
    pub async fn search_by_content(
        &self,
        embedding: &[f32],
        dialect: &Dialect,
        formality: Option<&dialect_coach_shared::Formality>,
        limit: usize,
    ) -> Result<Vec<(DialectDocument, f32)>> {
        let filter = Self::build_dialect_filter(dialect, formality);
        Self::retry_qdrant_operation(
            || self.search_named_vector("content", embedding, filter.clone(), limit),
            3,
        )
        .await
        .map_err(|e| {
            anyhow::anyhow!(
                "Failed to search content vector for dialect {} (limit: {}): {}",
                dialect.name(),
                limit,
                e
            )
        })
    }

    /// Search by context vector
    pub async fn search_by_context(
        &self,
        embedding: &[f32],
        dialect: &Dialect,
        formality: Option<&dialect_coach_shared::Formality>,
        limit: usize,
    ) -> Result<Vec<(DialectDocument, f32)>> {
        let filter = Self::build_dialect_filter(dialect, formality);
        Self::retry_qdrant_operation(
            || self.search_named_vector("context", embedding, filter.clone(), limit),
            3,
        )
        .await
        .map_err(|e| {
            anyhow::anyhow!(
                "Failed to search context vector for dialect {} (limit: {}): {}",
                dialect.name(),
                limit,
                e
            )
        })
    }

    /// Search by keyword vector (multiple embeddings, returns combined results)
    pub async fn search_by_keywords(
        &self,
        keyword_embeddings: &[Vec<f32>],
        dialect: &Dialect,
        formality: Option<&dialect_coach_shared::Formality>,
        limit_per_keyword: usize,
    ) -> Result<Vec<(DialectDocument, f32)>> {
        let filter = Self::build_dialect_filter(dialect, formality);
        let mut all_results = Vec::new();

        for embedding in keyword_embeddings {
            let results = Self::retry_qdrant_operation(
                || {
                    self.search_named_vector("keyword", embedding, filter.clone(), limit_per_keyword)
                },
                3,
            )
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to search keyword vector for dialect {} (limit: {}): {}",
                    dialect.name(),
                    limit_per_keyword,
                    e
                )
            })?;

            all_results.extend(results);
        }

        Ok(all_results)
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
