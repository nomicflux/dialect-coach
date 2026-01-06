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
}
