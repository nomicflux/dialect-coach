use anyhow::{Context, Result};
use dialect_coach_shared::DialectDocument;
use qdrant_client::{Qdrant, Payload};
use qdrant_client::qdrant::{
    CreateCollectionBuilder, Distance, VectorParamsBuilder, PointStruct, UpsertPointsBuilder, PointId,
};
use uuid::Uuid;

const COLLECTION_NAME: &str = "dialect_documents";

/// Qdrant client wrapper for uploading dialect documents
pub struct QdrantService {
    client: Qdrant,
}

impl QdrantService {
    /// Connect to Qdrant instance
    pub async fn new(url: &str) -> Result<Self> {
        let mut builder = Qdrant::from_url(url);

        // Check for API key in environment
        if let Ok(api_key) = std::env::var("QDRANT_API_KEY") {
            builder = builder.api_key(api_key);
        }

        let client = builder
            .build()
            .context("Failed to connect to Qdrant")?;

        Ok(Self { client })
    }

    /// Connect to Qdrant instance with explicit API key
    pub async fn new_with_api_key(url: &str, api_key: &str) -> Result<Self> {
        let client = Qdrant::from_url(url)
            .api_key(api_key)
            .build()
            .context("Failed to connect to Qdrant")?;

        Ok(Self { client })
    }

    /// Initialize collection if it doesn't exist
    pub async fn init_collection(&self, vector_size: u64) -> Result<()> {
        // Check if collection exists
        let collections = self
            .client
            .list_collections()
            .await
            .context("Failed to list collections")?;

        let exists = collections
            .collections
            .iter()
            .any(|c| c.name == COLLECTION_NAME);

        if !exists {
            println!("Creating collection '{}'...", COLLECTION_NAME);

            self.client
                .create_collection(
                    CreateCollectionBuilder::new(COLLECTION_NAME)
                        .vectors_config(VectorParamsBuilder::new(vector_size, Distance::Cosine)),
                )
                .await
                .context("Failed to create collection")?;

            println!("Collection created successfully");
        } else {
            println!("Collection '{}' already exists", COLLECTION_NAME);
        }

        Ok(())
    }

    /// Upload documents to Qdrant in batches
    pub async fn upload_documents(&self, documents: &[DialectDocument]) -> Result<()> {
        if documents.is_empty() {
            return Ok(());
        }

        // Verify all documents have embeddings
        if documents.iter().any(|doc| doc.embedding.is_empty()) {
            anyhow::bail!("All documents must have embeddings before uploading");
        }

        let vector_size = documents[0].embedding.len() as u64;
        self.init_collection(vector_size).await?;

        let batch_size = 100;
        let total_batches = (documents.len() + batch_size - 1) / batch_size;

        for (batch_idx, chunk) in documents.chunks(batch_size).enumerate() {
            let points: Vec<PointStruct> = chunk
                .iter()
                .map(|doc| {
                    // Generate a stable UUID based on content and dialect
                    // This ensures the same content always gets the same UUID
                    let namespace = Uuid::NAMESPACE_OID;
                    let name = format!("{}:{}", doc.dialect.name(), doc.content);
                    let uuid = Uuid::new_v5(&namespace, name.as_bytes());
                    let point_id = PointId::from(uuid.to_string());

                    // Create payload with document metadata
                    let mut payload = Payload::new();
                    payload.insert("content", doc.content.clone());
                    payload.insert("dialect", doc.dialect.name());

                    if let Some(formality) = &doc.formality {
                        payload.insert("formality", format!("{:?}", formality));
                    }

                    PointStruct::new(point_id, doc.embedding.clone(), payload)
                })
                .collect();

            self.client
                .upsert_points(UpsertPointsBuilder::new(COLLECTION_NAME, points))
                .await
                .context(format!("Failed to upload batch {}", batch_idx + 1))?;

            println!(
                "  Uploaded batch {}/{} ({} documents)",
                batch_idx + 1,
                total_batches,
                (batch_idx + 1) * batch_size.min(documents.len() - batch_idx * batch_size)
            );
        }

        Ok(())
    }

    /// Get collection info
    pub async fn get_collection_info(&self) -> Result<()> {
        let collection_info = self
            .client
            .collection_info(COLLECTION_NAME)
            .await
            .context("Failed to get collection info")?;

        println!("\nCollection Info:");
        println!("  Name: {}", COLLECTION_NAME);
        println!("  Points count: {:?}", collection_info.result.map(|r| r.points_count));

        Ok(())
    }
}
