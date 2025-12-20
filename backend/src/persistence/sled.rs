use super::{UserPersistence, UserRecord};
use anyhow::{Result, anyhow};
use dialect_coach_shared::{
    CURRENT_USER_STATE_VERSION, CURRENT_USER_VERSION, InviteCode, UsageStats, User, UserState,
    UserStateV1, UserStateVersion, UserVersion, VersionedData, migrate_user_state_to_current,
    migrate_user_to_current,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub struct SledPersistence {
    db: sled::Db,
}

impl SledPersistence {
    pub fn new(path: &str) -> Result<Self> {
        let abs_path = std::fs::canonicalize(std::path::Path::new(path).parent().unwrap())
            .unwrap_or_else(|_| std::path::PathBuf::from(path));
        tracing::info!(
            "Initializing sled database at: {} (resolved: {:?})",
            path,
            abs_path
        );
        let db = sled::open(path)?;
        Ok(Self { db })
    }

    fn users_tree(&self) -> Result<sled::Tree> {
        Ok(self.db.open_tree("users")?)
    }

    fn user_states_tree(&self) -> Result<sled::Tree> {
        Ok(self.db.open_tree("user_states")?)
    }

    fn usage_stats_tree(&self) -> Result<sled::Tree> {
        Ok(self.db.open_tree("usage_stats")?)
    }

    fn invite_codes_tree(&self) -> Result<sled::Tree> {
        Ok(self.db.open_tree("invite_codes")?)
    }
}

fn serialize_to_json<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec(value)?)
}

fn deserialize_from_json<T: for<'de> Deserialize<'de>>(bytes: &[u8]) -> Result<T> {
    Ok(serde_json::from_slice(bytes)?)
}

fn serialize_versioned_user_state(state: &UserState) -> Result<Vec<u8>> {
    let v1_data = UserStateV1::from(state.clone());
    let wrapper = VersionedData {
        version: CURRENT_USER_STATE_VERSION,
        data: serde_json::to_value(v1_data)?,
    };
    Ok(serde_json::to_vec(&wrapper)?)
}

fn deserialize_versioned_user_state(bytes: &[u8]) -> Result<UserState> {
    let wrapper: VersionedData<UserStateVersion> = serde_json::from_slice(bytes)?;
    let v1_data: UserStateV1 = serde_json::from_value(wrapper.data)?;
    Ok(migrate_user_state_to_current(wrapper.version, v1_data))
}

