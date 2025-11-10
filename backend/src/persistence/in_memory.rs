#![allow(dead_code)]

use super::UserPersistence;
use anyhow::{Result, anyhow};
use dialect_coach_shared::{InviteCode, UsageStats, User, UserState};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

/// In-memory implementation of UserPersistence for development and testing
///
/// This implementation stores user state in a HashMap in memory. Data is lost
/// when the server restarts. This is suitable for development but not for production.
pub struct InMemoryPersistence {
    state: Arc<Mutex<HashMap<Uuid, UserState>>>,
    users: Arc<Mutex<HashMap<String, User>>>,
    usage_stats: Arc<Mutex<HashMap<Uuid, UsageStats>>>,
    invite_codes: Arc<Mutex<HashMap<String, InviteCode>>>,
}

impl InMemoryPersistence {
    /// Create a new in-memory persistence instance
    pub fn new() -> Self {
        tracing::info!("Initializing in-memory user state storage");
        Self {
            state: Arc::new(Mutex::new(HashMap::new())),
            users: Arc::new(Mutex::new(HashMap::new())),
            usage_stats: Arc::new(Mutex::new(HashMap::new())),
            invite_codes: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryPersistence {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl UserPersistence for InMemoryPersistence {
    async fn initialize(&self) -> Result<()> {
        tracing::info!("In-memory persistence initialized (no-op)");
        Ok(())
    }

    async fn save(&self, user_state: &UserState) -> Result<()> {
        let mut clean = user_state.clone();
        clean.usage_stats = UsageStats::default();

        let mut state = self.state.lock().await;
        state.insert(clean.user_id, clean);
        tracing::debug!(
            "Saved user state to in-memory storage: {}",
            user_state.user_id
        );
        Ok(())
    }

    async fn load(&self, user_id: Uuid) -> Result<Option<UserState>> {
        let state = self.state.lock().await;
        let mut result = state.get(&user_id).cloned();

        if let Some(ref mut s) = result {
            drop(state);
            if let Some(stats) = self.load_usage_stats(user_id).await? {
                s.usage_stats = stats;
            }
        }

        tracing::debug!(
            "Loaded user state from in-memory storage: {} (found: {})",
            user_id,
            result.is_some()
        );
        Ok(result)
    }

    async fn create_user(&self, user: &User) -> Result<()> {
        if user.username.trim().is_empty() {
            return Err(anyhow!("Username cannot be empty"));
        }

        let mut users = self.users.lock().await;
        if users.contains_key(&user.username) {
            return Err(anyhow!("Username already exists"));
        }

        users.insert(user.username.clone(), user.clone());
        tracing::debug!("Created user: {}", user.username);
        Ok(())
    }

    async fn load_user_by_username(&self, username: &str) -> Result<Option<User>> {
        let users = self.users.lock().await;
        let result = users.get(username).cloned();
        tracing::debug!(
            "Loaded user by username: {} (found: {})",
            username,
            result.is_some()
        );
        Ok(result)
    }

    async fn save_usage_stats(&self, user_id: Uuid, usage_stats: &UsageStats) -> Result<()> {
        let mut stats = self.usage_stats.lock().await;
        stats.insert(user_id, usage_stats.clone());
        tracing::debug!("Saved usage stats to in-memory storage: {}", user_id);
        Ok(())
    }

    async fn load_usage_stats(&self, user_id: Uuid) -> Result<Option<UsageStats>> {
        let stats = self.usage_stats.lock().await;
        let result = stats.get(&user_id).cloned();
        tracing::debug!(
            "Loaded usage stats from in-memory storage: {} (found: {})",
            user_id,
            result.is_some()
        );
        Ok(result)
    }

    async fn create_invite_code(&self, invite_code: &InviteCode) -> Result<()> {
        let mut codes = self.invite_codes.lock().await;
        codes.insert(invite_code.code.clone(), invite_code.clone());
        Ok(())
    }

    async fn load_invite_code(&self, code: &str) -> Result<Option<InviteCode>> {
        let codes = self.invite_codes.lock().await;
        Ok(codes.get(code).cloned())
    }

    async fn save_invite_code(&self, invite_code: &InviteCode) -> Result<()> {
        let mut codes = self.invite_codes.lock().await;
        codes.insert(invite_code.code.clone(), invite_code.clone());
        Ok(())
    }

    async fn list_invite_codes(&self) -> Result<Vec<InviteCode>> {
        let codes = self.invite_codes.lock().await;
        Ok(codes.values().cloned().collect())
    }

    async fn delete_invite_code(&self, code: &str) -> Result<()> {
        let mut codes = self.invite_codes.lock().await;
        codes.remove(code);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_save_and_load() {
        let persistence = InMemoryPersistence::new();
        persistence.initialize().await.unwrap();

        let user_id = Uuid::new_v4();
        let user_state = UserState::new(user_id);

        // Save state
        persistence.save(&user_state).await.unwrap();

        // Load state
        let loaded = persistence.load(user_id).await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().user_id, user_id);
    }

    #[tokio::test]
    async fn test_load_nonexistent() {
        let persistence = InMemoryPersistence::new();
        persistence.initialize().await.unwrap();

        let user_id = Uuid::new_v4();
        let loaded = persistence.load(user_id).await.unwrap();
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_save_replaces_existing() {
        let persistence = InMemoryPersistence::new();
        persistence.initialize().await.unwrap();

        let user_id = Uuid::new_v4();
        let mut user_state = UserState::new(user_id);

        // Save initial state
        persistence.save(&user_state).await.unwrap();

        // Modify and save again
        user_state.tts_enabled = true;
        persistence.save(&user_state).await.unwrap();

        // Load and verify
        let loaded = persistence.load(user_id).await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().tts_enabled, true);
    }

    #[tokio::test]
    async fn test_multiple_users() {
        let persistence = InMemoryPersistence::new();
        persistence.initialize().await.unwrap();

        let user_id_1 = Uuid::new_v4();
        let user_id_2 = Uuid::new_v4();
        let state_1 = UserState::new(user_id_1);
        let state_2 = UserState::new(user_id_2);

        // Save both
        persistence.save(&state_1).await.unwrap();
        persistence.save(&state_2).await.unwrap();

        // Load both
        let loaded_1 = persistence.load(user_id_1).await.unwrap();
        let loaded_2 = persistence.load(user_id_2).await.unwrap();

        assert!(loaded_1.is_some());
        assert!(loaded_2.is_some());
        assert_eq!(loaded_1.unwrap().user_id, user_id_1);
        assert_eq!(loaded_2.unwrap().user_id, user_id_2);
    }

    #[tokio::test]
    async fn test_create_user_success() {
        let persistence = InMemoryPersistence::new();
        persistence.initialize().await.unwrap();

        let user = User::new(
            Uuid::new_v4(),
            "testuser".to_string(),
            "test@example.com".to_string(),
        );
        let result = persistence.create_user(&user).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_user_duplicate_username() {
        let persistence = InMemoryPersistence::new();
        persistence.initialize().await.unwrap();

        let user1 = User::new(
            Uuid::new_v4(),
            "duplicate".to_string(),
            "user1@example.com".to_string(),
        );
        let user2 = User::new(
            Uuid::new_v4(),
            "duplicate".to_string(),
            "user2@example.com".to_string(),
        );

        persistence.create_user(&user1).await.unwrap();
        let result = persistence.create_user(&user2).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[tokio::test]
    async fn test_load_user_by_username_found() {
        let persistence = InMemoryPersistence::new();
        persistence.initialize().await.unwrap();

        let user_id = Uuid::new_v4();
        let user = User::new(
            user_id,
            "alice".to_string(),
            "alice@example.com".to_string(),
        );
        persistence.create_user(&user).await.unwrap();

        let loaded = persistence.load_user_by_username("alice").await.unwrap();
        assert!(loaded.is_some());
        let loaded_user = loaded.unwrap();
        assert_eq!(loaded_user.id, user_id);
        assert_eq!(loaded_user.username, "alice");
    }

    #[tokio::test]
    async fn test_load_user_by_username_not_found() {
        let persistence = InMemoryPersistence::new();
        persistence.initialize().await.unwrap();

        let loaded = persistence
            .load_user_by_username("nonexistent")
            .await
            .unwrap();
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_create_user_empty_username() {
        let persistence = InMemoryPersistence::new();
        persistence.initialize().await.unwrap();

        let user = User::new(
            Uuid::new_v4(),
            "".to_string(),
            "test@example.com".to_string(),
        );
        let result = persistence.create_user(&user).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[tokio::test]
    async fn test_save_and_load_usage_stats() {
        let persistence = InMemoryPersistence::new();
        persistence.initialize().await.unwrap();

        let user_id = Uuid::new_v4();
        let mut stats = UsageStats::default();
        stats
            .response_events
            .push(dialect_coach_shared::models::AgentUsage {
                timestamp: 1000,
                input_tokens: 100,
                output_tokens: 50,
                is_retry: false,
                is_estimate: false,
                provider: "anthropic".to_string(),
                model: "claude-test".to_string(),
            });

        persistence.save_usage_stats(user_id, &stats).await.unwrap();

        let loaded = persistence.load_usage_stats(user_id).await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().response_count(), 1);
    }

    #[tokio::test]
    async fn test_load_usage_stats_nonexistent() {
        let persistence = InMemoryPersistence::new();
        persistence.initialize().await.unwrap();

        let user_id = Uuid::new_v4();
        let loaded = persistence.load_usage_stats(user_id).await.unwrap();
        assert!(loaded.is_none());
    }
}
