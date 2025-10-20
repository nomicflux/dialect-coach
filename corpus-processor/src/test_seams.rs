use anyhow::Result;
use dialect_coach_shared::DialectDocument;

/// Trait for embedding providers - allows injection of test implementations
pub trait EmbeddingProvider {
    /// Generate embeddings for a batch of texts
    fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>>;

    /// Get the dimension of embeddings produced by this provider
    fn dimension(&self) -> usize;
}

/// Trait for vector store uploaders - allows injection of test implementations
#[async_trait::async_trait]
pub trait VectorUploader {
    /// Upload documents to the vector store
    async fn upload_documents(&self, documents: &[DialectDocument]) -> Result<()>;

    /// Get collection information
    async fn get_collection_info(&self) -> Result<()>;

    /// Get detailed status
    async fn get_detailed_status(&self) -> Result<()>;
}

/// Path resolver with policy enforcement - allows testing of directory rules
pub struct PathResolver;

impl PathResolver {
    /// Resolve and validate input path according to directory policy
    pub fn resolve_input_path(input: Option<&str>) -> Result<String> {
        match input {
            Some(path) => {
                Self::validate_input_path(path)?;
                Ok(path.to_string())
            }
            None => {
                // Default to corpus-data when no input specified
                Ok("./corpus-data".to_string())
            }
        }
    }

    /// Validate input path against directory policy
    fn validate_input_path(path: &str) -> Result<()> {
        let path_lower = path.to_lowercase();

        // Reject corpus-downloads with migration guidance
        if path.contains("corpus-downloads") {
            anyhow::bail!(
                "Input path '{}' uses legacy corpus-downloads directory.\n\
                Please use 'corpus-data' instead. Run sync script to migrate:\n\
                tools/sync_corpus_downloads_to_data.sh",
                path
            );
        }

        // Reject ANY training terminology
        let training_terms = ["train", "training", "model_training", "train_data"];
        for term in &training_terms {
            if path_lower.contains(term) {
                anyhow::bail!(
                    "Input path '{}' contains training terminology ('{}').\n\
                    This tool is for RAG document preparation ONLY, not model training.",
                    path,
                    term
                );
            }
        }

        // Suggest hyphen version for underscore version
        if path.contains("corpus_data") {
            anyhow::bail!(
                "Input path '{}' uses underscore 'corpus_data'.\n\
                Please use hyphen version 'corpus-data' instead.",
                path
            );
        }

        Ok(())
    }

    /// Resolve and validate output path - enforces structured directory pattern
    pub fn resolve_output_path(
        output: Option<&str>,
        language: &str,
        dialect: &str,
    ) -> Result<String> {
        match output {
            Some(path) => {
                Self::validate_output_path(path)?;
                Ok(path.to_string())
            }
            None => {
                // Default to corpus-data/{language}/{dialect}/processed/
                Ok(format!("./corpus-data/{}/{}/processed", language, dialect))
            }
        }
    }

    /// Validate output path against directory policy  
    fn validate_output_path(path: &str) -> Result<()> {
        let path_lower = path.to_lowercase();

        // Reject ANY training terminology in output paths
        let training_terms = ["train", "training", "model_training"];
        for term in &training_terms {
            if path_lower.contains(term) {
                anyhow::bail!(
                    "Output path '{}' contains training terminology ('{}').\n\
                    This tool is for RAG document preparation ONLY, not model training.",
                    path,
                    term
                );
            }
        }

        Ok(())
    }
}

#[cfg(test)]
pub mod test_support {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    /// Deterministic embedding provider for tests
    pub struct MockEmbeddingProvider {
        dimension: usize,
        responses: Arc<Mutex<HashMap<String, Vec<f32>>>>,
    }

    impl MockEmbeddingProvider {
        pub fn new(dimension: usize) -> Self {
            Self {
                dimension,
                responses: Arc::new(Mutex::new(HashMap::new())),
            }
        }

        /// Set expected response for a specific text
        pub fn set_response(&self, text: &str, embedding: Vec<f32>) {
            let mut responses = self.responses.lock().unwrap();
            responses.insert(text.to_string(), embedding);
        }

        /// Set default response for any text not explicitly set
        pub fn set_default_response(&self, embedding: Vec<f32>) {
            let mut responses = self.responses.lock().unwrap();
            responses.insert("__default__".to_string(), embedding);
        }
    }

    impl EmbeddingProvider for MockEmbeddingProvider {
        fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
            let responses = self.responses.lock().unwrap();
            let mut embeddings = Vec::new();

