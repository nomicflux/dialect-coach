use serde::{Deserialize, Serialize};

/// Configuration for RAG retrieval with multi-vector strategies
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct RAGConfig {
    /// Content vector search WITH formality filter
    pub content_with_formality_limit: usize,
    /// Content vector search WITHOUT formality filter (diversity)
    pub content_without_formality_limit: usize,
    /// Context vector search WITH formality filter
    pub context_with_formality_limit: usize,
    /// Context vector search WITHOUT formality filter (diversity)
    pub context_without_formality_limit: usize,
    /// Keyword vector search WITH formality filter
    pub keyword_with_formality_limit: usize,
    /// Keyword vector search WITHOUT formality filter (diversity)
    pub keyword_without_formality_limit: usize,
}

impl RAGConfig {
    pub fn new(
        content_with_formality_limit: usize,
        content_without_formality_limit: usize,
        context_with_formality_limit: usize,
        context_without_formality_limit: usize,
        keyword_with_formality_limit: usize,
        keyword_without_formality_limit: usize,
    ) -> Self {
        Self {
            content_with_formality_limit,
            content_without_formality_limit,
            context_with_formality_limit,
            context_without_formality_limit,
            keyword_with_formality_limit,
            keyword_without_formality_limit,
        }
    }

    pub fn default_config() -> Self {
        Self::new(5, 3, 5, 3, 5, 3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_config() {
        let config = RAGConfig::new(5, 3, 5, 3, 5, 3);
        assert_eq!(config.content_with_formality_limit, 5);
        assert_eq!(config.content_without_formality_limit, 3);
        assert_eq!(config.context_with_formality_limit, 5);
        assert_eq!(config.context_without_formality_limit, 3);
        assert_eq!(config.keyword_with_formality_limit, 5);
        assert_eq!(config.keyword_without_formality_limit, 3);
    }

    #[test]
    fn test_zero_config() {
        let config = RAGConfig::new(0, 0, 0, 0, 0, 0);
        assert_eq!(config.content_with_formality_limit, 0);
        assert_eq!(config.content_without_formality_limit, 0);
        assert_eq!(config.context_with_formality_limit, 0);
        assert_eq!(config.context_without_formality_limit, 0);
        assert_eq!(config.keyword_with_formality_limit, 0);
        assert_eq!(config.keyword_without_formality_limit, 0);
    }

    #[test]
    fn test_config_equality() {
        let config1 = RAGConfig::new(10, 15, 12, 8, 10, 15);
        let config2 = RAGConfig::new(10, 15, 12, 8, 10, 15);
        assert_eq!(config1, config2);
    }

    #[test]
    fn test_default_config() {
        let config = RAGConfig::default_config();
        assert_eq!(config.content_with_formality_limit, 5);
        assert_eq!(config.content_without_formality_limit, 3);
        assert_eq!(config.context_with_formality_limit, 5);
        assert_eq!(config.context_without_formality_limit, 3);
        assert_eq!(config.keyword_with_formality_limit, 5);
        assert_eq!(config.keyword_without_formality_limit, 3);
    }
}
