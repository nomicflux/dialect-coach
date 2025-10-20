use std::fs;
use std::path::Path;
use wiremock::matchers::{method, path as path_matcher};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Helper for setting up a mock Qdrant server with cassette responses
pub struct QdrantMockServer {
    pub mock_server: MockServer,
    pub base_url: String,
}

impl QdrantMockServer {
    /// Start a new mock server with Qdrant API cassettes
    pub async fn start_with_cassettes(collection_name: &str) -> Self {
        let mock_server = MockServer::start().await;
        let base_url = mock_server.uri();

        let cassettes_dir = Path::new("tests/cassettes/qdrant");

        // Mock server version check (required by qdrant_client)
        Mock::given(method("GET"))
            .and(path_matcher("/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "title": "qdrant - vector search engine",
                "version": "1.7.0"
            })))
            .mount(&mock_server)
            .await;

        // Mock GET /collections
        if let Ok(collections_content) = load_cassette(&cassettes_dir, "collections_list.json") {
            Mock::given(method("GET"))
                .and(path_matcher("/collections"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_raw(collections_content, "application/json"),
                )
                .mount(&mock_server)
                .await;
        }

        // Mock GET /collections/{collection_name}
        let collection_info_file = format!("collection_info_{}.json", collection_name);
        if let Ok(collection_info_content) = load_cassette(&cassettes_dir, &collection_info_file) {
            let collection_path = format!("/collections/{}", collection_name);
            Mock::given(method("GET"))
                .and(path_matcher(collection_path.as_str()))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_raw(collection_info_content, "application/json"),
                )
                .mount(&mock_server)
                .await;
        }

        // Mock POST /collections/{collection_name}/points/count
        let count_points_file = format!("count_points_{}_nofilter.json", collection_name);
        if let Ok(count_content) = load_cassette(&cassettes_dir, &count_points_file) {
            let count_path = format!("/collections/{}/points/count", collection_name);
            Mock::given(method("POST"))
                .and(path_matcher(count_path.as_str()))
                .respond_with(
                    ResponseTemplate::new(200).set_body_raw(count_content, "application/json"),
                )
                .mount(&mock_server)
                .await;
        }

        Self {
            mock_server,
            base_url,
        }
    }

    /// Get the base URL for the mock server
    pub fn url(&self) -> &str {
        &self.base_url
    }
}

/// Load cassette file content
fn load_cassette(cassettes_dir: &Path, filename: &str) -> Result<String, std::io::Error> {
    let path = cassettes_dir.join(filename);
    fs::read_to_string(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_server_basic_setup() {
        let mock = QdrantMockServer::start_with_cassettes("dialect_documents").await;

        // Test that the mock server URL is valid
        assert!(mock.url().starts_with("http://"));
        assert!(mock.url().contains("127.0.0.1"));
    }

    #[tokio::test]
    async fn test_cassette_loading() {
        let cassettes_dir = Path::new("tests/cassettes/qdrant");

        // Test loading existing cassettes
        let collections_result = load_cassette(&cassettes_dir, "collections_list.json");
        assert!(
            collections_result.is_ok(),
            "Should load collections_list.json cassette"
        );

        let collection_info_result =
            load_cassette(&cassettes_dir, "collection_info_dialect_documents.json");
        assert!(
            collection_info_result.is_ok(),
            "Should load collection_info cassette"
        );

        // Test that content is valid JSON
        let collections_content = collections_result.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&collections_content)
            .expect("Collections cassette should be valid JSON");

        assert_eq!(parsed["status"], "ok");
        assert!(parsed["result"].is_object());
    }
}