fn serialize_versioned_user_record(user: &User, password_hash: &str) -> Result<Vec<u8>> {
    let record = UserRecord {
        user: user.clone(),
        password_hash: password_hash.to_string(),
    };
    let wrapper = VersionedData {
        version: CURRENT_USER_VERSION,
        data: serde_json::to_value(record)?,
    };
    Ok(serde_json::to_vec(&wrapper)?)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RawUserRecord {
    user: serde_json::Value,
    password_hash: String,
}

fn deserialize_versioned_user_record(bytes: &[u8]) -> Result<(User, String)> {
    let wrapper: VersionedData<UserVersion> = serde_json::from_slice(bytes)?;
    let record: RawUserRecord = serde_json::from_value(wrapper.data)?;
    let user = migrate_user_to_current(wrapper.version, record.user);
    Ok((user, record.password_hash))
}

#[async_trait::async_trait]
impl UserPersistence for SledPersistence {
    async fn initialize(&self) -> Result<()> {
        tracing::info!("Sled persistence initialized");
        Ok(())
    }

    async fn save(&self, user_state: &UserState) -> Result<()> {
        let mut clean = user_state.clone();
        clean.usage_stats = UsageStats::default();

        let tree = self.user_states_tree()?;
        let key = clean.user_id.to_string();
        let value = serialize_versioned_user_state(&clean)?;
        tree.insert(key.as_bytes(), value)?;

        // DEBUG: Log plans written to disk
        for plan in &clean.language_plans {
            tracing::info!("Sled persisting plan: {} (id: {})", plan.title, plan.id);
        }

        tracing::debug!("Saved user state to sled: {}", clean.user_id);
        Ok(())
    }

    async fn load(&self, user_id: Uuid) -> Result<Option<UserState>> {
        let tree = self.user_states_tree()?;
        let key = user_id.to_string();
        let mut result: Option<UserState> = tree
            .get(key.as_bytes())?
            .map(|bytes| deserialize_versioned_user_state(&bytes))
            .transpose()?;

        if let Some(ref mut state) = result
            && let Some(stats) = self.load_usage_stats(user_id).await?
        {
            state.usage_stats = stats;
        }

        tracing::debug!(
            "Loaded user state from sled: {} (found: {})",
            user_id,
            result.is_some()
        );
        Ok(result)
    }

    async fn create_user(&self, user: &User, password_hash: String) -> Result<()> {
        if user.username.trim().is_empty() {
            return Err(anyhow!("Username cannot be empty"));
        }

        let tree = self.users_tree()?;
        if tree.contains_key(user.username.as_bytes())? {
            return Err(anyhow!("Username already exists"));
        }

        let value = serialize_versioned_user_record(user, &password_hash)?;
        tree.insert(user.username.as_bytes(), value)?;
        tracing::debug!("Created user: {}", user.username);
        Ok(())
    }

    async fn load_user_by_username(&self, username: &str) -> Result<Option<(User, String)>> {
        let tree = self.users_tree()?;
        let result = tree
            .get(username.as_bytes())?
            .map(|bytes| deserialize_versioned_user_record(&bytes))
            .transpose()?;
        tracing::debug!(
            "Loaded user by username: {} (found: {})",
            username,
            result.is_some()
        );
        Ok(result)
    }

    async fn load_user_by_id(&self, user_id: Uuid) -> Result<Option<User>> {
        let tree = self.users_tree()?;
        for item in tree.iter() {
            let (_key, bytes) = item?;
            let (user, _) = deserialize_versioned_user_record(&bytes)?;
            if user.id == user_id {
                tracing::debug!("Loaded user by ID: {} (found)", user_id);
                return Ok(Some(user));
            }
        }
        tracing::debug!("Loaded user by ID: {} (not found)", user_id);
        Ok(None)
    }

    async fn save_usage_stats(&self, user_id: Uuid, usage_stats: &UsageStats) -> Result<()> {
        let tree = self.usage_stats_tree()?;
        let key = user_id.to_string();
        let value = serialize_to_json(usage_stats)?;
        tree.insert(key.as_bytes(), value)?;
        tracing::debug!("Saved usage stats to sled: {}", user_id);
        Ok(())
    }

    async fn load_usage_stats(&self, user_id: Uuid) -> Result<Option<UsageStats>> {
        let tree = self.usage_stats_tree()?;
        let key = user_id.to_string();
        let result = tree
            .get(key.as_bytes())?
            .map(|bytes| deserialize_from_json(&bytes))
            .transpose()?;
        tracing::debug!(
            "Loaded usage stats from sled: {} (found: {})",
            user_id,
            result.is_some()
        );
        Ok(result)
    }

    async fn create_invite_code(&self, invite_code: &InviteCode) -> Result<()> {
        let tree = self.invite_codes_tree()?;
        let value = serialize_to_json(invite_code)?;
        tree.insert(invite_code.code.as_bytes(), value)?;
        Ok(())
    }

    async fn load_invite_code(&self, code: &str) -> Result<Option<InviteCode>> {
        let tree = self.invite_codes_tree()?;
        tracing::info!(
            "Loading invite code: '{}' (len: {}, bytes: {:?})",
            code,
            code.len(),
            code.as_bytes()
        );

        // Debug: List all codes in tree
        tracing::info!("All codes in database:");
        for (key, _) in tree.iter().flatten() {
            let key_str = String::from_utf8_lossy(&key);
            tracing::info!(
                "  - '{}' (len: {}, bytes: {:?})",
                key_str,
                key.len(),
                &key[..]
            );
        }

        let result = tree
            .get(code.as_bytes())?
            .map(|bytes| deserialize_from_json(&bytes))
            .transpose()?;

        tracing::info!(
            "Load result: {}",
            if result.is_some() {
                "FOUND"
            } else {
                "NOT FOUND"
            }
        );
        Ok(result)
    }

    async fn save_invite_code(&self, invite_code: &InviteCode) -> Result<()> {
        let tree = self.invite_codes_tree()?;
        let value = serialize_to_json(invite_code)?;
        tree.insert(invite_code.code.as_bytes(), value)?;
        Ok(())
    }

    async fn list_invite_codes(&self) -> Result<Vec<InviteCode>> {
        let tree = self.invite_codes_tree()?;
        let mut codes = Vec::new();
        for item in tree.iter() {
            let (_key, bytes) = item?;
            let code: InviteCode = deserialize_from_json(&bytes)?;
            codes.push(code);
        }
        Ok(codes)
    }

    async fn delete_invite_code(&self, code: &str) -> Result<()> {
        let tree = self.invite_codes_tree()?;
        tree.remove(code.as_bytes())?;
        Ok(())
    }
}
