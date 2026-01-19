use anyhow::{Context, Result};
use bzip2::read::BzDecoder;
use fastembed::{TextEmbedding, TokenizerFiles, UserDefinedEmbeddingModel};
use std::io;

/// Embedding service using Fastembed
pub struct EmbeddingService {
    model: TextEmbedding,
}

/// Decompress model.onnx.bz2 and return bytes
fn load_model_onnx() -> Result<Vec<u8>> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let bz2_path = format!("{}/vendor/model.onnx.bz2", manifest_dir);

    let bz2_file =
        std::fs::File::open(&bz2_path).context(format!("Model file not found at {}", bz2_path))?;

    let mut decoder = BzDecoder::new(bz2_file);
    let mut onnx_bytes = Vec::new();
    io::copy(&mut decoder, &mut onnx_bytes).context("Failed to decompress model")?;

    Ok(onnx_bytes)
}

/// Load tokenizer files from vendor directory
fn load_tokenizer_files() -> Result<TokenizerFiles> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let vendor_dir = format!("{}/vendor", manifest_dir);

    Ok(TokenizerFiles {
        tokenizer_file: std::fs::read(format!("{}/tokenizer.json", vendor_dir))?,
        config_file: std::fs::read(format!("{}/config.json", vendor_dir))?,
        special_tokens_map_file: std::fs::read(format!("{}/special_tokens_map.json", vendor_dir))?,
        tokenizer_config_file: std::fs::read(format!("{}/tokenizer_config.json", vendor_dir))?,
    })
}

impl EmbeddingService {
    /// Initialize embedding service with local bundled model
    pub fn new() -> Result<Self> {
        tracing::info!("Loading embedding model from bundled files...");

        let onnx_bytes = load_model_onnx().context("Failed to load model.onnx")?;

        let tokenizer_files = load_tokenizer_files().context("Failed to load tokenizer files")?;

        let user_model = UserDefinedEmbeddingModel::new(onnx_bytes, tokenizer_files);

        let model = TextEmbedding::try_new_from_user_defined(user_model, Default::default())
            .context("Failed to initialize embedding model")?;

        tracing::info!("Embedding model loaded successfully");

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
