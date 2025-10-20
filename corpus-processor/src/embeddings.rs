use crate::test_seams::EmbeddingProvider;
use anyhow::{Context, Result};
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

/// Wrapper for Fastembed embedding model
pub struct EmbeddingService {
    model: TextEmbedding,
}

impl EmbeddingService {
    /// Initialize the embedding service with a multilingual model
    pub fn new() -> Result<Self> {
        println!("Loading multilingual embedding model...");

        let model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::MultilingualE5Base).with_show_download_progress(true),
        )
        .context("Failed to initialize embedding model")?;

        println!("Embedding model loaded successfully");

        Ok(Self { model })
    }

    /// Generate embeddings for a batch of texts
    pub fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        let embeddings = self
            .model
            .embed(texts, None)
            .context("Failed to generate embeddings")?;

        Ok(embeddings)
    }
}

// Implement EmbeddingProvider trait for dependency injection
impl EmbeddingProvider for EmbeddingService {
    fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        self.embed_batch(texts)
    }

    fn dimension(&self) -> usize {
        // MultilingualE5Base produces 768-dimensional embeddings
        768
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embed_batch() {
        let service = EmbeddingService::new().unwrap();
        let texts = vec!["First text".to_string(), "Second text".to_string()];
        let embeddings = service.embed_batch(texts).unwrap();

        assert_eq!(embeddings.len(), 2);
        for embedding in embeddings {
            assert_eq!(embedding.len(), 768); // MultilingualE5Base dimension
        }
    }

    #[test]
    fn test_dimension() {
        let service = EmbeddingService::new().unwrap();
        assert_eq!(service.dimension(), 768);
    }
}
