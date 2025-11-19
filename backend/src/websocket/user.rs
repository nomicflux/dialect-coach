use dialect_coach_shared::UserMessage;
use tokio::sync::mpsc;

use super::errors;
use crate::AppState;
use crate::agent_service::AgentService;

fn convert_auth_credentials(
    creds: dialect_coach_shared::AuthCredentials,
) -> crate::auth_service::AuthCredentials {
    match creds {
        dialect_coach_shared::AuthCredentials::InviteCode(code) => {
            tracing::info!(
                "Converting invite code, length: {}, content: '{}'",
                code.len(),
                code
            );
            crate::auth_service::AuthCredentials::Token(code)
        }
    }
}

pub async fn handle_create_user(
    state: &AppState,
    username: String,
    email: String,
    credentials: dialect_coach_shared::AuthCredentials,
    tx: &mpsc::UnboundedSender<String>,
) -> Result<(), ()> {
    tracing::info!("Creating user: {} with email {}", username, email);

    let auth_creds = convert_auth_credentials(credentials);
    let response = match state
        .auth_service
        .create_user(username, email, auth_creds)
        .await
    {
        Ok(user) => UserMessage::CreateUserResponse(Ok(user)),
        Err(e) => {
            let error_msg =
                if let Some(auth_error) = e.downcast_ref::<crate::auth_service::AuthError>() {
                    errors::auth_error_to_message(auth_error.clone())
                } else {
                    format!("Failed to create user: {}", e)
                };
            tracing::error!("{}", error_msg);
            UserMessage::CreateUserResponse(Err(error_msg))
        }
    };

    send_user_message(&response, tx)
}

pub async fn handle_sign_in(
    state: &AppState,
    username: String,
    tx: &mpsc::UnboundedSender<String>,
) -> Result<(), ()> {
    tracing::info!("Sign in request for user: {}", username);

    let response = match state.auth_service.authenticate(&username).await {
        Ok(user) => UserMessage::SignInResponse(Ok(user)),
        Err(e) => {
            let error_msg =
                if let Some(auth_error) = e.downcast_ref::<crate::auth_service::AuthError>() {
                    errors::auth_error_to_message(auth_error.clone())
                } else {
                    format!("Authentication failed: {}", e)
                };
            tracing::warn!("{}", error_msg);
            UserMessage::SignInResponse(Err(error_msg))
        }
    };

    send_user_message(&response, tx)
}

pub fn send_user_message(msg: &UserMessage, tx: &mpsc::UnboundedSender<String>) -> Result<(), ()> {
    let json = serde_json::to_string(msg)
        .map_err(|e| tracing::error!("Failed to serialize UserMessage: {}", e))?;

    tx.send(json)
        .map_err(|e| tracing::error!("Failed to send UserMessage: {}", e))?;

    Ok(())
}

pub fn validate_user_message(user_text: &str) -> Result<(), anyhow::Error> {
    if AgentService::contains_illegal_characters(user_text) {
        tracing::error!("User message contains illegal characters (null bytes or control chars)");
        return Err(anyhow::anyhow!("User message contains illegal characters"));
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use dialect_coach_shared::User;
    use uuid::Uuid;

    #[test]
    fn test_send_user_message_sign_in_response_err() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let response = UserMessage::SignInResponse(Err("User not found".to_string()));

        let result = send_user_message(&response, &tx);
        assert!(result.is_ok());

        let json = rx.try_recv().unwrap();
        let parsed: UserMessage = serde_json::from_str(&json).unwrap();
        if let UserMessage::SignInResponse(Err(err_msg)) = parsed {
            assert_eq!(err_msg, "User not found");
        } else {
            panic!("Expected SignInResponse(Err)");
        }
    }

    #[test]
    fn test_send_user_message_serialization() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let user = User::new(
            Uuid::new_v4(),
            "bob".to_string(),
            "bob@example.com".to_string(),
        );
        let response = UserMessage::SignInResponse(Ok(user.clone()));

        let result = send_user_message(&response, &tx);
        assert!(result.is_ok());

        let json = rx.try_recv().unwrap();
        let parsed: UserMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, response);
    }

    #[test]
    fn test_send_user_message_create_user_response_ok() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let user = User::new(
            Uuid::new_v4(),
            "testuser".to_string(),
            "test@example.com".to_string(),
        );
        let response = UserMessage::CreateUserResponse(Ok(user.clone()));

        let result = send_user_message(&response, &tx);
        assert!(result.is_ok());

        let json = rx.try_recv().unwrap();
        let parsed: UserMessage = serde_json::from_str(&json).unwrap();
        if let UserMessage::CreateUserResponse(Ok(parsed_user)) = parsed {
            assert_eq!(parsed_user.username, "testuser");
        } else {
            panic!("Expected CreateUserResponse(Ok)");
        }
    }
}
