use crate::auth_service::{AuthError, UnauthorizedReason};

use dialect_coach_shared::{AgentResponse, Message, MessageMetadata};

pub fn auth_error_to_message(error: crate::auth_service::AuthError) -> String {
    match error {
        AuthError::UserExists => "Username already taken".to_string(),
        AuthError::InvalidCredentials => "Invalid credentials".to_string(),
        AuthError::UserNotFound => "User not found".to_string(),
        AuthError::Unauthorized(UnauthorizedReason::InviteCodeExpired) => {
            "Invite code has expired".to_string()
        }
        AuthError::Unauthorized(UnauthorizedReason::InviteCodeInvalid) => {
            "Invalid invite code".to_string()
        }
        AuthError::Unauthorized(UnauthorizedReason::InviteCodeUsed) => {
            "Invite code already used".to_string()
        }
        AuthError::Persistence(msg) => format!("Authentication error: {}", msg),
    }
}

pub fn error_to_agent_response(error_message: String) -> AgentResponse {
    AgentResponse::from(error_message)
}

pub fn create_error_message(error_text: String, metadata: MessageMetadata) -> Message {
    let error_response = error_to_agent_response(error_text);
    Message::agent_message(error_response, metadata, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dialect_coach_shared::MessageMetadata;
    use dialect_coach_shared::models::{Dialect, Formality, Language, TeachingMode};
    use uuid::Uuid;

    fn test_metadata(session_id: Uuid) -> MessageMetadata {
        MessageMetadata::at_now(
            Formality::Informal,
            TeachingMode::Immersive,
            Language::Spanish,
            Dialect::SpanishMexican,
            session_id,
        )
    }

    #[test]
    fn test_create_error_message() {
        let session_id = Uuid::new_v4();
        let error_msg = create_error_message("Test error".to_string(), test_metadata(session_id));

        assert_eq!(error_msg.get_content(), "Test error");
        assert_eq!(error_msg.metadata.session_id, session_id);
        assert_eq!(error_msg.metadata.formality, Formality::Informal);
        assert_eq!(error_msg.metadata.teaching_mode, TeachingMode::Immersive);
    }
}
