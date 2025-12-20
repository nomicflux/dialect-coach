use dialect_coach_shared::UserMessage;
use tokio::sync::mpsc;

use super::errors;
use crate::AppState;
use crate::agent_service::AgentService;
use crate::crypto;

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
        dialect_coach_shared::AuthCredentials::Password(pwd) => {
            crate::auth_service::AuthCredentials::Password(pwd)
        }
    }
}

pub async fn handle_create_user(
    state: &AppState,
    username: String,
    email: String,
    credentials: dialect_coach_shared::AuthCredentials,
    password: String,
    _initial_settings: Option<dialect_coach_shared::InitialUserSettings>,
    tx: &mpsc::UnboundedSender<String>,
) -> Result<(), ()> {
    tracing::info!("Creating user: {} with email {}", username, email);

    let auth_creds = convert_auth_credentials(credentials);
    let response = match state
        .auth_service
        .create_user(username, email, auth_creds, password)
        .await
    {
        Ok(user) => match crypto::jwt::generate_token(user.id) {
            Ok(token) => UserMessage::CreateUserResponse(Ok((user, token))),
            Err(e) => {
                tracing::error!("Failed to generate JWT: {}", e);
                UserMessage::CreateUserResponse(Err("Failed to generate session token".to_string()))
            }
        },
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
    password: String,
    tx: &mpsc::UnboundedSender<String>,
) -> Result<(), ()> {
    tracing::info!("Sign in request for user: {}", username);

    let response = match state.auth_service.authenticate(&username, &password).await {
        Ok(user) => match crypto::jwt::generate_token(user.id) {
            Ok(token) => UserMessage::SignInResponse(Ok((user, token))),
            Err(e) => {
                tracing::error!("Failed to generate JWT: {}", e);
                UserMessage::SignInResponse(Err("Failed to generate session token".to_string()))
            }
        },
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

pub async fn handle_validate_session(
    state: &AppState,
    token: String,
    tx: &mpsc::UnboundedSender<String>,
) -> Result<(), ()> {
    tracing::info!("Validating session token");

    let response = match crypto::jwt::validate_token(&token) {
        Ok(user_id) => match state.user_persistence.load_user_by_id(user_id).await {
            Ok(Some(user)) => {
                tracing::info!("Session valid for user: {}", user.username);
                UserMessage::ValidateSessionResponse(Ok(user))
            }
            Ok(None) => {
                tracing::warn!("User not found for valid token");
                UserMessage::ValidateSessionResponse(Err("User not found".to_string()))
            }
            Err(e) => {
                tracing::error!("Database error during session validation: {}", e);
                UserMessage::ValidateSessionResponse(Err("Failed to validate session".to_string()))
            }
        },
        Err(e) => {
            tracing::warn!("Invalid session token: {}", e);
            UserMessage::ValidateSessionResponse(Err("Invalid or expired session".to_string()))
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
        let token = "test_jwt_token".to_string();
        let response = UserMessage::SignInResponse(Ok((user.clone(), token.clone())));

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
        let token = "test_jwt_token".to_string();
        let response = UserMessage::CreateUserResponse(Ok((user.clone(), token.clone())));

        let result = send_user_message(&response, &tx);
        assert!(result.is_ok());

        let json = rx.try_recv().unwrap();
        let parsed: UserMessage = serde_json::from_str(&json).unwrap();
        if let UserMessage::CreateUserResponse(Ok((parsed_user, parsed_token))) = parsed {
            assert_eq!(parsed_user.username, "testuser");
            assert_eq!(parsed_token, "test_jwt_token");
        } else {
            panic!("Expected CreateUserResponse(Ok)");
        }
    }
}
