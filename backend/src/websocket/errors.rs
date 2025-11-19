use crate::auth_service::{AuthError, UnauthorizedReason};

use dialect_coach_shared::AgentResponse;

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