            for text in texts {
                let embedding = responses
                    .get(&text)
                    .or_else(|| responses.get("__default__"))
                    .cloned()
                    .unwrap_or_else(|| {
                        // Generate deterministic embedding based on text hash
                        let hash = std::collections::hash_map::DefaultHasher::new();
                        use std::hash::{Hash, Hasher};
                        text.hash(&mut hash.clone());
                        let seed = hash.finish() as f32 / u64::MAX as f32;
                        (0..self.dimension)
                            .map(|i| seed + (i as f32 * 0.001))
                            .collect()
                    });

                embeddings.push(embedding);
            }

            Ok(embeddings)
        }

        fn dimension(&self) -> usize {
            self.dimension
        }
    }

    /// Mock vector uploader for tests
    pub struct MockVectorUploader {
        uploaded_documents: Arc<Mutex<Vec<DialectDocument>>>,
        should_fail: Arc<Mutex<bool>>,
    }

    impl MockVectorUploader {
        pub fn new() -> Self {
            Self {
                uploaded_documents: Arc::new(Mutex::new(Vec::new())),
                should_fail: Arc::new(Mutex::new(false)),
            }
        }

        /// Get documents that were uploaded
        pub fn get_uploaded_documents(&self) -> Vec<DialectDocument> {
            self.uploaded_documents.lock().unwrap().clone()
        }

        /// Set whether upload should fail
        pub fn set_should_fail(&self, should_fail: bool) {
            *self.should_fail.lock().unwrap() = should_fail;
        }
    }

    #[async_trait::async_trait]
    impl VectorUploader for MockVectorUploader {
        async fn upload_documents(&self, documents: &[DialectDocument]) -> Result<()> {
            if *self.should_fail.lock().unwrap() {
                anyhow::bail!("Mock upload failure");
            }

            let mut uploaded = self.uploaded_documents.lock().unwrap();
            uploaded.extend_from_slice(documents);
            Ok(())
        }

        async fn get_collection_info(&self) -> Result<()> {
            println!("Mock collection info");
            Ok(())
        }

        async fn get_detailed_status(&self) -> Result<()> {
            println!("Mock detailed status");
            Ok(())
        }
    }

    /// Temporary directory builder for tests
    pub struct TempDirBuilder;

    impl TempDirBuilder {
        /// Create a temporary directory with corpus-data structure
        pub fn create_corpus_structure() -> Result<tempfile::TempDir> {
            let temp_dir = tempfile::tempdir()?;
            let corpus_path = temp_dir.path().join("corpus-data");
            std::fs::create_dir_all(&corpus_path)?;
            Ok(temp_dir)
        }

        /// Create temporary directory with test dialect data
        pub fn create_with_dialect_data(
            language: &str,
            dialect: &str,
            files: &[(&str, &str)], // (filename, content)
        ) -> Result<tempfile::TempDir> {
            let temp_dir = Self::create_corpus_structure()?;
            let dialect_path = temp_dir
                .path()
                .join("corpus-data")
                .join(language)
                .join(dialect);
            std::fs::create_dir_all(&dialect_path)?;

            for (filename, content) in files {
                std::fs::write(dialect_path.join(filename), content)?;
            }

            Ok(temp_dir)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_input_path() {
        let resolved = PathResolver::resolve_input_path(None).unwrap();
        assert_eq!(resolved, "./corpus-data");
    }

    #[test]
    fn test_reject_corpus_downloads() {
        let result = PathResolver::resolve_input_path(Some("./corpus-downloads"));
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("corpus-downloads"));
        assert!(error_msg.contains("migrate"));
    }

    #[test]
    fn test_reject_training_terminology() {
        let training_paths = [
            "./data/train",
            "./training_data",
            "./model_training",
            "./corpus-data/train/subset",
        ];

        for path in &training_paths {
            let result = PathResolver::resolve_input_path(Some(path));
            assert!(result.is_err(), "Should reject path: {}", path);
            let error = result.unwrap_err().to_string();
            assert!(
                error.contains("training"),
                "Error should mention training: {}",
                error
            );
        }
    }

    #[test]
    fn test_suggest_hyphen_version() {
        let result = PathResolver::resolve_input_path(Some("./corpus_data"));
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("hyphen"));
    }

    #[test]
    fn test_valid_input_paths() {
        let valid_paths = [
            "./corpus-data",
            "./corpus-data/arabic/egyptian",
            "/absolute/path/to/corpus-data",
        ];

        for path in &valid_paths {
            let result = PathResolver::resolve_input_path(Some(path));
            assert!(result.is_ok(), "Should accept path: {}", path);
            assert_eq!(result.unwrap(), *path);
        }
    }

    #[test]
    fn test_default_output_path() {
        let resolved = PathResolver::resolve_output_path(None, "arabic", "egyptian").unwrap();
        assert_eq!(resolved, "./corpus-data/arabic/egyptian/processed");
    }

    #[test]
    fn test_reject_training_in_output() {
        let result =
            PathResolver::resolve_output_path(Some("./output/train"), "arabic", "egyptian");
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("training"));
    }
}
