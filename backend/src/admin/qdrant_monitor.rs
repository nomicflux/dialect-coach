use crate::admin::types::{DialectCount, QdrantStats};
use crate::qdrant_service::QdrantService;
use anyhow::Result;
use dialect_coach_shared::Dialect;
use qdrant_client::qdrant::{Condition, CountPointsBuilder, Filter};
use std::collections::HashMap;

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

pub fn parse_prometheus_metrics(text: &str) -> HashMap<String, f64> {
    let mut metrics = HashMap::new();

    for line in text.lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() == 2
            && let Ok(value) = parts[1].parse::<f64>()
        {
            metrics.insert(parts[0].to_string(), value);
        }
    }

    metrics
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_prometheus_metrics() {
        let sample = r#"
# HELP process_resident_memory_bytes Resident memory size
# TYPE process_resident_memory_bytes gauge
process_resident_memory_bytes 134217728
collections_total 1
rest_responses_total 156
"#;

        let metrics = parse_prometheus_metrics(sample);
        assert_eq!(
            metrics.get("process_resident_memory_bytes"),
            Some(&134217728.0)
        );
        assert_eq!(metrics.get("collections_total"), Some(&1.0));
        assert_eq!(metrics.get("rest_responses_total"), Some(&156.0));
    }
}
