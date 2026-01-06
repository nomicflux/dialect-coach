use anyhow::Result;
use dialect_coach_shared::Dialect;

use crate::embedding_service::EmbeddingService;
use crate::qdrant_service::QdrantService;

/// Compute cosine similarity between two embeddings
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    (dot / (norm_a * norm_b)) as f64
}

/// Compute Euclidean distance squared between two embeddings
pub fn euclidean_distance_squared(a: &[f32], b: &[f32]) -> f64 {
    (a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f32>()) as f64
}

/// Compute cosine distance squared between two embeddings
pub fn cosine_distance_squared(a: &[f32], b: &[f32]) -> f64 {
    (1.0 - cosine_similarity(a, b)).powi(2)
}

/// Compute mean squared error for two metrics: cosine distance and L2 distance
pub async fn compute_corpus_similarity(
    response: &str,
    dialect: Dialect,
    qdrant: &QdrantService,
    embeddings: &EmbeddingService,
    limit: usize,
) -> Result<(f64, f64)> {
    let response_embedding = embeddings.embed_text(response)?;
    let samples_with_scores = qdrant
        .search_by_content(&response_embedding, &dialect, None, limit)
        .await?;
    let samples: Vec<_> = samples_with_scores
        .into_iter()
        .map(|(doc, _)| doc)
        .collect();
    compute_all_mse_metrics(&response_embedding, &samples, embeddings)
}

fn compute_all_mse_metrics(
    response_embedding: &[f32],
    samples: &[dialect_coach_shared::DialectDocument],
    embeddings: &EmbeddingService,
) -> Result<(f64, f64)> {
    let mut cosine_mse_values = Vec::new();
    let mut l2_mse_values = Vec::new();

    for doc in samples {
        let sample_embedding = embeddings.embed_text(&doc.content)?;
        cosine_mse_values.push(cosine_distance_squared(
            response_embedding,
            &sample_embedding,
        ));
        l2_mse_values.push(euclidean_distance_squared(
            response_embedding,
            &sample_embedding,
        ));
    }

    Ok((mean(&cosine_mse_values), mean(&l2_mse_values)))
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let similarity = cosine_similarity(&a, &b);
        assert!((similarity - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let similarity = cosine_similarity(&a, &b);
        assert!(similarity.abs() < 1e-6);
    }

    #[test]
    fn test_mean_empty() {
        assert_eq!(mean(&[]), 0.0);
    }

    #[test]
    fn test_mean_values() {
        let values = vec![0.5, 0.7, 0.9];
        let result = mean(&values);
        assert!((result - 0.7).abs() < 1e-6);
    }

    #[test]
    fn test_euclidean_distance_squared_identical() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let distance = euclidean_distance_squared(&a, &b);
        assert!(distance.abs() < 1e-6);
    }

    #[test]
    fn test_euclidean_distance_squared_different() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let distance = euclidean_distance_squared(&a, &b);
        assert!((distance - 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_distance_squared_identical() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let distance = cosine_distance_squared(&a, &b);
        assert!(distance.abs() < 1e-6);
    }

    #[test]
    fn test_cosine_distance_squared_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let distance = cosine_distance_squared(&a, &b);
        assert!((distance - 1.0).abs() < 1e-6);
    }
}
