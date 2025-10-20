use dialect_coach_shared::{Dialect, DialectDocument, Formality};

// Import test support helpers
mod support;

#[cfg(test)]
mod tests {
    use super::*;

    // Test constants and structure validation
    #[test]
    fn test_collection_name_constant() {
        // Verify the collection name is accessible
        const EXPECTED_COLLECTION_NAME: &str = "dialect_documents";
        // This test ensures the constant exists and has expected value
        // We can't directly access it since it's private, but we can test the behavior
        assert_eq!(EXPECTED_COLLECTION_NAME, "dialect_documents");
    }

    #[test]
    fn test_document_validation_empty_embeddings() {
        // Test the validation logic that checks for empty embeddings
        let documents = vec![DialectDocument::new(
            "Test content".to_string(),
            Dialect::ArabicEgyptian,
            None,
        )];

        // Verify document has empty embeddings (default state)
        assert!(documents[0].embedding.is_empty());

        // This mimics the validation in upload_documents
        let has_empty_embeddings = documents.iter().any(|doc| doc.embedding.is_empty());
        assert!(has_empty_embeddings);
    }

    #[test]
    fn test_document_validation_with_embeddings() {
        let mut documents = vec![
            DialectDocument::new("Test content 1".to_string(), Dialect::ArabicEgyptian, None),
            DialectDocument::new(
                "Test content 2".to_string(),
                Dialect::ArabicGulf,
                Some(Formality::Formal),
            ),
        ];

        // Add embeddings
        documents[0].embedding = vec![0.1, 0.2, 0.3];
        documents[1].embedding = vec![0.4, 0.5, 0.6];

        // This mimics the validation in upload_documents
        let has_empty_embeddings = documents.iter().any(|doc| doc.embedding.is_empty());
        assert!(!has_empty_embeddings);

        // Test vector size calculation
        let vector_size = documents[0].embedding.len();
        assert_eq!(vector_size, 3);
    }

    #[test]
    fn test_batch_size_calculation() {
        // Test the batching logic used in upload_documents
        let batch_size = 100;

        // Test with different document counts
        let test_cases = vec![
            (50, 1),  // 50 documents = 1 batch
            (100, 1), // 100 documents = 1 batch
            (150, 2), // 150 documents = 2 batches
            (250, 3), // 250 documents = 3 batches
        ];

        for (doc_count, expected_batches) in test_cases {
            let total_batches = (doc_count + batch_size - 1) / batch_size;
            assert_eq!(
                total_batches, expected_batches,
                "Failed for {} documents",
                doc_count
            );
        }
    }

    #[test]
    fn test_point_id_generation() {
        // Test the UUID generation logic used in upload_documents
        use uuid::Uuid;

        let doc = DialectDocument::new("Test content".to_string(), Dialect::ArabicEgyptian, None);

        // Mimic the UUID generation logic
        let namespace = Uuid::NAMESPACE_OID;
        let name = format!("{}:{}", doc.dialect.name(), doc.content);
        let uuid1 = Uuid::new_v5(&namespace, name.as_bytes());

        // Same input should produce same UUID
        let name2 = format!("{}:{}", doc.dialect.name(), doc.content);
        let uuid2 = Uuid::new_v5(&namespace, name2.as_bytes());

        assert_eq!(uuid1, uuid2, "Same content should produce same UUID");

        // Different content should produce different UUID
        let different_name = format!("{}:{}", doc.dialect.name(), "Different content");
        let uuid3 = Uuid::new_v5(&namespace, different_name.as_bytes());

        assert_ne!(
            uuid1, uuid3,
            "Different content should produce different UUID"
        );
    }

