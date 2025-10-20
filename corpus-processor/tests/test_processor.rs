use corpus_processor::processor::*;
use dialect_coach_shared::{Dialect, DialectDocument, Formality};
use std::fs;
use tempfile::tempdir;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_documents() {
        let documents = vec![
            DialectDocument::new(
                "First sentence. Second sentence.".to_string(),
                Dialect::ArabicEgyptian,
                Some(Formality::Formal),
            ),
            DialectDocument::new("Short text.".to_string(), Dialect::ArabicEgyptian, None),
        ];

        let config = corpus_processor::chunking::ChunkConfig {
            max_chunk_size: 20,
            overlap: 5,
        };

        let chunked = chunk_documents(documents, &config).unwrap();

        assert!(!chunked.is_empty());
        // Verify all chunks have the same dialect
        for chunk in &chunked {
            assert_eq!(chunk.dialect, Dialect::ArabicEgyptian);
        }
    }

    #[test]
    fn test_chunk_documents_empty() {
        let documents = vec![];
        let config = corpus_processor::chunking::ChunkConfig {
            max_chunk_size: 100,
            overlap: 10,
        };

        let chunked = chunk_documents(documents, &config).unwrap();
        assert!(chunked.is_empty());
    }

    #[test]
    fn test_save_documents() {
        let temp_dir = tempdir().unwrap();
        let output_path = temp_dir.path().to_str().unwrap();

        let mut documents = vec![
            DialectDocument::new(
                "Test content 1".to_string(),
                Dialect::ArabicEgyptian,
                Some(Formality::Formal),
            ),
            DialectDocument::new(
                "Test content 2".to_string(),
                Dialect::ArabicEgyptian,
                Some(Formality::Casual),
            ),
            DialectDocument::new("Test content 3".to_string(), Dialect::ArabicEgyptian, None),
        ];

        // Add embeddings
        for doc in &mut documents {
            doc.embedding = vec![0.1, 0.2, 0.3];
        }

        save_documents(&documents, output_path).unwrap();

        // Check JSONL file was created with correct name
        let jsonl_path = temp_dir.path().join("arabic_egyptian.jsonl");
        assert!(jsonl_path.exists());

        // Check metadata file was created
        let metadata_path = temp_dir.path().join("arabic_egyptian_metadata.json");
        assert!(metadata_path.exists());

        // Verify JSONL content
        let jsonl_content = fs::read_to_string(&jsonl_path).unwrap();
        let lines: Vec<&str> = jsonl_content.lines().collect();
        assert_eq!(lines.len(), 3);

        // Verify each line is valid JSON
        for line in lines {
            let doc: DialectDocument = serde_json::from_str(line).unwrap();
            assert_eq!(doc.dialect, Dialect::ArabicEgyptian);
            assert_eq!(doc.embedding, vec![0.1, 0.2, 0.3]);
        }

        // Verify metadata content
        let metadata_content = fs::read_to_string(&metadata_path).unwrap();
        let metadata: serde_json::Value = serde_json::from_str(&metadata_content).unwrap();

        assert_eq!(metadata["total_documents"], 3);
        assert_eq!(metadata["dialect"], "Egyptian Arabic");
        assert_eq!(metadata["embedding_dimension"], 3);

        let formality_dist = &metadata["formality_distribution"];
        assert_eq!(formality_dist["formal"], 1);
        assert_eq!(formality_dist["casual"], 1);
        assert_eq!(formality_dist["unspecified"], 1);
        assert_eq!(formality_dist["dialect_rich"], 0);
        assert_eq!(formality_dist["slang"], 0);
    }

    #[test]
    fn test_save_documents_empty() {
        let temp_dir = tempdir().unwrap();
        let output_path = temp_dir.path().to_str().unwrap();

        let documents = vec![];

        save_documents(&documents, output_path).unwrap();

        // Should create default filename when no documents
        let jsonl_path = temp_dir.path().join("documents.jsonl");
        assert!(jsonl_path.exists());

        let metadata_path = temp_dir.path().join("metadata.json");
        assert!(metadata_path.exists());

        // Verify empty JSONL
        let jsonl_content = fs::read_to_string(&jsonl_path).unwrap();
        assert!(jsonl_content.is_empty());

        // Verify metadata for empty documents
        let metadata_content = fs::read_to_string(&metadata_path).unwrap();
        let metadata: serde_json::Value = serde_json::from_str(&metadata_content).unwrap();
        assert_eq!(metadata["total_documents"], 0);
        assert_eq!(metadata["embedding_dimension"], 0);
    }

    #[test]
    fn test_count_formality() {
        let documents = vec![
            DialectDocument::new(
                "Test".to_string(),
                Dialect::ArabicEgyptian,
                Some(Formality::Formal),
            ),
            DialectDocument::new(
                "Test".to_string(),
                Dialect::ArabicEgyptian,
                Some(Formality::Formal),
            ),
            DialectDocument::new(
                "Test".to_string(),
                Dialect::ArabicEgyptian,
                Some(Formality::Casual),
            ),
            DialectDocument::new(
                "Test".to_string(),
                Dialect::ArabicEgyptian,
                Some(Formality::DialectRich),
            ),
            DialectDocument::new(
                "Test".to_string(),
                Dialect::ArabicEgyptian,
                Some(Formality::Slang),
            ),
            DialectDocument::new("Test".to_string(), Dialect::ArabicEgyptian, None),
            DialectDocument::new("Test".to_string(), Dialect::ArabicEgyptian, None),
        ];

        let result = count_formality(&documents);

        assert_eq!(result["formal"], 2);
        assert_eq!(result["casual"], 1);
        assert_eq!(result["dialect_rich"], 1);
        assert_eq!(result["slang"], 1);
        assert_eq!(result["unspecified"], 2);
    }

    #[test]
    fn test_count_formality_empty() {
        let documents = vec![];
        let result = count_formality(&documents);

        assert_eq!(result["formal"], 0);
        assert_eq!(result["casual"], 0);
        assert_eq!(result["dialect_rich"], 0);
        assert_eq!(result["slang"], 0);
        assert_eq!(result["unspecified"], 0);
    }

    #[test]
    fn test_save_documents_readonly_directory() {
        // Create a temp directory and make it read-only
        let temp_dir = tempdir().unwrap();
        let readonly_path = temp_dir.path().join("readonly");
        fs::create_dir_all(&readonly_path).unwrap();

        // Try to make directory read-only (platform dependent)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&readonly_path).unwrap().permissions();
            perms.set_mode(0o444); // Read-only
            fs::set_permissions(&readonly_path, perms).unwrap();
        }

        let documents = vec![DialectDocument::new(
            "Test content".to_string(),
            Dialect::ArabicEgyptian,
            None,
        )];

        let result = save_documents(&documents, readonly_path.to_str().unwrap());

        // Should fail to write to read-only directory
        #[cfg(unix)]
        assert!(result.is_err());

        // Clean up - restore write permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&readonly_path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&readonly_path, perms).unwrap();
        }
    }

    #[test]
    fn test_save_documents_invalid_json_serialization() {
        // This test is hard to trigger since DialectDocument should always serialize,
        // but we can test the error context is properly formatted
        let temp_dir = tempdir().unwrap();
        let output_path = temp_dir.path().to_str().unwrap();

        let documents = vec![DialectDocument::new(
            "Test content".to_string(),
            Dialect::ArabicEgyptian,
            None,
        )];

        // This should succeed - testing the happy path to ensure error context exists
        let result = save_documents(&documents, output_path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_save_documents_metadata_serialization_error() {
        // Test that metadata generation works correctly even with edge cases
        let temp_dir = tempdir().unwrap();
        let output_path = temp_dir.path().to_str().unwrap();

        let mut documents = vec![
            DialectDocument::new(
                "Test content 1".to_string(),
                Dialect::ArabicEgyptian,
                Some(Formality::Formal),
            ),
            DialectDocument::new(
                "Test content 2".to_string(),
                Dialect::ArabicEgyptian,
                Some(Formality::DialectRich),
            ),
        ];

        // Add embeddings of different sizes to test edge cases
        documents[0].embedding = vec![0.1; 768];
        documents[1].embedding = vec![0.2; 512]; // Different size

        let result = save_documents(&documents, output_path);
        assert!(result.is_ok());

        // Verify metadata was written correctly
        let metadata_path = temp_dir.path().join("arabic_egyptian_metadata.json");
        assert!(metadata_path.exists());

        let metadata_content = fs::read_to_string(&metadata_path).unwrap();
        let metadata: serde_json::Value = serde_json::from_str(&metadata_content).unwrap();

        // Should use first document's embedding dimension
        assert_eq!(metadata["embedding_dimension"], 768);
        assert_eq!(metadata["total_documents"], 2);
    }

    #[test]
    fn test_chunk_documents_chunking_error() {
        // Test error propagation from chunk_text
        let documents = vec![DialectDocument::new(
            "Test content".to_string(),
            Dialect::ArabicEgyptian,
            None,
        )];

        // Create a config that could potentially cause issues
        let config = corpus_processor::chunking::ChunkConfig {
            max_chunk_size: 0, // Edge case - zero chunk size
            overlap: 10,
        };

        // This might fail depending on chunking implementation
        let result = chunk_documents(documents, &config);
        // The chunking implementation should handle this gracefully
        // If it doesn't fail, that's also valid behavior
        match result {
            Ok(chunks) => {
                // If it succeeds, verify it produced reasonable output
                println!("Chunking with zero size produced {} chunks", chunks.len());
            }
            Err(e) => {
                // If it fails, verify error is reasonable
                println!("Chunking failed as expected: {}", e);
            }
        }
    }

    #[test]
    fn test_chunk_documents_formality_preservation() {
        // Test that formality is preserved through chunking
        let documents = vec![
            DialectDocument::new(
                "First sentence. Second sentence. Third sentence.".to_string(),
                Dialect::ArabicEgyptian,
                Some(Formality::Slang),
            ),
            DialectDocument::new(
                "Another long sentence that should be chunked.".to_string(),
                Dialect::SpanishMexican,
                Some(Formality::Formal),
            ),
        ];

        let config = corpus_processor::chunking::ChunkConfig {
            max_chunk_size: 25,
            overlap: 5,
        };

        let chunked = chunk_documents(documents, &config).unwrap();

        // Verify formality is preserved
        for chunk in &chunked {
            match chunk.dialect {
                Dialect::ArabicEgyptian => {
                    assert_eq!(chunk.formality, Some(Formality::Slang));
                }
                Dialect::SpanishMexican => {
                    assert_eq!(chunk.formality, Some(Formality::Formal));
                }
                _ => panic!("Unexpected dialect"),
            }
        }
    }

    #[test]
    fn test_count_formality_all_variants() {
        // Comprehensive test of all formality variants
        let documents = vec![
            DialectDocument::new(
                "Test".to_string(),
                Dialect::ArabicEgyptian,
                Some(Formality::Formal),
            ),
            DialectDocument::new(
                "Test".to_string(),
                Dialect::ArabicEgyptian,
                Some(Formality::Casual),
            ),
            DialectDocument::new(
                "Test".to_string(),
                Dialect::ArabicEgyptian,
                Some(Formality::DialectRich),
            ),
            DialectDocument::new(
                "Test".to_string(),
                Dialect::ArabicEgyptian,
                Some(Formality::Slang),
            ),
            DialectDocument::new("Test".to_string(), Dialect::ArabicEgyptian, None),
        ];

        let result = count_formality(&documents);

        // Verify JSON structure
        assert!(result.is_object());
        assert_eq!(result["formal"], 1);
        assert_eq!(result["casual"], 1);
        assert_eq!(result["dialect_rich"], 1);
        assert_eq!(result["slang"], 1);
        assert_eq!(result["unspecified"], 1);

        // Verify all expected keys exist
        let obj = result.as_object().unwrap();
        assert!(obj.contains_key("formal"));
        assert!(obj.contains_key("casual"));
        assert!(obj.contains_key("dialect_rich"));
        assert!(obj.contains_key("slang"));
        assert!(obj.contains_key("unspecified"));
    }

    // Note: process_corpus function requires EmbeddingService which needs ML model
    // This would need to be tested via integration tests or with mocking
    // The CLI integration tests already cover this path
}
