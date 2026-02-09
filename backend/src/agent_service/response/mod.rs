use dialect_coach_shared::{
    DialectWithFeatures, Explained, Exploratory, Formality, LanguageLevel, LanguageOption,
    LearningGoal, Mistake, TeachingMode, Translated, UserGender,
};
use rig::completion::Message as RigMessage;
use std::sync::Arc;

use super::keyword_extraction::KeywordExtractor;
use super::provider::CompletionAgent;
use crate::embedding_service::EmbeddingService;
use crate::qdrant_service::QdrantService;
use crate::rag_config::RAGConfig;

// Module declarations
mod config;
mod examples;
mod generation;
mod parsing;
mod retrieval;
mod speaker;
mod system_content;
mod teaching;

/// Parameters for generating a response
pub struct GenerateResponseParams<'a> {
    pub user_message: &'a str,
    pub dialect: DialectWithFeatures,
    pub formality: Formality,
    pub teaching_mode: TeachingMode,
    pub conversation_history: &'a [RigMessage],
    pub learning_goals: &'a [LearningGoal],
    pub rag_config: &'a RAGConfig,
    pub past_mistakes: &'a [Mistake],
    pub past_explained: &'a [Explained],
    pub past_translated: &'a [Translated],
    pub past_exploratory: &'a [Exploratory],
    pub user_gender: UserGender,
    pub language_option: &'a Option<LanguageOption>,
    pub active_plan: Option<&'a dialect_coach_shared::LanguagePlan>,
    pub language_level: LanguageLevel,
}

pub struct ResponseContext {
    pub response_agent: Arc<dyn CompletionAgent>,
    pub learning_agent: Arc<dyn CompletionAgent>,
    pub pronunciation_agent: Arc<dyn CompletionAgent>,
    pub qdrant: Arc<QdrantService>,
    pub embeddings: Arc<EmbeddingService>,
    pub keyword_extractor: Arc<KeywordExtractor>,
}