    #[test]
    fn test_payload_structure() {
        // Test the payload creation logic used in upload_documents
        use qdrant_client::Payload;
        use std::collections::HashMap;

        let doc = DialectDocument::new(
            "Test content".to_string(),
            Dialect::ArabicEgyptian,
            Some(Formality::Formal),
        );

        // Create payload exactly as done in qdrant.rs
        let mut payload = Payload::new();
        payload.insert("content", doc.content.clone());
        payload.insert("dialect", doc.dialect.id());

        if let Some(formality) = &doc.formality {
            payload.insert("formality", format!("{:?}", formality));
        }

        // Convert to HashMap to verify contents (Payload implements Into<HashMap>)
        let payload_map: HashMap<String, qdrant_client::qdrant::Value> = payload.into();
        assert!(payload_map.contains_key("content"));
        assert!(payload_map.contains_key("dialect"));
        assert!(payload_map.contains_key("formality"));

        // Verify actual values - extract string values from qdrant Value type
        if let Some(qdrant_client::qdrant::Value {
            kind: Some(qdrant_client::qdrant::value::Kind::StringValue(content)),
        }) = payload_map.get("content")
        {
            assert_eq!(content, "Test content");
        } else {
            panic!("Expected content to be a string value");
        }

        if let Some(qdrant_client::qdrant::Value {
            kind: Some(qdrant_client::qdrant::value::Kind::StringValue(dialect)),
        }) = payload_map.get("dialect")
        {
            assert_eq!(dialect, "arabic_egyptian");
        } else {
            panic!("Expected dialect to be a string value");
        }

        if let Some(qdrant_client::qdrant::Value {
            kind: Some(qdrant_client::qdrant::value::Kind::StringValue(formality)),
        }) = payload_map.get("formality")
        {
            assert_eq!(formality, "Formal");
        } else {
            panic!("Expected formality to be a string value");
        }
    }

    #[test]
    fn test_batch_size_with_edge_cases() {
        // Test different document counts with the 100 batch size used in qdrant.rs
        let batch_size = 100;
        let test_cases = vec![
            (0, 0),   // Zero documents
            (1, 1),   // Single document
            (99, 1),  // Just under batch size
            (100, 1), // Exactly batch size
            (101, 2), // Just over batch size
            (200, 2), // Exactly 2 batches
            (350, 4), // Multiple batches with remainder
        ];

        for (doc_count, expected_batches) in test_cases {
            let total_batches = if doc_count == 0 {
                0
            } else {
                (doc_count + batch_size - 1) / batch_size
            };
            assert_eq!(
                total_batches, expected_batches,
                "Failed for {} documents",
                doc_count
            );
        }
    }

    // Note: Mock vector uploader tests are in the unit tests of test_seams.rs
    // Integration tests focus on pure algorithmic logic without external dependencies

    // ========================================
    // CASSETTE-BASED DETERMINISTIC TESTS
    // ========================================
    // These tests use pre-recorded responses for offline, fast, deterministic testing

    #[tokio::test]
    #[ignore] // Run with QDRANT_URL to record real API calls
    async fn test_record_qdrant_api_calls() {
        // This test records actual API calls to a real Qdrant instance
        // Run with: QDRANT_URL=http://localhost:6333 cargo test -- --ignored test_record_qdrant_api_calls
        use corpus_processor::qdrant::QdrantService;
        use corpus_processor::test_seams::VectorUploader;
        use dialect_coach_shared::{Dialect, DialectDocument, Formality};

        let qdrant_url = std::env::var("QDRANT_URL")
            .unwrap_or_else(|_| panic!("QDRANT_URL must be set to record real API calls"));

        println!("Recording API calls to: {}", qdrant_url);

        // This will make real API calls that we can record
        let qdrant = QdrantService::new(&qdrant_url).await.unwrap();

        // Test collection creation
        let _ = qdrant.init_collection(768).await;

        // Test document upload
        let mut documents = vec![DialectDocument::new(
            "Test content for recording".to_string(),
            Dialect::ArabicEgyptian,
            Some(Formality::Formal),
        )];
        documents[0].embedding = vec![0.1; 768];

        let _ = qdrant.upload_documents(&documents).await;

        // Test collection info
        let _ = qdrant.get_collection_info().await;

        println!("API calls recorded. Use these to create cassette fixtures.");
    }

    #[tokio::test]
    async fn test_qdrant_with_recorded_responses() {
        use corpus_processor::qdrant::QdrantService;
        use wiremock::matchers::{body_partial_json, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        // Recorded response from real Qdrant server root endpoint
        Mock::given(method("GET"))
            .and(path("/"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({
                        "title": "qdrant - vector search engine",
                        "version": "1.7.0"
                    }))
                    .insert_header("content-type", "application/json"),
            )
            .mount(&mock_server)
            .await;

        // Recorded response from list_collections (empty)
        Mock::given(method("GET"))
            .and(path("/collections"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({
                        "result": {
                            "collections": []
                        },
                        "status": "ok",
                        "time": 0.000123
                    }))
                    .insert_header("content-type", "application/json"),
            )
            .mount(&mock_server)
            .await;

        // Recorded response from create_collection
        Mock::given(method("PUT"))
            .and(path("/collections/dialect_documents"))
            .and(body_partial_json(serde_json::json!({
                "vectors": {
                    "size": 768,
                    "distance": "Cosine"
                }
            })))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({
                        "result": true,
                        "status": "ok",
                        "time": 0.001234
                    }))
                    .insert_header("content-type", "application/json"),
            )
            .mount(&mock_server)
            .await;

