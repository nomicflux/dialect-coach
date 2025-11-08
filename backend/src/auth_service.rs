use anyhow::Result;
use dialect_coach_shared::User;
use std::sync::Arc;
use uuid::Uuid;

use crate::persistence::UserPersistence;

#[derive(Debug, Clone)]
pub enum AuthCredentials {
    Password(String),
    Token(String),
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("User already exists")]
    UserExists,
    #[error("Invalid credentials")]
    InvalidCredentials,
    #[error("User not found")]
    UserNotFound,
    #[error("Unauthorized: {0}")]
    Unauthorized(UnauthorizedReason),
    #[error("Persistence error: {0}")]
    Persistence(String),
}

#[derive(Debug, Clone)]
pub enum UnauthorizedReason {
    InviteCodeExpired,
    InviteCodeInvalid,
    InviteCodeUsed,
    Other(String),
}

impl std::fmt::Display for UnauthorizedReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InviteCodeExpired => write!(f, "invite code expired"),
            Self::InviteCodeInvalid => write!(f, "invite code invalid"),
            Self::InviteCodeUsed => write!(f, "invite code already used"),
            Self::Other(msg) => write!(f, "{}", msg),
        }
    }
}

#[async_trait::async_trait]
pub trait AuthService: Send + Sync {
    async fn create_user(
        &self,
        username: String,
        email: String,
        credentials: AuthCredentials,
    ) -> Result<User>;

    async fn authenticate(&self, username: &str, credentials: AuthCredentials) -> Result<User>;

    async fn is_authorized(&self, user_id: Uuid) -> Result<bool>;
}

pub struct InviteCodeAuthService {
    persistence: Arc<dyn UserPersistence>,
}

impl InviteCodeAuthService {
    pub fn new(persistence: Arc<dyn UserPersistence>) -> Self {
        Self { persistence }
    }

    fn extract_token(credentials: &AuthCredentials) -> Result<String, AuthError> {
        match credentials {
            AuthCredentials::Token(token) => Ok(token.clone()),
            _ => Err(AuthError::InvalidCredentials),
        }
    }

    fn current_timestamp() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }
}

#[async_trait::async_trait]
impl AuthService for InviteCodeAuthService {
    async fn create_user(
        &self,
        username: String,
        email: String,
        credentials: AuthCredentials,
    ) -> Result<User> {
        let token = Self::extract_token(&credentials)?;

        let mut invite = self.validate_invite_code(&token).await?;
        self.check_username_available(&username).await?;

        let user = self.create_and_persist_user(username, email).await?;
        self.mark_code_used(&mut invite, user.id).await?;

        Ok(user)
    }

    async fn authenticate(&self, username: &str, _credentials: AuthCredentials) -> Result<User> {
        self.persistence
            .load_user_by_username(username)
            .await
            .map_err(|e| AuthError::Persistence(e.to_string()))?
            .ok_or_else(|| AuthError::UserNotFound.into())
    }

    async fn is_authorized(&self, user_id: Uuid) -> Result<bool> {
        self.persistence
            .load(user_id)
            .await
            .map(|opt| opt.is_some())
            .map_err(|e| AuthError::Persistence(e.to_string()).into())
    }
}

impl InviteCodeAuthService {
    async fn validate_invite_code(&self, code: &str) -> Result<dialect_coach_shared::InviteCode, AuthError> {
        let invite = self.persistence
            .load_invite_code(code)
            .await
            .map_err(|e| AuthError::Persistence(e.to_string()))?
            .ok_or_else(|| AuthError::Unauthorized(UnauthorizedReason::InviteCodeInvalid))?;

        let now = Self::current_timestamp();

        if invite.is_used() {
            return Err(AuthError::Unauthorized(UnauthorizedReason::InviteCodeUsed));
        }
        if invite.is_expired(now) {
            return Err(AuthError::Unauthorized(UnauthorizedReason::InviteCodeExpired));
        }

        Ok(invite)
    }

