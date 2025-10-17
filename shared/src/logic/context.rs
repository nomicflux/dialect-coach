use crate::models::{ChatSession, Message, Participant};

/// Build a context string from recent messages for an agent prompt
pub fn build_conversation_context(session: &ChatSession, max_messages: usize) -> String {
    let messages = session.recent_messages(max_messages);

    messages
        .iter()
        .map(|msg| {
            let participant = session
                .get_participant(&msg.participant_id)
                .map(|p| p.name.as_str())
                .unwrap_or("Unknown");

            format!("{}: {}", participant, msg.content)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn build_agent_prompt(
    agent: &Participant,
    session: &ChatSession,
    user_message: &Message,
) -> String {
    let context = build_conversation_context(session, 10);

    format!(
        "You are {}. Respond naturally in character.\n\nConversation so far:\n{}\n\nRespond to latest user message: {}",
        agent.name, context, user_message.content
    )
}

/// Estimate token count (rough approximation: 1 token ≈ 4 characters)
pub fn estimate_tokens(text: &str) -> usize {
    text.chars().count() / 4
}

/// Truncate context to fit within a token budget
pub fn truncate_to_tokens(text: &str, max_tokens: usize) -> String {
    let max_chars = max_tokens * 4;
    if text.chars().count() <= max_chars {
        text.to_string()
    } else {
        text.chars().take(max_chars).collect::<String>() + "..."
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        AgentType, Dialect, DialectConfig, Formality, Language, SessionConfig, TeachingMode,
    };

    #[test]
    fn test_build_conversation_context() {
        let mut session = ChatSession::new("Test".to_string(), SessionConfig::default());

        session.add_participant(Participant::new_human(
            "h1".to_string(),
            "Alice".to_string(),
            "user1".to_string(),
        ));

        let msg1 = Message::new(
            session.id,
            "h1".to_string(),
            "Hello".to_string(),
            "es-MX".to_string(),
        );
        session.add_message(msg1);

        let context = build_conversation_context(&session, 10);
        assert!(context.contains("Alice:"));
        assert!(context.contains("Hello"));
    }

    #[test]
    fn test_estimate_tokens() {
        let text = "This is a test";
        let tokens = estimate_tokens(text);
        assert!(tokens > 0);
        assert!(tokens < text.len());
    }

    #[test]
    fn test_truncate_to_tokens() {
        let text = "A".repeat(1000);
        let truncated = truncate_to_tokens(&text, 50);
        assert!(truncated.len() < text.len());
        assert!(truncated.ends_with("..."));
    }
}