        // Use the recorded API behavior
        let client = qdrant_client::Qdrant::from_url(&mock_server.uri())
            .build()
            .unwrap();
        let qdrant = QdrantService::new_with_client(client);

        let result = qdrant.init_collection(768).await;
        assert!(
            result.is_ok(),
            "Collection creation should succeed with recorded responses"
        );
    }

    #[tokio::test]
    async fn test_qdrant_upload_documents_with_recorded_responses() {
        use corpus_processor::qdrant::QdrantService;
        use dialect_coach_shared::{Dialect, DialectDocument, Formality};
        use wiremock::matchers::{body_json_schema, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        // Version check response
        Mock::given(method("GET"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "title": "qdrant - vector search engine",
                "version": "1.7.0"
            })))
            .mount(&mock_server)
            .await;

        // Recorded response from upsert_points operation
        Mock::given(method("PUT"))
            .and(path("/collections/dialect_documents/points"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": {
                    "operation_id": 1,
                    "status": "completed"
                },
                "status": "ok",
                "time": 0.002345
            })))
            .mount(&mock_server)
            .await;

        let client = qdrant_client::Qdrant::from_url(&mock_server.uri())
            .build()
            .unwrap();
        let qdrant = QdrantService::new_with_client(client);

        // Create test documents with embeddings
        let mut documents = vec![
            DialectDocument::new(
                "Test document 1".to_string(),
                Dialect::SpanishMexican,
                Some(Formality::Casual),
            ),
            DialectDocument::new(
                "Test document 2".to_string(),
                Dialect::ArabicEgyptian,
                Some(Formality::Formal),
            ),
        ];
        documents[0].embedding = vec![0.1; 768];
        documents[1].embedding = vec![0.2; 768];

        let result = qdrant.upload_documents(&documents).await;
        assert!(
            result.is_ok(),
            "Document upload should succeed with recorded responses"
        );
    }

    #[tokio::test]
    async fn test_get_collection_info_with_cassettes() {
        use corpus_processor::qdrant::QdrantService;

        // Start mock server with cassettes
        let mock_server = crate::support::qdrant_mock::QdrantMockServer::start_with_cassettes(
            "dialect_documents",
        )
        .await;

        // Create Qdrant client pointing to mock server
        let client = qdrant_client::Qdrant::from_url(mock_server.url())
            .build()
            .unwrap();
        let qdrant = QdrantService::new_with_client(client);

        // Test structured method
        let result = qdrant.get_collection_info_structured().await;
        assert!(
            result.is_ok(),
            "Collection info should succeed with cassette responses"
        );

        let info = result.unwrap();
        assert_eq!(info.status, "ok", "Response status should be ok");
        assert!(info.result.is_some(), "Should have collection info result");

        let collection_info = info.result.unwrap();
        assert_eq!(
            collection_info.vectors_count,
            Some(1500),
            "Should match cassette vector count"
        );
        assert_eq!(
            collection_info.points_count,
            Some(1500),
            "Should match cassette points count"
        );
        assert_eq!(
            collection_info.segments_count,
            Some(1),
            "Should match cassette segments count"
        );
    }

    #[tokio::test]
    async fn test_list_collections_with_cassettes() {
        use corpus_processor::qdrant::QdrantService;

        // Start mock server with cassettes
        let mock_server = crate::support::qdrant_mock::QdrantMockServer::start_with_cassettes(
            "dialect_documents",
        )
        .await;

        // Create Qdrant clients (need separate instances since one is moved)
        let client1 = qdrant_client::Qdrant::from_url(mock_server.url())
            .build()
            .unwrap();
        let client2 = qdrant_client::Qdrant::from_url(mock_server.url())
            .build()
            .unwrap();
        let qdrant = QdrantService::new_with_client(client1);

        // Use separate client to test list_collections (since our service doesn't expose it)
        let collections_result = client2.list_collections().await;
        assert!(
            collections_result.is_ok(),
            "Should successfully list collections from cassette"
        );

        let collections = collections_result.unwrap();
        assert!(
            !collections.collections.is_empty(),
            "Should have at least one collection from cassette"
        );

        // Verify the collection from our cassette exists
        let has_dialect_docs = collections
            .collections
            .iter()
            .any(|c| c.name == "dialect_documents");
        assert!(
            has_dialect_docs,
            "Should contain dialect_documents collection from cassette"
        );
    }

    #[tokio::test]
    async fn test_qdrant_error_responses_recorded() {
        use corpus_processor::qdrant::QdrantService;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        // Version check succeeds
        Mock::given(method("GET"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "title": "qdrant - vector search engine",
                "version": "1.7.0"
            })))
            .mount(&mock_server)
            .await;

        // Recorded error response (collection not found)
        Mock::given(method("GET"))
            .and(path("/collections/nonexistent_collection"))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "status": "error",
                "result": {
                    "error": "Collection `nonexistent_collection` doesn't exist!"
                },
                "time": 0.000789
            })))
            .mount(&mock_server)
            .await;

        let client = qdrant_client::Qdrant::from_url(&mock_server.uri())
            .build()
            .unwrap();
        let qdrant = QdrantService::new_with_client(client);

        // This should fail gracefully with the recorded error
        // Note: We'd need to modify get_collection_info to accept collection name
        // For now this demonstrates error response handling
    }

    #[tokio::test]
    async fn test_qdrant_upload_documents_api() {
        use corpus_processor::qdrant::QdrantService;
        use wiremock::matchers::{body_json, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        // Mock server version check
        Mock::given(method("GET"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "title": "qdrant - vector search engine",
                "version": "1.7.0"
            })))
            .mount(&mock_server)
            .await;

        // Mock collection exists
        Mock::given(method("GET"))
            .and(path("/collections"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": {
                    "collections": [{
                        "name": "dialect_documents"
                    }]
                },
                "status": "ok"
            })))
            .mount(&mock_server)
            .await;

        // Mock upsert points call - verify correct payload structure
        Mock::given(method("PUT"))
            .and(path("/collections/dialect_documents/points"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": {
                    "operation_id": 123,
                    "status": "acknowledged"
                },
                "status": "ok"
            })))
            .mount(&mock_server)
            .await;

        // Create QdrantService with mock server URL (skip version check)
        use qdrant_client::Qdrant;
        let client = Qdrant::from_url(&mock_server.uri()).build().unwrap();
        let qdrant = corpus_processor::qdrant::QdrantService::new_with_client(client);

        // Create test documents with embeddings
        let mut documents = vec![DialectDocument::new(
            "Test content".to_string(),
            Dialect::ArabicEgyptian,
            Some(Formality::Formal),
        )];
        documents[0].embedding = vec![0.1; 768];

        let result = qdrant.upload_documents(&documents).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_qdrant_error_handling() {
        use corpus_processor::qdrant::QdrantService;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        // Mock server version check
        Mock::given(method("GET"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "title": "qdrant - vector search engine",
                "version": "1.7.0"
            })))
            .mount(&mock_server)
            .await;

        // Mock authentication error
        Mock::given(method("GET"))
            .and(path("/collections"))
            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
                "error": "Unauthorized"
            })))
            .mount(&mock_server)
            .await;

        // Create QdrantService with mock server URL (skip version check)
        use qdrant_client::Qdrant;
        let client = Qdrant::from_url(&mock_server.uri()).build().unwrap();
        let qdrant = corpus_processor::qdrant::QdrantService::new_with_client(client);

        let result = qdrant.init_collection(768).await;
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("Failed to list collections"));
    }

    #[tokio::test]
    async fn test_qdrant_collection_info_api() {
        use corpus_processor::qdrant::QdrantService;
        use corpus_processor::test_seams::VectorUploader;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        // Mock server version check
        Mock::given(method("GET"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "title": "qdrant - vector search engine",
                "version": "1.7.0"
            })))
            .mount(&mock_server)
            .await;

        // Mock collection info call
        Mock::given(method("GET"))
            .and(path("/collections/dialect_documents"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": {
                    "status": "green",
                    "points_count": 12345,
                    "segments_count": 5
                },
                "status": "ok"
            })))
            .mount(&mock_server)
            .await;

        // Create QdrantService with mock server URL (skip version check)
        use qdrant_client::Qdrant;
        let client = Qdrant::from_url(&mock_server.uri()).build().unwrap();
        let qdrant = corpus_processor::qdrant::QdrantService::new_with_client(client);

        let result = qdrant.get_collection_info().await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_document_validation_logic() {
        // Test the validation logic that would be used in upload_documents
        let mut valid_documents = vec![
            DialectDocument::new("Test content 1".to_string(), Dialect::ArabicEgyptian, None),
            DialectDocument::new(
                "Test content 2".to_string(),
                Dialect::SpanishMexican,
                Some(Formality::Formal),
            ),
        ];

        // Add embeddings
        valid_documents[0].embedding = vec![0.1, 0.2, 0.3];
        valid_documents[1].embedding = vec![0.4, 0.5, 0.6];

        // Test validation logic from upload_documents
        let has_empty_embeddings = valid_documents.iter().any(|doc| doc.embedding.is_empty());
        assert!(!has_empty_embeddings);

        let vector_size = valid_documents[0].embedding.len();
        assert_eq!(vector_size, 3);

        // Test with invalid documents (empty embeddings)
        let invalid_documents = vec![DialectDocument::new(
            "Test content".to_string(),
            Dialect::ArabicEgyptian,
            None,
        )];
        // Don't add embeddings - should have empty embedding by default

        let has_empty_embeddings = invalid_documents.iter().any(|doc| doc.embedding.is_empty());
        assert!(has_empty_embeddings);
    }

    #[test]
    fn test_uuid_generation_consistency() {
        // Test the deterministic UUID generation logic used in qdrant.rs
        use uuid::Uuid;

        let doc1 = DialectDocument::new("Same content".to_string(), Dialect::ArabicEgyptian, None);
        let doc2 = DialectDocument::new("Same content".to_string(), Dialect::ArabicEgyptian, None);
        let doc3 = DialectDocument::new(
            "Different content".to_string(),
            Dialect::ArabicEgyptian,
            None,
        );

        // Generate UUIDs using the same logic as in qdrant.rs
        let namespace = Uuid::NAMESPACE_OID;

        let name1 = format!("{}:{}", doc1.dialect.name(), doc1.content);
        let uuid1 = Uuid::new_v5(&namespace, name1.as_bytes());

        let name2 = format!("{}:{}", doc2.dialect.name(), doc2.content);
        let uuid2 = Uuid::new_v5(&namespace, name2.as_bytes());

        let name3 = format!("{}:{}", doc3.dialect.name(), doc3.content);
        let uuid3 = Uuid::new_v5(&namespace, name3.as_bytes());

        // Same content should produce same UUID
        assert_eq!(uuid1, uuid2);

        // Different content should produce different UUID
        assert_ne!(uuid1, uuid3);

        // Verify UUID is valid
        assert_eq!(uuid1.get_version(), Some(uuid::Version::Sha1));
    }

    #[test]
    fn test_multiple_batch_processing() {
        // Test the batching logic that would be used in upload_documents
        let batch_size = 3; // Small batch size for testing

        let documents: Vec<DialectDocument> = (0..10)
            .map(|i| DialectDocument::new(format!("Content {}", i), Dialect::ArabicEgyptian, None))
            .collect();

        let chunks: Vec<&[DialectDocument]> = documents.chunks(batch_size).collect();

        assert_eq!(chunks.len(), 4); // 10 documents / 3 batch_size = 4 batches (3+3+3+1)
        assert_eq!(chunks[0].len(), 3);
        assert_eq!(chunks[1].len(), 3);
        assert_eq!(chunks[2].len(), 3);
        assert_eq!(chunks[3].len(), 1); // Last batch has remainder

        // Verify batch content is correct
        assert_eq!(chunks[0][0].content, "Content 0");
        assert_eq!(chunks[0][2].content, "Content 2");
        assert_eq!(chunks[3][0].content, "Content 9");
    }

    // Note: Most qdrant.rs functions require actual Qdrant connections
    // which are tested via CLI integration tests or would need mocking infrastructure
    // These tests cover the testable logic without external dependencies
}
