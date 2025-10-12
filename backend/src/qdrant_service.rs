use anyhow::{Context, Result};
use dialect_coach_shared::{Dialect, DialectDocument};
use qdrant_client::Qdrant;
use qdrant_client::qdrant::{Condition, Filter, SearchPointsBuilder, CreateFieldIndexCollectionBuilder, FieldType};

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

    /// Create from environment variables
    pub async fn from_env() -> Result<Self> {
        let url = std::env::var("QDRANT_URL")
            .context("QDRANT_URL environment variable not set")?;
        let api_key = std::env::var("QDRANT_API_KEY")
            .context("QDRANT_API_KEY environment variable not set")?;

        Self::new(&url, &api_key).await
    }

    /// Search for relevant dialect examples
    pub async fn search_dialect_examples(
        &self,
        query_embedding: Vec<f32>,
        dialect: Dialect,
        limit: usize,
    ) -> Result<Vec<DialectDocument>> {
        // Build filter for dialect
        let filter = Filter::must([Condition::matches("dialect", dialect.id().to_string())]);

        let search_result = self
            .client
            .search_points(
                SearchPointsBuilder::new(COLLECTION_NAME, query_embedding, limit as u64)
                    .filter(filter)
                    .with_payload(true),
            )
            .await
            .context("Failed to search Qdrant")?;

        let mut documents = Vec::new();

        for point in search_result.result {
            let payload = point.payload;

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
                        "Slang" => Some(dialect_coach_shared::Formality::Slang),
                        _ => None,
                    }
                });

            documents.push(DialectDocument {
                content,
                dialect, // Use the dialect we're filtering for
                formality,
                embedding: Vec::new(), // Don't need embeddings in results
            });
        }

        tracing::info!(
            "Found {} dialect examples for {}",
            documents.len(),
            dialect.name()
        );

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
            .create_field_index(
                CreateFieldIndexCollectionBuilder::new(
                    COLLECTION_NAME,
                    field_name,
                    FieldType::Keyword,
                ),
            )
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
