use serde::{Deserialize, Serialize};
use super::{DialectConfig, Dialect};

/// A participant in a chat session (human user or AI agent)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Participant {
    pub id: String,
    pub name: String,
    pub participant_type: ParticipantType,
}

/// Type of participant
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ParticipantType {
    Human {
        user_id: String,
    },
    Agent {
        agent_type: AgentType,
        dialect: Dialect,
        config: DialectConfig,
    },
}

/// Type of AI agent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentType {
    /// Main dialect coach agent
    DialectCoach,

    /// Conversation partner for practice
    ConversationPartner,

    /// Grammar and usage explainer
    GrammarExpert,
}

impl Participant {
    /// Create a new human participant
    pub fn new_human(id: String, name: String, user_id: String) -> Self {
        Self {
            id,
            name,
            participant_type: ParticipantType::Human { user_id },
        }
    }

    /// Create a new agent participant
    pub fn new_agent(
        id: String,
        name: String,
        agent_type: AgentType,
        dialect: Dialect,
        config: DialectConfig,
    ) -> Self {
        Self {
            id,
            name,
            participant_type: ParticipantType::Agent {
                agent_type,
                dialect,
                config,
            },
        }
    }

    /// Check if this participant is a human
    pub fn is_human(&self) -> bool {
        matches!(self.participant_type, ParticipantType::Human { .. })
    }

    /// Check if this participant is an agent
    pub fn is_agent(&self) -> bool {
        matches!(self.participant_type, ParticipantType::Agent { .. })
    }

    /// Get the agent type if this is an agent
    pub fn agent_type(&self) -> Option<AgentType> {
        match &self.participant_type {
            ParticipantType::Agent { agent_type, .. } => Some(*agent_type),
            _ => None,
        }
    }

    /// Get the dialect if this is an agent
    pub fn dialect(&self) -> Option<Dialect> {
        match &self.participant_type {
            ParticipantType::Agent { dialect, .. } => Some(*dialect),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Language, Formality, TeachingMode};

    #[test]
    fn test_human_participant() {
        let p = Participant::new_human(
            "h1".to_string(),
            "Alice".to_string(),
            "user123".to_string(),
        );

        assert_eq!(p.id, "h1");
        assert_eq!(p.name, "Alice");
        assert!(p.is_human());
        assert!(!p.is_agent());
        assert!(p.agent_type().is_none());
    }

    #[test]
    fn test_agent_participant() {
        let config = DialectConfig {
            language: Language::Spanish,
            dialect: Dialect::SpanishMexican,
            formality: Formality::Casual,
            teaching_mode: TeachingMode::Immersive,
            personality_traits: vec!["friendly".to_string()],
        };

        let p = Participant::new_agent(
            "a1".to_string(),
            "Coach María".to_string(),
            AgentType::DialectCoach,
            Dialect::SpanishMexican,
            config,
        );

        assert_eq!(p.id, "a1");
        assert!(!p.is_human());
        assert!(p.is_agent());
        assert_eq!(p.agent_type(), Some(AgentType::DialectCoach));
        assert_eq!(p.dialect(), Some(Dialect::SpanishMexican));
    }
}
