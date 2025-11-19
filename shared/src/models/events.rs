use super::message::UserMessageWithContext;
use super::{AIActionRequest, Message, Participant};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// WebSocket events for real-time communication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsEvent {
    // Client -> Server
    JoinSession {
        session_id: Uuid,
        user_id: String,
        user_name: String,
    },

    LeaveSession {
        session_id: Uuid,
    },

    SendMessage {
        session_id: Uuid,
        content: String,
        is_speech: bool,
    },

    UserTyping {
        session_id: Uuid,
    },

    RequestAgentResponse {
        session_id: Uuid,
        agent_id: String,
    },

    RequestAIAction {
        session_id: Uuid,
        user_id: Uuid,
        action: AIActionRequest,
    },

    // Server -> Client
    SessionJoined {
        session_id: Uuid,
        participant: Participant,
    },

    UserJoined {
        participant: Participant,
    },

    UserLeft {
        participant_id: String,
    },

    MessageReceived {
        message: Message,
    },

    AgentResponse {
        agent_id: String,
        message: Message,
    },

    AgentThinking {
        agent_id: String,
    },

    TypingIndicator {
        participant_id: String,
    },

    UserMessage {
        user_message: UserMessageWithContext,
    },

    Error {
        message: String,
        code: Option<String>,
    },
}

impl WsEvent {
    /// Check if this is a client-to-server event
    pub fn is_client_event(&self) -> bool {
        matches!(
            self,
            WsEvent::JoinSession { .. }
                | WsEvent::LeaveSession { .. }
                | WsEvent::SendMessage { .. }
                | WsEvent::UserTyping { .. }
                | WsEvent::RequestAgentResponse { .. }
                | WsEvent::RequestAIAction { .. }
        )
    }

    /// Check if this is a server-to-client event
    pub fn is_server_event(&self) -> bool {
        !self.is_client_event()
    }

    /// Get the session ID associated with this event, if any
    pub fn session_id(&self) -> Option<Uuid> {
        match self {
            WsEvent::JoinSession { session_id, .. }
            | WsEvent::LeaveSession { session_id }
            | WsEvent::SendMessage { session_id, .. }
            | WsEvent::UserTyping { session_id }
            | WsEvent::RequestAgentResponse { session_id, .. }
            | WsEvent::RequestAIAction { session_id, .. }
            | WsEvent::SessionJoined { session_id, .. } => Some(*session_id),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_classification() {
        let join_event = WsEvent::JoinSession {
            session_id: Uuid::new_v4(),
            user_id: "user1".to_string(),
            user_name: "Alice".to_string(),
        };

        assert!(join_event.is_client_event());
        assert!(!join_event.is_server_event());

        let error_event = WsEvent::Error {
            message: "Test error".to_string(),
            code: None,
        };

        assert!(!error_event.is_client_event());
        assert!(error_event.is_server_event());
    }

    #[test]
    fn test_session_id_extraction() {
        let session_id = Uuid::new_v4();
        let event = WsEvent::SendMessage {
            session_id,
            content: "Hello".to_string(),
            is_speech: false,
        };

        assert_eq!(event.session_id(), Some(session_id));

        let error_event = WsEvent::Error {
            message: "Test".to_string(),
            code: None,
        };

        assert_eq!(error_event.session_id(), None);
    }

    #[test]
    fn test_request_ai_action_event() {
        use super::AIActionRequest;

        let event = WsEvent::RequestAIAction {
            session_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            action: AIActionRequest::StartConversation,
        };
        assert!(event.is_client_event());
        assert!(event.session_id().is_some());
    }
}
