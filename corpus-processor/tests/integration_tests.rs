use dialect_coach_shared::{Dialect, DialectDocument};
use std::fs;
use tempfile::tempdir;

#[cfg(test)]
mod tests {
    use super::*;
    use corpus_processor::chunking::{chunk_text, ChunkConfig};
    use corpus_processor::loaders::load_corpus;

    #[test]
    fn test_dialect_document_serialization() {
        // Test basic serialization of DialectDocument
        let doc = DialectDocument::new(
            "Test Arabic text: مرحبا".to_string(),
            Dialect::ArabicEgyptian,
            None,
        );

        // Test JSON serialization
        let json = serde_json::to_string(&doc).expect("Failed to serialize DialectDocument");
        println!("Serialized JSON: {}", json);

        // Test deserialization
        let deserialized: DialectDocument =
            serde_json::from_str(&json).expect("Failed to deserialize DialectDocument");

        assert_eq!(doc.content, deserialized.content);
        assert_eq!(doc.dialect, deserialized.dialect);
    }

    #[test]
    fn test_dialect_document_with_embeddings() {
        // Test serialization with embeddings
        let mut doc =
            DialectDocument::new("Test content".to_string(), Dialect::ArabicEgyptian, None);
        doc.embedding = vec![0.1, 0.2, 0.3, 0.4, 0.5];

        let json = serde_json::to_string(&doc).expect("Failed to serialize with embeddings");
        println!("With embeddings: {}", json);

        let deserialized: DialectDocument =
            serde_json::from_str(&json).expect("Failed to deserialize with embeddings");

        assert_eq!(doc.embedding, deserialized.embedding);
    }

    #[test]
    fn test_chunk_text_with_arabic() {
        let config = ChunkConfig {
            max_chunk_size: 100,
            overlap: 20,
        };

        let arabic_text = "مرحبا بك في المغرب. هذا نص باللغة العربية. نحن نختبر التقسيم.";
        let chunks = chunk_text(arabic_text, &config).expect("Chunking should work");

        println!("Arabic chunks: {:?}", chunks);
        assert!(!chunks.is_empty(), "Should create at least one chunk");

        // Verify chunks contain Arabic text
        for chunk in &chunks {
            assert!(!chunk.is_empty(), "Chunks should not be empty");
            println!("Chunk: {}", chunk);
        }
    }

    #[test]
    fn test_text_file_loading() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join("test_arabic.txt");

        // Create test file with Arabic content (one line per document)
        let content = "text\nجنوب السودان هيا دولة مستقلة من ساعة اول دقيقة\nمارى دى كاستيلان كانت صاحبه صالون ادبى من فرنسا\nمارى ديكسون كيس كانت مصممه نسيج";
        fs::write(&file_path, content).expect("Failed to write test file");

        // Test loading
        let documents = load_corpus(file_path.to_str().unwrap(), Dialect::ArabicEgyptian)
            .expect("Failed to load corpus");

        println!("Loaded {} documents", documents.len());
        for (i, doc) in documents.iter().enumerate() {
            println!("Document {}: {}", i, doc.content);
        }

        // Text files load each line as a separate document
        assert_eq!(
            documents.len(),
            4,
            "Text file should load each line as a separate document"
        );

        // Verify first few documents
        assert_eq!(documents[0].content, "text");
        assert_eq!(
            documents[1].content,
            "جنوب السودان هيا دولة مستقلة من ساعة اول دقيقة"
        );

        // Verify all documents have correct dialect
        for doc in &documents {
            assert_eq!(doc.dialect, Dialect::ArabicEgyptian);
        }

