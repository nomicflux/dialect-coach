use anyhow::{anyhow, Context, Result};

use qdrant_client::qdrant::points_selector::PointsSelectorOneOf;

use qdrant_client::qdrant::points_update_operation::{Operation, SetPayload};
use qdrant_client::qdrant::value::Kind;
use qdrant_client::qdrant::{
    Condition, CreateCollectionBuilder, DeletePointsBuilder, Distance, Filter, PointId,
    PointStruct, PointsIdsList, PointsSelector, PointsUpdateOperation, ScrollPointsBuilder,
    UpdateBatchPointsBuilder, UpsertPointsBuilder, Value, VectorParamsBuilder, VectorParamsMap,
    VectorsConfig,
};
use qdrant_client::{Payload, Qdrant};
use std::collections::HashMap;
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

    /// Initialize collection with Named Vectors
    pub async fn init_collection(&self, vector_size: u64) -> Result<()> {
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
            println!(
                "Creating collection '{}' with Named Vectors...",
                COLLECTION_NAME
            );

            // Define named vector configurations
            let mut vector_params = HashMap::new();
            vector_params.insert(
                "content".to_string(),
                VectorParamsBuilder::new(vector_size, Distance::Cosine).build(),
            );
            vector_params.insert(
                "context".to_string(),
                VectorParamsBuilder::new(vector_size, Distance::Cosine).build(),
            );
            vector_params.insert(
                "keyword".to_string(),
                VectorParamsBuilder::new(vector_size, Distance::Cosine).build(),
            );

            use qdrant_client::qdrant::vectors_config::Config;

            // ...

            self.client
                .create_collection(
                    CreateCollectionBuilder::new(COLLECTION_NAME).vectors_config(VectorsConfig {
                        config: Some(Config::ParamsMap(VectorParamsMap { map: vector_params })),
                    }),
                )
                .await
                .context("Failed to create collection")?;

            println!("Collection created successfully");
        } else {
            println!("Collection '{}' already exists. Note: Ensure it has 'content', 'context', and 'keyword' vectors.", COLLECTION_NAME);
        }

        Ok(())
    }

    /// Upload enriched documents to Qdrant
    /// Takes a tuple of (Document, Context Embedding, Keyword Embedding, EnrichedData)
    pub async fn upload_enriched_documents(
        &self,
        documents: &[crate::processor::EnrichedCorpusTuple],
    ) -> Result<()> {
        if documents.is_empty() {
            return Ok(());
        }

        let vector_size = documents[0].0.embedding.len() as u64;
        self.init_collection(vector_size).await?;

        let batch_size = 100;
        let total_batches = documents.len().div_ceil(batch_size);

        for (batch_idx, chunk) in documents.chunks(batch_size).enumerate() {
            let points: Vec<PointStruct> = chunk
                .iter()
                .map(|item| {
                    let (doc, context_emb, keyword_emb, data) = item;
                    let namespace = Uuid::NAMESPACE_OID;
                    let name = format!("{}:{}", doc.dialect.name(), doc.content);
                    let uuid = Uuid::new_v5(&namespace, name.as_bytes());
                    let point_id = PointId::from(uuid.to_string());

                    // Payload
                    let mut payload = Payload::new();
                    payload.insert("content", doc.content.clone());
                    payload.insert("dialect", doc.dialect.id());

                    // Add Enriched Metadata
                    if let Some(f) = &data.formality {
                        payload.insert("formality", format!("{:?}", f));
                    }
                    if let Some(i) = &data.intent {
                        payload.insert("intent", format!("{:?}", i));
                    }
                    if let Some(e) = &data.emotion {
                        payload.insert("emotion", format!("{:?}", e));
                    }
                    payload.insert("topics", data.topics.clone());
                    payload.insert("context_triggers", data.context_triggers.clone());
                    payload.insert("keywords", data.keywords.clone());

                    // Named Vectors
                    let mut vectors = HashMap::new();
                    vectors.insert("content".to_string(), doc.embedding.clone());

                    if let Some(emb) = context_emb {
                        vectors.insert("context".to_string(), emb.clone());
                    }
                    if let Some(emb) = keyword_emb {
                        vectors.insert("keyword".to_string(), emb.clone());
                    }

                    PointStruct::new(point_id, vectors, payload)
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

    pub async fn delete_points(&self, dialect: &String) -> Result<()> {
        let filter = Filter::must([Condition::matches("dialect", (*dialect).clone())]);
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

        let point_ids = results
            .result
            .iter()
            .map(|point| point.id.clone().unwrap())
            .collect();
        let point_selector = PointsSelectorOneOf::Points(PointsIdsList { ids: point_ids });
        let builder = DeletePointsBuilder::new(COLLECTION_NAME).points(point_selector);
        let res = self
            .client
            .delete_points(builder.wait(true))
            .await
            .context("Failed to delete points");
        match res {
            Ok(r) => println!("Returned result: {:?}", r),
            Err(e) => println!("Returned error: {:?}", e),
        }

        Ok(())
    }
}
