use super::{Dialect, Formality};
use serde::{Deserialize, Serialize};

/// A document from a dialect corpus, used for RAG retrieval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialectDocument {
    pub content: String,
    pub dialect: Dialect,
    pub formality: Option<Formality>,
    pub embedding: Vec<f32>,
}

impl DialectDocument {
    pub fn new(content: String, dialect: Dialect, formality: Option<Formality>) -> Self {
        Self {
            content,
            dialect,
            formality,
            embedding: Vec::new(),
        }
    }
}
