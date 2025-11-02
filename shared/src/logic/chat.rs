use crate::models::{ChatSession, Message};

pub fn add_message_pure(mut session: ChatSession, message: Message) -> ChatSession {
    session.add_message(message);
    session
}

pub fn should_agent_respond(message: &Message) -> bool {
    !message.is_agent()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        AgentResponse, AgentType, Dialect, DialectConfig, Formality, Language, MessageContent,
        MessageMetadata, Participant, SessionConfig, TeachingMode,
    };
    use uuid::Uuid;

    fn test_metadata(session_id: Uuid) -> MessageMetadata {
        MessageMetadata::at_now(
            Formality::Casual,
            TeachingMode::Immersive,
            Language::Spanish,
            Dialect::SpanishArgentinian,
            session_id,
        )
    }

    fn create_test_session() -> ChatSession {
        let mut session = ChatSession::new("Test".to_string(), SessionConfig::default());

        // Add a human
        session.add_participant(Participant::new_human(
            "human1".to_string(),
            "Alice".to_string(),
            "user1".to_string(),
        ));

        // Add an agent
        let config = DialectConfig {
            language: Language::Spanish,
            dialect: Dialect::SpanishMexican,
            formality: Formality::Casual,
            teaching_mode: TeachingMode::Immersive,
            personality_traits: vec![],
        };

        session.add_participant(Participant::new_agent(
            "agent1".to_string(),
            "María".to_string(),
            AgentType::DialectCoach,
            Dialect::SpanishMexican,
            config,
        ));

        session
    }

    #[test]
    fn test_add_message_pure() {
        let session = create_test_session();
        let msg = Message::new(
            MessageContent::UserMessage {
                content: "Hello".to_string(),
            },
            test_metadata(session.id),
            None,
        );

        let initial_count = session.messages.len();
        let new_session = add_message_pure(session, msg);

        assert_eq!(new_session.messages.len(), initial_count + 1);
    }

    #[test]
    fn test_agent_responds_to_human() {
        let session = create_test_session();
        let msg = Message::new(
            MessageContent::UserMessage {
                content: "Hola".to_string(),
            },
            test_metadata(session.id),
            None,
        );

        assert!(should_agent_respond(&msg));
    }

    #[test]
    fn test_agent_does_not_respond_to_self() {
        let session = create_test_session();
        let msg = Message::new(
            MessageContent::AgentMessage {
                content: AgentResponse::from("Hola".to_string()),
            },
            test_metadata(session.id),
            None,
        );

        assert!(!should_agent_respond(&msg));
    }
}
