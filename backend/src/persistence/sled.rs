use anyhow::{anyhow, Result};
use dialect_coach_shared::{User, UserState};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use super::UserPersistence;

pub struct SledPersistence {
    db: sled::Db,
}

impl SledPersistence {
    pub fn new(path: &str) -> Result<Self> {
        tracing::info!("Initializing sled database at: {}", path);
        let db = sled::open(path)?;
        Ok(Self { db })
    }

    fn users_tree(&self) -> Result<sled::Tree> {
        Ok(self.db.open_tree("users")?)
    }

    fn user_states_tree(&self) -> Result<sled::Tree> {
        Ok(self.db.open_tree("user_states")?)
    }
}

fn serialize_to_json<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec(value)?)
}

fn deserialize_from_json<T: for<'de> Deserialize<'de>>(bytes: &[u8]) -> Result<T> {
    Ok(serde_json::from_slice(bytes)?)
}

#[async_trait::async_trait]
impl UserPersistence for SledPersistence {
    async fn initialize(&self) -> Result<()> {
        tracing::info!("Sled persistence initialized");
        Ok(())
    }

    async fn save(&self, user_state: &UserState) -> Result<()> {
        let tree = self.user_states_tree()?;
        let key = user_state.user_id.to_string();
        let value = serialize_to_json(user_state)?;
        tree.insert(key.as_bytes(), value)?;
        tracing::debug!("Saved user state to sled: {}", user_state.user_id);
        Ok(())
    }

    async fn load(&self, user_id: Uuid) -> Result<Option<UserState>> {
        let tree = self.user_states_tree()?;
        let key = user_id.to_string();
        let result = tree.get(key.as_bytes())?
            .map(|bytes| deserialize_from_json(&bytes))
            .transpose()?;
        tracing::debug!("Loaded user state from sled: {} (found: {})", user_id, result.is_some());
        Ok(result)
    }

    async fn create_user(&self, user: &User) -> Result<()> {
        if user.username.trim().is_empty() {
            return Err(anyhow!("Username cannot be empty"));
        }

        let tree = self.users_tree()?;
        if tree.contains_key(user.username.as_bytes())? {
            return Err(anyhow!("Username already exists"));
        }

        let value = serialize_to_json(user)?;
        tree.insert(user.username.as_bytes(), value)?;
        tracing::debug!("Created user: {}", user.username);
        Ok(())
    }

    async fn load_user_by_username(&self, username: &str) -> Result<Option<User>> {
        let tree = self.users_tree()?;
        let result = tree.get(username.as_bytes())?
            .map(|bytes| deserialize_from_json(&bytes))
            .transpose()?;
        tracing::debug!("Loaded user by username: {} (found: {})", username, result.is_some());
        Ok(result)
    }
}
