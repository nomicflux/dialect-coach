use super::{Dialect, Language, Message, Participant};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A chat session with one or more participants
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSession {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub participants: Vec<Participant>,
    pub messages: Vec<Message>,
    pub config: SessionConfig,
}

/// Configuration for a chat session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Primary language for the session
    pub language: Language,

    /// Primary dialect being practiced
    pub dialect: Dialect,

    /// Whether to allow multiple dialects in one session
    pub multi_dialect: bool,

    /// Maximum context window size (number of messages)
    pub context_window: usize,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            language: Language::Spanish,
            dialect: Dialect::SpanishMexican,
            multi_dialect: false,
            context_window: 20,
        }
    }
}

impl ChatSession {
    /// Create a new chat session
    pub fn new(name: String, config: SessionConfig) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            created_at: now,
            updated_at: now,
            participants: Vec::new(),
            messages: Vec::new(),
            config,
        }
    }

    /// Add a participant to the session
    pub fn add_participant(&mut self, participant: Participant) {
        self.participants.push(participant);
        self.updated_at = Utc::now();
    }

    /// Add a message to the session
    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
        self.updated_at = Utc::now();
    }

    /// Get the most recent N messages (for context window)
    pub fn recent_messages(&self, count: usize) -> &[Message] {
        let start = self.messages.len().saturating_sub(count);
        &self.messages[start..]
    }

    /// Get context messages for building agent prompts
    pub fn context_messages(&self) -> &[Message] {
        self.recent_messages(self.config.context_window)
    }

    /// Find a participant by ID
    pub fn get_participant(&self, id: &str) -> Option<&Participant> {
        self.participants.iter().find(|p| p.id == id)
    }

    /// Get all agent participants
    pub fn agents(&self) -> Vec<&Participant> {
        self.participants.iter().filter(|p| p.is_agent()).collect()
    }

    /// Get all human participants
    pub fn humans(&self) -> Vec<&Participant> {
        self.participants.iter().filter(|p| p.is_human()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        AgentType, DialectConfig, Formality, MessageContent, MessageMetadata, TeachingMode,
    };

    fn test_metadata(session_id: Uuid) -> MessageMetadata {
        MessageMetadata::at_now(
            Formality::Casual,
            TeachingMode::Immersive,
            Language::Spanish,
            Dialect::SpanishArgentinian,
            session_id,
        )
    }

    #[test]
    fn test_new_session() {
        let config = SessionConfig::default();
        let session = ChatSession::new("Test Session".to_string(), config);

        assert_eq!(session.name, "Test Session");
        assert_eq!(session.participants.len(), 0);
        assert_eq!(session.messages.len(), 0);
    }

    #[test]
    fn test_add_participant() {
        let mut session = ChatSession::new("Test".to_string(), SessionConfig::default());
        let participant =
            Participant::new_human("h1".to_string(), "Alice".to_string(), "user1".to_string());

        session.add_participant(participant);
        assert_eq!(session.participants.len(), 1);
        assert!(session.get_participant("h1").is_some());
    }

    #[test]
    fn test_add_message() {
        let mut session = ChatSession::new("Test".to_string(), SessionConfig::default());
        let msg = Message::new(
            MessageContent::UserMessage {
                content: "Hello".to_string(),
            },
            test_metadata(session.id),
            None,
        );

        session.add_message(msg);
        assert_eq!(session.messages.len(), 1);
    }

    #[test]
    fn test_recent_messages() {
        let mut session = ChatSession::new("Test".to_string(), SessionConfig::default());

        for i in 0..10 {
            let msg = Message::new(
                MessageContent::UserMessage {
                    content: format!("Message {}", i),
                },
                test_metadata(session.id),
                None,
            );
            session.add_message(msg);
        }

        assert_eq!(session.recent_messages(5).len(), 5);
        assert_eq!(session.recent_messages(20).len(), 10);
        assert_eq!(session.recent_messages(5)[0].get_content(), "Message 5");
    }

    #[test]
    fn test_filter_participants() {
        let mut session = ChatSession::new("Test".to_string(), SessionConfig::default());

        session.add_participant(Participant::new_human(
            "h1".to_string(),
            "Alice".to_string(),
            "user1".to_string(),
        ));

        let config = DialectConfig {
            language: Language::Spanish,
            dialect: Dialect::SpanishMexican,
            formality: Formality::Casual,
            teaching_mode: TeachingMode::Immersive,
            personality_traits: vec![],
        };

        session.add_participant(Participant::new_agent(
            "a1".to_string(),
            "Coach".to_string(),
            AgentType::DialectCoach,
            Dialect::SpanishMexican,
            config,
        ));

        assert_eq!(session.humans().len(), 1);
        assert_eq!(session.agents().len(), 1);
    }
}
