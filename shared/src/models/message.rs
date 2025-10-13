use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use super::{Formality, TeachingMode};

/// A message in a chat session
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub session_id: Uuid,
    pub participant_id: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub language: String,
    pub metadata: MessageMetadata,
}

/// Metadata associated with a message
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MessageMetadata {
    /// Whether this message was spoken (vs typed)
    pub is_speech: bool,

    /// Detected language/dialect from speech recognition
    pub detected_dialect: Option<String>,

    /// Confidence score for speech recognition (0.0-1.0)
    pub speech_confidence: Option<f32>,

    /// Any grammar corrections or suggestions
    pub corrections: Vec<Correction>,

    /// Desired formality level for agent responses
    pub formality: Option<Formality>,

    /// Teaching mode for agent behavior
    pub teaching_mode: Option<TeachingMode>,

    /// Additional key-value metadata
    pub extra: HashMap<String, String>,
}

/// A grammar or usage correction
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Correction {
    pub original: String,
    pub corrected: String,
    pub explanation: String,
    pub correction_type: CorrectionType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CorrectionType {
    Grammar,
    Spelling,
    Usage,
    Pronunciation,
    Idiom,
}

impl Message {
    /// Create a new message
    pub fn new(
        session_id: Uuid,
        participant_id: String,
        content: String,
        language: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id,
            participant_id,
            content,
            timestamp: Utc::now(),
            language,
            metadata: MessageMetadata::default(),
        }
    }

    /// Create a speech message
    pub fn new_speech(
        session_id: Uuid,
        participant_id: String,
        content: String,
        language: String,
        confidence: f32,
    ) -> Self {
        let mut msg = Self::new(session_id, participant_id, content, language);
        msg.metadata.is_speech = true;
        msg.metadata.speech_confidence = Some(confidence);
        msg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_message() {
        let session_id = Uuid::new_v4();
        let msg = Message::new(
            session_id,
            "user1".to_string(),
            "Hello!".to_string(),
            "es-MX".to_string(),
        );

        assert_eq!(msg.session_id, session_id);
        assert_eq!(msg.participant_id, "user1");
        assert_eq!(msg.content, "Hello!");
        assert!(!msg.metadata.is_speech);
    }

    #[test]
    fn test_speech_message() {
        let msg = Message::new_speech(
            Uuid::new_v4(),
            "user1".to_string(),
            "Hola".to_string(),
            "es-MX".to_string(),
            0.95,
        );

        assert!(msg.metadata.is_speech);
        assert_eq!(msg.metadata.speech_confidence, Some(0.95));
    }
}
