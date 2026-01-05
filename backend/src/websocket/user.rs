use dialect_coach_shared::{User, UserMessage, UserState};
use tokio::sync::mpsc;
use uuid::Uuid;

use super::errors;
use crate::AppState;
use crate::agent_service::AgentService;
use crate::crypto;

async fn sign_in_user(
    state: &AppState,
    user_id: Uuid,
) -> Result<(User, UserState, String), String> {
    let user = load_user(state, user_id).await?;
    let mut user_state = load_user_state(state, user_id).await?;
    user_state.is_admin = user.is_admin;
    let token = crypto::jwt::generate_token(user_id)
        .map_err(|e| format!("Failed to generate token: {}", e))?;
    Ok((user, user_state, token))
}

async fn load_user(state: &AppState, user_id: Uuid) -> Result<User, String> {
    state
        .user_persistence
        .load_user_by_id(user_id)
        .await
        .map_err(|e| format!("Failed to load user: {}", e))?
        .ok_or_else(|| "User not found".to_string())
}

async fn load_user_state(state: &AppState, user_id: Uuid) -> Result<UserState, String> {
    state
        .user_persistence
        .load(user_id)
        .await
        .map_err(|e| format!("Failed to load user state: {}", e))?
        .ok_or_else(|| "Failed to load user state".to_string())
}

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

fn format_create_user_error(e: anyhow::Error) -> String {
    if let Some(auth_error) = e.downcast_ref::<crate::auth_service::AuthError>() {
        errors::auth_error_to_message(auth_error.clone())
    } else {
        format!("Failed to create user: {}", e)
    }
}

async fn create_and_save_user(
    state: &AppState,
    auth_creds: crate::auth_service::AuthCredentials,
    username: String,
    email: String,
    password: String,
    initial_settings: Option<dialect_coach_shared::InitialUserSettings>,
) -> Result<(User, UserState), String> {
    let user = state
        .auth_service
        .create_user(username, email, auth_creds, password)
        .await
        .map_err(|e| {
            let msg = format_create_user_error(e);
            tracing::error!("{}", msg);
            msg
        })?;

    let user_state = UserState::with_initial_settings(user.id, initial_settings);
    state
        .user_persistence
        .save(&user_state)
        .await
        .map_err(|e| {
            tracing::error!("Failed to save user state: {}", e);
            "Failed to save user state".to_string()
        })?;

    Ok((user, user_state))
}

pub async fn handle_create_user(
    state: &AppState,
    username: String,
    email: String,
    credentials: dialect_coach_shared::AuthCredentials,
    password: String,
    initial_settings: Option<dialect_coach_shared::InitialUserSettings>,
    tx: &mpsc::UnboundedSender<String>,
) -> Result<(), ()> {
    tracing::info!("Creating user: {} with email {}", username, email);
    let auth_creds = convert_auth_credentials(credentials);
    let response = match create_and_save_user(
        state, auth_creds, username, email, password, initial_settings,
    )
    .await
    {
        Ok((user, _)) => {
            let result = sign_in_user(state, user.id).await;
            UserMessage::SignInResponse(Box::new(result))
        }
        Err(e) => UserMessage::SignInResponse(Box::new(Err(e))),
    };
    send_user_message(&response, tx)
}

fn format_auth_error(e: anyhow::Error) -> String {
    if let Some(auth_error) = e.downcast_ref::<crate::auth_service::AuthError>() {
        errors::auth_error_to_message(auth_error.clone())
    } else {
        format!("Authentication failed: {}", e)
    }
}

async fn build_sign_in_response(
    state: &AppState,
    username: &str,
    password: &str,
) -> UserMessage {
    match state.auth_service.authenticate(username, password).await {
        Ok(user) => {
            let result = sign_in_user(state, user.id).await;
            UserMessage::SignInResponse(Box::new(result))
        }
        Err(e) => {
            let error_msg = format_auth_error(e);
            tracing::warn!("{}", error_msg);
            UserMessage::SignInResponse(Box::new(Err(error_msg)))
        }
    }
}

pub async fn handle_sign_in(
    state: &AppState,
    username: String,
    password: String,
    tx: &mpsc::UnboundedSender<String>,
) -> Result<(), ()> {
    tracing::info!("Sign in request for user: {}", username);
    let response = build_sign_in_response(state, &username, &password).await;
    send_user_message(&response, tx)
}

pub async fn handle_validate_session(
    state: &AppState,
    token: String,
    tx: &mpsc::UnboundedSender<String>,
) -> Result<(), ()> {
    let response = match crypto::jwt::validate_token(&token) {
        Ok(user_id) => {
            let result = sign_in_user(state, user_id).await;
            UserMessage::SignInResponse(Box::new(result))
        }
        Err(e) => {
            let error_msg = format!("Invalid session token: {}", e);
            tracing::warn!("{}", error_msg);
            UserMessage::SignInResponse(Box::new(Err(error_msg)))
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
        let response =
            UserMessage::SignInResponse(Box::new(Err("User not found".to_string())));

        let result = send_user_message(&response, &tx);
        assert!(result.is_ok());

        let json = rx.try_recv().unwrap();
        let parsed: UserMessage = serde_json::from_str(&json).unwrap();
        if let UserMessage::SignInResponse(result) = parsed {
            assert!(result.is_err());
        } else {
            panic!("Expected SignInResponse");
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
        let user_state = UserState::new(user.id);
        let token = "test_jwt_token".to_string();
        let response = UserMessage::SignInResponse(Box::new(Ok((
            user.clone(),
            user_state.clone(),
            token.clone(),
        ))));

        let result = send_user_message(&response, &tx);
        assert!(result.is_ok());

        let json = rx.try_recv().unwrap();
        let parsed: UserMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, response);
    }

    #[test]
    fn test_send_user_message_sign_in_response_ok() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let user = User::new(
            Uuid::new_v4(),
            "testuser".to_string(),
            "test@example.com".to_string(),
        );
        let user_state = UserState::new(user.id);
        let token = "test_jwt_token".to_string();
        let response = UserMessage::SignInResponse(Box::new(Ok((
            user.clone(),
            user_state.clone(),
            token.clone(),
        ))));

        let result = send_user_message(&response, &tx);
        assert!(result.is_ok());

        let json = rx.try_recv().unwrap();
        let parsed: UserMessage = serde_json::from_str(&json).unwrap();
        if let UserMessage::SignInResponse(result) = parsed {
            let (parsed_user, _parsed_state, parsed_token) = result.unwrap();
            assert_eq!(parsed_user.username, "testuser");
            assert_eq!(parsed_token, "test_jwt_token");
        } else {
            panic!("Expected SignInResponse(Ok)");
        }
    }

    #[test]
    fn test_send_user_message_validate_session() {
        let user = User::new(
            Uuid::new_v4(),
            "validator".to_string(),
            "validator@example.com".to_string(),
        );
        let state = UserState::new(user.id);
        let token = "test_token".to_string();
        let response = UserMessage::SignInResponse(Box::new(Ok((user, state, token))));

        let (tx, mut rx) = mpsc::unbounded_channel();
        let result = send_user_message(&response, &tx);

        assert!(result.is_ok());
        let msg = rx.try_recv().ok();
        assert!(msg.is_some());
    }
}
