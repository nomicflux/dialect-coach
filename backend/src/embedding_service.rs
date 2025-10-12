use anyhow::{Context, Result};
use fastembed::{TextEmbedding, InitOptions, EmbeddingModel};

/// Embedding service using Fastembed
pub struct EmbeddingService {
    model: TextEmbedding,
}

impl EmbeddingService {
    /// Initialize embedding service with MultilingualE5Base model
    pub fn new() -> Result<Self> {
        tracing::info!("Initializing Fastembed model (MultilingualE5Base)...");

        let model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::MultilingualE5Base)
                .with_show_download_progress(true),
        )
        .context("Failed to initialize Fastembed model")?;

        tracing::info!("Fastembed model initialized successfully");

        Ok(Self { model })
    }

    /// Generate embedding for a single text
    pub fn embed_text(&self, text: &str) -> Result<Vec<f32>> {
        let embeddings = self
            .model
            .embed(vec![text.to_string()], None)
            .context("Failed to generate embedding")?;

        embeddings
            .into_iter()
            .next()
            .context("No embedding generated")
    }

    /// Generate embeddings for multiple texts
    pub fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        self.model
            .embed(texts, None)
            .context("Failed to generate batch embeddings")
    }

    /// Get the dimensionality of embeddings
    pub fn dimension(&self) -> usize {
        768 // MultilingualE5Base produces 768-dimensional embeddings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_dimension() {
        let service = EmbeddingService::new().unwrap();
        assert_eq!(service.dimension(), 768);
    }

    #[test]
    fn test_embed_text() {
        let service = EmbeddingService::new().unwrap();
        let embedding = service.embed_text("Hola, ¿cómo estás?").unwrap();

        assert_eq!(embedding.len(), 768);
        // Check that embedding is not all zeros
        assert!(embedding.iter().any(|&x| x != 0.0));
    }

    #[test]
    fn test_embed_batch() {
        let service = EmbeddingService::new().unwrap();
        let texts = vec![
            "Hola".to_string(),
            "Buenos días".to_string(),
            "¿Qué tal?".to_string(),
        ];

        let embeddings = service.embed_batch(texts).unwrap();

        assert_eq!(embeddings.len(), 3);
        for embedding in embeddings {
            assert_eq!(embedding.len(), 768);
            assert!(embedding.iter().any(|&x| x != 0.0));
        }
    }
}
