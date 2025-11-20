use super::AIActionRequest;
use super::message::UserMessageWithContext;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// WebSocket events for real-time communication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsEvent {
    RequestAIAction {
        session_id: Uuid,
        user_id: Uuid,
        action: AIActionRequest,
    },

    UserMessage {
        user_message: Box<UserMessageWithContext>,
    },
}

impl WsEvent {}
