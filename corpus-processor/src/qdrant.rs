use crate::test_seams::VectorUploader;
use anyhow::{Context, Result, anyhow};
use dialect_coach_shared::DialectDocument;
use qdrant_client::qdrant::points_selector::PointsSelectorOneOf;
use qdrant_client::qdrant::points_update_operation::{Operation, SetPayload};
use qdrant_client::qdrant::value::Kind;
use qdrant_client::qdrant::{
    Condition, CreateCollectionBuilder, Distance, Filter, PointId, PointStruct, PointsIdsList,
    PointsSelector, PointsUpdateOperation, ScrollPointsBuilder, UpdateBatchPointsBuilder,
    UpsertPointsBuilder, Value, VectorParamsBuilder,
};
use qdrant_client::{Payload, Qdrant};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

const COLLECTION_NAME: &str = "dialect_documents";

/// Generic Qdrant API response envelope
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct QdrantResponse<T> {
    pub status: String,
    pub time: f64,
    pub result: Option<T>,
}

/// Collection information from Qdrant API
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct CollectionInfo {
    pub status: Option<String>,
    pub points_count: Option<u64>,
    pub vectors_count: Option<u64>,
    pub indexed_vectors_count: Option<u64>,
    pub segments_count: Option<u32>,
}

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

        let client = builder.build().context("Failed to connect to Qdrant")?;

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
        let total_batches = documents.len().div_ceil(batch_size);

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
                    payload.insert("dialect", doc.dialect.id());

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

    /// Get collection info as structured data
    pub async fn get_collection_info_structured(&self) -> Result<QdrantResponse<CollectionInfo>> {
        let collection_info = self
            .client
            .collection_info(COLLECTION_NAME)
            .await
            .context("Failed to get collection info")?;

        // Convert qdrant_client response to our structured format
        let response = QdrantResponse {
            status: "ok".to_string(), // qdrant_client doesn't expose status, but success means ok
            time: collection_info.time,
            result: collection_info.result.map(|result| CollectionInfo {
                status: Some("green".to_string()), // Default assumption for successful response
                points_count: result.points_count,
                vectors_count: result.vectors_count,
                indexed_vectors_count: result.indexed_vectors_count,
                segments_count: Some(result.segments_count as u32),
            }),
        };

        Ok(response)
    }

    /// Get detailed status with counts per dialect
    pub async fn get_detailed_status(&self) -> Result<()> {
        // Get basic collection info first
        let collection_info = match self.client.collection_info(COLLECTION_NAME).await {
            Ok(info) => {
                println!("\n✅ Collection Status:\n Name: {}", COLLECTION_NAME);
                if let Some(result) = info.result {
                    println!(
                        "  Points count: {}\nStatus: Available",
                        result.points_count.unwrap_or(0)
                    );
                } else {
                    println!("  Status: Collection found but no details available");
                }
                true
            }
            Err(e) => {
                println!(
                    "\n❌ Collection Status:\n Error: {}\n  Collection '{}' may not exist or connection failed",
                    e, COLLECTION_NAME
                );
                false
            }
        };

        if collection_info {
            // Try to get a sample of points to show what dialects exist
            println!("\n🔍 Checking for dialect data...");

            match self
                .client
                .scroll(
                    ScrollPointsBuilder::new(COLLECTION_NAME)
                        .limit(20000)
                        .with_payload(true)
                        .with_vectors(false),
                )
                .await
            {
                Ok(result) => {
                    let points = result.result;
                    let mut dialect_samples = HashMap::new();

                    for point in points.iter() {
                        if let Some(qdrant_client::qdrant::Value {
                            kind: Some(qdrant_client::qdrant::value::Kind::StringValue(dialect)),
                        }) = point.payload.get("dialect")
                        {
                            dialect_samples
                                .entry(dialect.clone())
                                .and_modify(|e| *e += 1)
                                .or_insert(1);
                        }
                    }

                    if dialect_samples.is_empty() {
                        println!("  No dialect data found in sample");
                    } else {
                        println!("  Dialects found:");
                        for (dialect, count) in dialect_samples {
                            println!("    - {}: {} points", dialect, count);
                        }
                    }
                }
                Err(e) => {
                    println!("  Could not sample points: {}", e);
                }
            }
        }

        println!(
            "\n💡 Tip: Use 'cargo run -p corpus-processor -- upload --input <file>' to add more dialects"
        );
        Ok(())
    }

    pub async fn update_points(&self, dialect_from: &String, dialect_to: &String) -> Result<()> {
        let filter = Filter::must([Condition::matches("dialect", (*dialect_from).clone())]);
        let limit = 100000;
        let results = self
            .client
            .scroll(
                ScrollPointsBuilder::new(COLLECTION_NAME)
                    .limit(limit)
                    .filter(filter)
                    .with_payload(true),
            )
            .await
            .context("Failed to search Qdrant")?;

        for (idx, point) in results.result.iter().enumerate() {
            let payload = &point.payload;
            let mut new_payload = HashMap::new();
            let content = payload.get("content").ok_or(anyhow!("No content!"))?;
            let new_dialect = Value {
                kind: Some(Kind::StringValue(dialect_to.to_string())),
            };
            new_payload.insert(String::from("content"), (*content).clone());
            new_payload.insert(String::from("dialect"), new_dialect);

            let point_id = match &point.id {
                Some(id) => vec![id.clone()],
                None => Vec::new(),
            };
            let points_selector = PointsSelector {
                points_selector_one_of: Some(PointsSelectorOneOf::Points(PointsIdsList {
                    ids: point_id,
                })),
            };
            let set_payload = PointsUpdateOperation {
                operation: Some(Operation::SetPayload(SetPayload {
                    payload: new_payload.clone(),
                    points_selector: Some(points_selector),
                    shard_key_selector: None,
                    key: None,
                })),
            };

            println!("{}: Payload: {:?}", idx, *payload);
            println!("{}: New payload: {:?}", idx, new_payload.clone());
            let builder = UpdateBatchPointsBuilder::new(COLLECTION_NAME, vec![set_payload]);
            let res = self
                .client
                .update_points_batch(builder.wait(true))
                .await
                .context("Failed to update points");
            match res {
                Ok(r) => println!("Returned result: {:?}", r),
                Err(e) => println!("Returned error: {:?}", e),
            }
        }

        Ok(())
    }
}

// Implement VectorUploader trait for dependency injection
#[async_trait::async_trait]
impl VectorUploader for QdrantService {
    async fn upload_documents(&self, documents: &[DialectDocument]) -> Result<()> {
        self.upload_documents(documents).await
    }

    async fn get_collection_info(&self) -> Result<()> {
        // For the trait, we'll maintain the old printing behavior
        let info = self.get_collection_info_structured().await?;
        println!("\nCollection Info:");
        println!("  Name: {}", COLLECTION_NAME);
        if let Some(result) = &info.result {
            println!("  Points count: {:?}", result.points_count);
            println!("  Vectors count: {:?}", result.vectors_count);
        }
        Ok(())
    }

    async fn get_detailed_status(&self) -> Result<()> {
        self.get_detailed_status().await
    }
}
