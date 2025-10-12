use crate::models::{ChatSession, Message, Participant, AgentType};

/// Pure function: Add a message to a session (returns new session)
/// Following functional programming principles for easier testing
pub fn add_message_pure(mut session: ChatSession, message: Message) -> ChatSession {
    session.add_message(message);
    session
}

/// Check if an agent should respond to a message
pub fn should_agent_respond(
    session: &ChatSession,
    agent_id: &str,
    message: &Message,
) -> bool {
    // Don't respond to own messages
    if message.participant_id == agent_id {
        return false;
    }

    // Get the agent
    let agent = match session.get_participant(agent_id) {
        Some(p) if p.is_agent() => p,
        _ => return false,
    };

    // Agent responds if:
    // 1. It's mentioned by name
    // 2. It's the only agent in the session
    // 3. The message is from a human (not another agent)

    let sender = session.get_participant(&message.participant_id);
    let sender_is_human = sender.map(|p| p.is_human()).unwrap_or(false);

    if !sender_is_human {
        return false; // Agents don't respond to other agents
    }

    // Check if agent is mentioned
    let agent_name_lower = agent.name.to_lowercase();
    if message.content.to_lowercase().contains(&agent_name_lower) {
        return true;
    }

    // If only one agent, always respond to humans
    let agent_count = session.agents().len();
    if agent_count == 1 {
        return true;
    }

    // For multi-agent sessions, be more selective
    // For now, respond randomly or based on agent type
    // (This can be enhanced with more sophisticated logic)
    match agent.agent_type() {
        Some(AgentType::DialectCoach) => true, // Primary coach always responds
        _ => false, // Other agents wait to be mentioned
    }
}

/// Get the list of agent IDs that should respond to a message
pub fn agents_to_respond(session: &ChatSession, message: &Message) -> Vec<String> {
    session
        .agents()
        .iter()
        .filter(|agent| should_agent_respond(session, &agent.id, message))
        .map(|agent| agent.id.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{SessionConfig, DialectConfig, Dialect, Language, Formality, TeachingMode};
    use uuid::Uuid;

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
            session.id,
            "human1".to_string(),
            "Hello".to_string(),
            "es-MX".to_string(),
        );

        let initial_count = session.messages.len();
        let new_session = add_message_pure(session, msg);

        assert_eq!(new_session.messages.len(), initial_count + 1);
    }

    #[test]
    fn test_agent_responds_to_human() {
        let session = create_test_session();
        let msg = Message::new(
            session.id,
            "human1".to_string(),
            "Hola".to_string(),
            "es-MX".to_string(),
        );

        assert!(should_agent_respond(&session, "agent1", &msg));
    }

    #[test]
    fn test_agent_does_not_respond_to_self() {
        let session = create_test_session();
        let msg = Message::new(
            session.id,
            "agent1".to_string(),
            "Hola".to_string(),
            "es-MX".to_string(),
        );

        assert!(!should_agent_respond(&session, "agent1", &msg));
    }

    #[test]
    fn test_agents_to_respond() {
        let session = create_test_session();
        let msg = Message::new(
            session.id,
            "human1".to_string(),
            "¿Cómo estás?".to_string(),
            "es-MX".to_string(),
        );

        let agents = agents_to_respond(&session, &msg);
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0], "agent1");
    }
}
