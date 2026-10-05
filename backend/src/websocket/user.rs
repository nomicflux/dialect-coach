use dialect_coach_shared::{NewUser, SignInResult, UsageStats, User, UserState};
use uuid::Uuid;

use super::errors;
use crate::AppState;
use crate::agent_service::AgentService;
use crate::crypto;
use crate::crypto::jwt::Expiry;

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

pub async fn create_user(state: &AppState, new_user: NewUser) -> SignInResult {
    tracing::info!(
        "Creating user: {} with email {}",
        new_user.username,
        new_user.email
    );
    let auth_creds = convert_auth_credentials(new_user.credentials);
    let (user, _) = create_and_save_user(
        state,
        auth_creds,
        new_user.username,
        new_user.email,
        new_user.password,
        new_user.initial_settings,
    )
    .await?;
    sign_in_user(state, user.id).await
}

fn format_auth_error(e: anyhow::Error) -> String {
    if let Some(auth_error) = e.downcast_ref::<crate::auth_service::AuthError>() {
        errors::auth_error_to_message(auth_error.clone())
    } else {
        format!("Authentication failed: {}", e)
    }
}

pub async fn sign_in(state: &AppState, username: String, password: String) -> SignInResult {
    tracing::info!("Sign in request for user: {}", username);
    match state.auth_service.authenticate(&username, &password).await {
        Ok(user) => sign_in_user(state, user.id).await,
        Err(e) => {
            let error_msg = format_auth_error(e);
            tracing::warn!("{}", error_msg);
            Err(error_msg)
        }
    }
}

fn invalid_token(e: jsonwebtoken::errors::Error) -> String {
    let error_msg = format!("Invalid session token: {}", e);
    tracing::warn!("{}", error_msg);
    error_msg
}

/// Sign in from a fresh page's stored token; an expired token is rejected.
pub async fn validate_session(state: &AppState, token: &str) -> SignInResult {
    let user_id = crypto::jwt::validate_token(token, Expiry::Enforce).map_err(invalid_token)?;
    tracing::info!("Session token valid for user_id: {}", user_id);
    let result = sign_in_user(state, user_id).await;
    if result.is_err() {
        tracing::error!("sign_in_user failed: {:?}", result);
    }
    result
}

/// A new 24-hour token for any token with a valid signature, expired or not.
pub fn renew(token: &str) -> Result<(Uuid, String), String> {
    let user_id = crypto::jwt::validate_token(token, Expiry::Ignore).map_err(invalid_token)?;
    let renewed = crypto::jwt::generate_token(user_id)
        .map_err(|e| format!("Failed to generate token: {}", e))?;
    Ok((user_id, renewed))
}

/// Renew a signed-in tab's token for a user who still exists, with their current usage.
pub async fn reattach(state: &AppState, token: &str) -> Result<(Uuid, String, UsageStats), String> {
    let (user_id, renewed) = renew(token)?;
    load_user(state, user_id).await?;
    let usage_stats = state
        .user_persistence
        .load_usage_stats(user_id)
        .await
        .map_err(|e| format!("Failed to load usage stats: {}", e))?;
    Ok((user_id, renewed, usage_stats.unwrap_or_default()))
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

    #[test]
    fn test_renew_accepts_expired_token_and_issues_valid_one() {
        let secret = "test_secret_for_renew";
        unsafe {
            std::env::set_var("JWT_SECRET", secret);
        }
        let user_id = Uuid::new_v4();
        let claims = crypto::jwt::Claims {
            sub: user_id.to_string(),
            exp: 0,
        };
        let expired = jsonwebtoken::encode(
            &jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS256),
            &claims,
            &jsonwebtoken::EncodingKey::from_secret(secret.as_bytes()),
        )
        .unwrap();

        let (renewed_for, renewed) = renew(&expired).unwrap();

        assert_eq!(renewed_for, user_id);
        assert_eq!(
            crypto::jwt::validate_token(&renewed, Expiry::Enforce).unwrap(),
            user_id
        );
    }

    #[test]
    fn test_renew_rejects_unsigned_token() {
        assert!(renew("invalid.token.here").is_err());
    }
}