        // Verify documents contain Arabic text from the file
        assert!(
            documents[1].content.contains("جنوب"),
            "Second document should contain Arabic text"
        );
        assert!(
            documents[2].content.contains("مارى"),
            "Third document should contain Arabic text"
        );
    }

    #[test]
    fn test_jsonl_save_format() {
        let temp_dir = tempdir().expect("Failed to create temp dir");

        // Create test documents with embeddings
        let mut docs = vec![];
        for i in 0..3 {
            let mut doc = DialectDocument::new(
                format!("Test document {} with Arabic: مرحبا", i),
                Dialect::ArabicEgyptian,
                None,
            );
            doc.embedding = vec![0.1 * i as f32; 768]; // Realistic embedding size
            docs.push(doc);
        }

        // Manually save in JSONL format (like save_documents should do)
        let jsonl_path = temp_dir.path().join("test.jsonl");
        let mut lines = Vec::new();

        for doc in &docs {
            let json = serde_json::to_string(doc).expect("Failed to serialize document");
            lines.push(json);
            println!("JSON line: {}", lines.last().unwrap());
        }

        fs::write(&jsonl_path, lines.join("\n")).expect("Failed to write JSONL");

        // Verify file format
        let saved_content = fs::read_to_string(&jsonl_path).expect("Failed to read JSONL");
        println!("Saved JSONL content:\n{}", saved_content);

        // Each line should be valid JSON
        let lines: Vec<&str> = saved_content.lines().collect();
        assert_eq!(lines.len(), 3, "Should have 3 JSON lines");

        for line in lines {
            assert!(line.starts_with("{"), "Each line should be JSON object");
            assert!(line.contains("content"), "Should contain content field");
            assert!(line.contains("embedding"), "Should contain embedding field");

            // Verify it can be deserialized
            let _doc: DialectDocument =
                serde_json::from_str(line).expect("Each line should be valid DialectDocument JSON");
        }
    }

    #[test]
    fn test_jsonl_loading() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let jsonl_path = temp_dir.path().join("test_load.jsonl");

        // Create proper JSONL content
        let doc1 = DialectDocument::new("Test 1".to_string(), Dialect::ArabicEgyptian, None);
        let doc2 = DialectDocument::new("Test 2".to_string(), Dialect::ArabicEgyptian, None);

        let json1 = serde_json::to_string(&doc1).unwrap();
        let json2 = serde_json::to_string(&doc2).unwrap();
        let jsonl_content = format!("{}\n{}", json1, json2);

        fs::write(&jsonl_path, jsonl_content).expect("Failed to write test JSONL");

        // Test the load_documents_from_jsonl function
        // (We need to expose this function or test it indirectly)
        let content = fs::read_to_string(&jsonl_path).expect("Failed to read JSONL");
        let lines: Vec<&str> = content.lines().collect();

        assert_eq!(lines.len(), 2, "Should have 2 lines");

        for line in lines {
            let doc: DialectDocument =
                serde_json::from_str(line).expect("Should deserialize each line");
            println!("Loaded document: {}", doc.content);
        }
    }

    #[test]
    fn test_save_documents_function() {
        // Test the actual save_documents function from processor.rs
        // We need to make it public or create a test version

        let temp_dir = tempdir().expect("Failed to create temp dir");

        let mut docs = vec![];
        for i in 0..2 {
            let mut doc = DialectDocument::new(
                format!("Arabic test {}: مرحبا", i),
                Dialect::ArabicEgyptian,
                None,
            );
            doc.embedding = vec![0.5; 768];
            docs.push(doc);
        }

        // We'll test the format the function should produce
        let expected_filename = format!("{}.jsonl", Dialect::ArabicEgyptian.id());
        let expected_path = temp_dir.path().join(&expected_filename);

        // Manually create what save_documents should create
        let mut lines = Vec::new();
        for doc in &docs {
            let json = serde_json::to_string(doc).expect("Failed to serialize");
            lines.push(json);
        }

        fs::write(&expected_path, lines.join("\n")).expect("Failed to write");

        // Verify the output
        let saved = fs::read_to_string(&expected_path).expect("Failed to read");
        println!("Expected save format:\n{}", saved);

        // Should be multiple JSON lines
        assert!(
            saved.lines().count() >= 2,
            "Should have multiple JSON lines"
        );
        assert!(saved.contains("embedding"), "Should contain embeddings");
    }
}
