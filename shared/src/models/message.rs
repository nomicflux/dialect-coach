use super::{AgentResponse, Formality, TeachingMode};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A message in a chat session
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub session_id: Uuid,
    pub participant_id: String,
    pub content: AgentResponse,
    pub timestamp: DateTime<Utc>,
    pub language: String,
    pub metadata: MessageMetadata,
}

/// Metadata associated with a message
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageMetadata {
    /// Desired formality level for agent responses
    pub formality: Formality,

    /// Teaching mode for agent behavior
    pub teaching_mode: TeachingMode,
}

impl MessageMetadata {
    pub fn new(formality: Formality, teaching_mode: TeachingMode) -> Self {
        Self {
            formality,
            teaching_mode,
        }
    }
}

impl Default for MessageMetadata {
    fn default() -> Self {
        Self {
            formality: Formality::Casual,
            teaching_mode: TeachingMode::Immersive,
        }
    }
}

impl Message {
    /// Create a new message
    pub fn new(
        session_id: Uuid,
        participant_id: String,
        content: AgentResponse,
        language: String,
        formality: Formality,
        teaching_mode: TeachingMode,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id,
            participant_id,
            content,
            timestamp: Utc::now(),
            language,
            metadata: MessageMetadata::new(formality, teaching_mode),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_message() {
        let session_id = Uuid::new_v4();
        let content = AgentResponse::from("Hello!");
        let msg = Message::new(
            session_id,
            "user1".to_string(),
            content.clone(),
            "es-MX".to_string(),
            Formality::Casual,
            TeachingMode::Immersive,
        );

        assert_eq!(msg.session_id, session_id);
        assert_eq!(msg.participant_id, "user1");
        assert_eq!(msg.content, content);
        assert_eq!(msg.content.response, "Hello!");
        assert_eq!(msg.metadata.formality, Formality::Casual);
        assert_eq!(msg.metadata.teaching_mode, TeachingMode::Immersive);
    }

    #[test]
    fn test_metadata_serialization() {
        let content = AgentResponse::from("Hola");
        let msg = Message::new(
            Uuid::new_v4(),
            "user1".to_string(),
            content.clone(),
            "es-MX".to_string(),
            Formality::DialectRich,
            TeachingMode::Corrective,
        );

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"formality\":\"dialect_rich\""));
        assert!(json.contains("\"teaching_mode\":\"corrective\""));

        // Verify metadata fields are not null
        assert!(!json.contains("\"formality\":null"));
        assert!(!json.contains("\"teaching_mode\":null"));
    }
}