    async fn check_username_available(&self, username: &str) -> Result<(), AuthError> {
        let existing = self.persistence
            .load_user_by_username(username)
            .await
            .map_err(|e| AuthError::Persistence(e.to_string()))?;

        if existing.is_some() {
            return Err(AuthError::UserExists);
        }
        Ok(())
    }

    async fn create_and_persist_user(&self, username: String, email: String) -> Result<User, AuthError> {
        let user = User::new(Uuid::new_v4(), username, email);
        self.persistence
            .create_user(&user)
            .await
            .map_err(|e| AuthError::Persistence(e.to_string()))?;
        Ok(user)
    }

    async fn mark_code_used(&self, invite: &mut dialect_coach_shared::InviteCode, user_id: Uuid) -> Result<(), AuthError> {
        invite.mark_used(user_id);
        self.persistence
            .save_invite_code(invite)
            .await
            .map_err(|e| AuthError::Persistence(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dialect_coach_shared::InviteCode;
    use crate::persistence::in_memory::InMemoryPersistence;

    fn test_persistence() -> Arc<dyn UserPersistence> {
        Arc::new(InMemoryPersistence::new())
    }

    #[tokio::test]
    async fn test_create_user_success() {
        let persistence = test_persistence();
        persistence.initialize().await.unwrap();

        let code = "VALIDCODE123";
        let invite = InviteCode::new(code.to_string(), None);
        persistence.create_invite_code(&invite).await.unwrap();

        let service = InviteCodeAuthService::new(persistence);
        let user = service.create_user(
            "testuser".to_string(),
            "test@example.com".to_string(),
            AuthCredentials::Token(code.to_string()),
        ).await.unwrap();

        assert_eq!(user.username, "testuser");
        assert_eq!(user.email, "test@example.com");
    }

    #[tokio::test]
    async fn test_create_user_invalid_code() {
        let service = InviteCodeAuthService::new(test_persistence());

        let result = service.create_user(
            "testuser".to_string(),
            "test@example.com".to_string(),
            AuthCredentials::Token("INVALID".to_string()),
        ).await;

        assert!(matches!(result.unwrap_err().downcast::<AuthError>().unwrap(),
            AuthError::Unauthorized(UnauthorizedReason::InviteCodeInvalid)));
    }

    #[tokio::test]
    async fn test_create_user_expired_code() {
        let persistence = test_persistence();
        persistence.initialize().await.unwrap();

        let code = "EXPIRED";
        let past = 1000000000i64;
        let invite = InviteCode::new(code.to_string(), Some(past));
        persistence.create_invite_code(&invite).await.unwrap();

        let service = InviteCodeAuthService::new(persistence);
        let result = service.create_user(
            "testuser".to_string(),
            "test@example.com".to_string(),
            AuthCredentials::Token(code.to_string()),
        ).await;

        assert!(matches!(result.unwrap_err().downcast::<AuthError>().unwrap(),
            AuthError::Unauthorized(UnauthorizedReason::InviteCodeExpired)));
    }

    #[tokio::test]
    async fn test_create_user_used_code() {
        let persistence = test_persistence();
        persistence.initialize().await.unwrap();

        let code = "USEDCODE";
        let mut invite = InviteCode::new(code.to_string(), None);
        invite.mark_used(Uuid::new_v4());
        persistence.create_invite_code(&invite).await.unwrap();

        let service = InviteCodeAuthService::new(persistence);
        let result = service.create_user(
            "testuser".to_string(),
            "test@example.com".to_string(),
            AuthCredentials::Token(code.to_string()),
        ).await;

        assert!(matches!(result.unwrap_err().downcast::<AuthError>().unwrap(),
            AuthError::Unauthorized(UnauthorizedReason::InviteCodeUsed)));
    }

    #[tokio::test]
    async fn test_create_user_duplicate_username() {
        let persistence = test_persistence();
        persistence.initialize().await.unwrap();

        let code1 = "CODE1";
        let code2 = "CODE2";
        persistence.create_invite_code(&InviteCode::new(code1.to_string(), None)).await.unwrap();
        persistence.create_invite_code(&InviteCode::new(code2.to_string(), None)).await.unwrap();

        let service = InviteCodeAuthService::new(persistence);
        service.create_user(
            "testuser".to_string(),
            "test1@example.com".to_string(),
            AuthCredentials::Token(code1.to_string()),
        ).await.unwrap();

        let result = service.create_user(
            "testuser".to_string(),
            "test2@example.com".to_string(),
            AuthCredentials::Token(code2.to_string()),
        ).await;

        assert!(matches!(result.unwrap_err().downcast::<AuthError>().unwrap(), AuthError::UserExists));
    }

    #[tokio::test]
    async fn test_create_user_wrong_credential_type() {
        let service = InviteCodeAuthService::new(test_persistence());

        let result = service.create_user(
            "testuser".to_string(),
            "test@example.com".to_string(),
            AuthCredentials::Password("password".to_string()),
        ).await;

        assert!(matches!(result.unwrap_err().downcast::<AuthError>().unwrap(), AuthError::InvalidCredentials));
    }

    #[tokio::test]
    async fn test_authenticate_success() {
        let persistence = test_persistence();
        persistence.initialize().await.unwrap();

        let user = User::new(Uuid::new_v4(), "testuser".to_string(), "test@example.com".to_string());
        persistence.create_user(&user).await.unwrap();

        let service = InviteCodeAuthService::new(persistence);
        let result = service.authenticate("testuser", AuthCredentials::Token("any".to_string())).await.unwrap();

        assert_eq!(result.username, "testuser");
    }

    #[tokio::test]
    async fn test_authenticate_not_found() {
        let service = InviteCodeAuthService::new(test_persistence());

        let result = service.authenticate("nonexistent", AuthCredentials::Token("any".to_string())).await;

        assert!(matches!(result.unwrap_err().downcast::<AuthError>().unwrap(), AuthError::UserNotFound));
    }

    #[tokio::test]
    async fn test_is_authorized_true() {
        let persistence = test_persistence();
        persistence.initialize().await.unwrap();

        let user_id = Uuid::new_v4();
        let state = dialect_coach_shared::UserState::new(user_id);
        persistence.save(&state).await.unwrap();

        let service = InviteCodeAuthService::new(persistence);
        let result = service.is_authorized(user_id).await.unwrap();

        assert!(result);
    }

    #[tokio::test]
    async fn test_is_authorized_false() {
        let service = InviteCodeAuthService::new(test_persistence());

        let result = service.is_authorized(Uuid::new_v4()).await.unwrap();

        assert!(!result);
    }

    #[test]
    fn test_auth_credentials_password() {
        let creds = AuthCredentials::Password("secret123".to_string());
        match creds {
            AuthCredentials::Password(p) => assert_eq!(p, "secret123"),
            _ => panic!("Expected Password variant"),
        }
    }

    #[test]
    fn test_auth_credentials_token() {
        let creds = AuthCredentials::Token("abc123".to_string());
        match creds {
            AuthCredentials::Token(t) => assert_eq!(t, "abc123"),
            _ => panic!("Expected Token variant"),
        }
    }

    #[test]
    fn test_unauthorized_reason_display() {
        assert_eq!(UnauthorizedReason::InviteCodeExpired.to_string(), "invite code expired");
        assert_eq!(UnauthorizedReason::InviteCodeInvalid.to_string(), "invite code invalid");
        assert_eq!(UnauthorizedReason::InviteCodeUsed.to_string(), "invite code already used");
        assert_eq!(UnauthorizedReason::Other("custom".to_string()).to_string(), "custom");
    }
}
