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

    /// Generate embedding for a single text
    pub fn embed_one(&self, text: String) -> Result<Vec<f32>> {
        let mut embeddings = self.embed_batch(vec![text])?;

        embeddings
            .pop()
            .context("Expected embedding but got empty result")
    }

    /// Get the dimension of embeddings produced by this model
    pub fn dimension(&self) -> usize {
        // MultilingualE5Base produces 768-dimensional embeddings
        768
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embed_single_text() {
        let service = EmbeddingService::new().unwrap();
        let embedding = service.embed_one("Hello world".to_string()).unwrap();

        assert_eq!(embedding.len(), service.dimension());
        assert!(embedding.iter().any(|&x| x != 0.0));
    }

    #[test]
    fn test_embed_batch() {
        let service = EmbeddingService::new().unwrap();
        let texts = vec!["First text".to_string(), "Second text".to_string()];
        let embeddings = service.embed_batch(texts).unwrap();

        assert_eq!(embeddings.len(), 2);
        for embedding in embeddings {
            assert_eq!(embedding.len(), service.dimension());
        }
    }
}
