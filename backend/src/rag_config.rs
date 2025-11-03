use serde::{Deserialize, Serialize};

/// Configuration for RAG retrieval
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct RAGConfig {
    /// Number of conversation-matched documents to retrieve (semantic search)
    pub num_conversation_documents: usize,
    /// Number of random documents to retrieve (style diversity)
    pub num_random_documents: usize,
}

impl RAGConfig {
    pub fn new(num_conversation_documents: usize, num_random_documents: usize) -> Self {
        Self {
            num_conversation_documents,
            num_random_documents,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_config() {
        let config = RAGConfig::new(5, 10);
        assert_eq!(config.num_conversation_documents, 5);
        assert_eq!(config.num_random_documents, 10);
    }

    #[test]
    fn test_zero_config() {
        let config = RAGConfig::new(0, 0);
        assert_eq!(config.num_conversation_documents, 0);
        assert_eq!(config.num_random_documents, 0);
    }

    #[test]
    fn test_config_equality() {
        let config1 = RAGConfig::new(10, 15);
        let config2 = RAGConfig::new(10, 15);
        assert_eq!(config1, config2);
    }
}
