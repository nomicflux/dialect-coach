use crate::admin::types::{DialectCount, QdrantStats};
use crate::qdrant_service::QdrantService;
use anyhow::Result;
use dialect_coach_shared::Dialect;
use qdrant_client::qdrant::{Condition, CountPointsBuilder, Filter};

const COLLECTION_NAME: &str = "dialect_documents";

pub async fn get_qdrant_stats(service: &QdrantService) -> Result<QdrantStats> {
    let collection_info = get_collection_info(service).await?;
    let dialect_counts = get_dialect_counts(service).await?;
    let memory_bytes = None; // Will be implemented with cluster metrics

    Ok(QdrantStats {
        points_count: collection_info.0,
        vectors_count: collection_info.1,
        segments_count: collection_info.2,
        dialect_counts,
        memory_bytes,
    })
}

async fn get_collection_info(service: &QdrantService) -> Result<(u64, u64, u32)> {
    let client = service.client();
    let info = client.collection_info(COLLECTION_NAME).await?;
    let result = info
        .result
        .ok_or_else(|| anyhow::anyhow!("No collection info"))?;

    Ok((
        result.points_count.unwrap_or(0),
        result.vectors_count.unwrap_or(0),
        result.segments_count as u32,
    ))
}

async fn get_dialect_counts(service: &QdrantService) -> Result<Vec<DialectCount>> {
    let client = service.client();
    let mut counts = Vec::new();

    for dialect in Dialect::all() {
        let filter = Filter::must([Condition::matches("dialect", dialect.id().to_string())]);
        let count_result = client
            .count(CountPointsBuilder::new(COLLECTION_NAME).filter(filter))
            .await?;

        if let Some(result) = count_result.result {
            counts.push(DialectCount {
                dialect: dialect.id().to_string(),
                count: result.count,
            });
        }
    }

    Ok(counts)
}
