use super::UserPersistence;
use anyhow::{Result, anyhow};
use dialect_coach_shared::{
    CURRENT_USER_STATE_VERSION, InviteCode, UsageStats, User, UserState, UserStateV1,
    UserStateVersion, VersionedData, migrate_user_state_to_current,
};
use serde_json::Value as JsonValue;
use sqlx::Row;
use uuid::Uuid;

pub struct PostgresPersistence {
    pool: sqlx::PgPool,
}

impl PostgresPersistence {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }
}

fn serialize_to_json<T: serde::Serialize>(value: &T) -> Result<JsonValue> {
    Ok(serde_json::to_value(value)?)
}

fn deserialize_from_json<T: serde::de::DeserializeOwned>(value: &JsonValue) -> Result<T> {
    Ok(serde_json::from_value(value.clone())?)
}

fn serialize_versioned_user_state(state: &UserState) -> Result<JsonValue> {
    let v1_data = UserStateV1::from(state.clone());
    let wrapper = VersionedData {
        version: CURRENT_USER_STATE_VERSION,
        data: serde_json::to_value(v1_data)?,
    };
    Ok(serde_json::to_value(wrapper)?)
}

fn deserialize_versioned_user_state(data: &JsonValue) -> Result<UserState> {
    let wrapper: VersionedData<UserStateVersion> = serde_json::from_value(data.clone())?;
    let v1_data: UserStateV1 = serde_json::from_value(wrapper.data)?;
    Ok(migrate_user_state_to_current(wrapper.version, v1_data))
}

fn parse_user_from_row(r: &sqlx::postgres::PgRow) -> User {
    let id: Uuid = r.get("id");
    let username: String = r.get("username");
    let email: Option<String> = r.get("email");
    User {
        id,
        username,
        email: email.unwrap_or_default(),
    }
}

fn parse_invite_code_from_row(r: &sqlx::postgres::PgRow) -> InviteCode {
    let code: String = r.get("code");
    let created_date: i64 = r.get("created_date");
    let used_by: Option<Uuid> = r.get("used_by");
    let expiration: Option<i64> = r.get("expiration");
    InviteCode {
        code,
        created_date,
        used_by,
        expiration,
    }
}

#[async_trait::async_trait]
impl UserPersistence for PostgresPersistence {
    async fn initialize(&self) -> Result<()> {
        tracing::info!("PostgreSQL persistence initialized");
        Ok(())
    }

    async fn save(&self, user_state: &UserState) -> Result<()> {
        let mut clean = user_state.clone();
        clean.usage_stats = UsageStats::default();

        let user_id = user_state.user_id;
        let data = serialize_versioned_user_state(&clean)?;

        sqlx::query(
            "INSERT INTO user_states (user_id, data, version) VALUES ($1, $2, 'V1')
             ON CONFLICT (user_id) DO UPDATE SET data = $2, version = 'V1', updated_at = NOW()",
        )
        .bind(user_id)
        .bind(&data)
        .execute(&self.pool)
        .await?;

        tracing::debug!("Saved user state to postgres: {}", user_id);
        Ok(())
    }

    async fn load(&self, user_id: Uuid) -> Result<Option<UserState>> {
        let row = sqlx::query("SELECT data FROM user_states WHERE user_id = $1")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?;

        let mut result = match row {
            Some(r) => {
                let data: JsonValue = r.get("data");
                Some(deserialize_versioned_user_state(&data)?)
            }
            None => None,
        };

        if let Some(ref mut state) = result
            && let Some(stats) = self.load_usage_stats(user_id).await?
        {
            state.usage_stats = stats;
        }

        tracing::debug!(
            "Loaded user state from postgres: {} (found: {})",
            user_id,
            result.is_some()
        );
        Ok(result)
    }

    async fn create_user(&self, user: &User, password_hash: String) -> Result<()> {
        if user.username.trim().is_empty() {
            return Err(anyhow!("Username cannot be empty"));
        }

        let result = sqlx::query(
            "INSERT INTO users (id, username, email, password_hash) VALUES ($1, $2, $3, $4)",
        )
        .bind(user.id)
        .bind(&user.username)
        .bind(&user.email)
        .bind(&password_hash)
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => {
                tracing::debug!("Created user: {}", user.username);
                Ok(())
            }
            Err(e) => {
                let msg = if e.to_string().contains("unique constraint") {
                    "Username already exists"
                } else {
                    "Failed to create user"
                };
                Err(anyhow!(msg))
            }
        }
    }

    async fn load_user_by_username(&self, username: &str) -> Result<Option<(User, String)>> {
        let row =
            sqlx::query("SELECT id, username, email, password_hash FROM users WHERE username = $1")
                .bind(username)
                .fetch_optional(&self.pool)
                .await?;

        let result = row.map(|r| {
            let user = parse_user_from_row(&r);
            let password_hash: String = r.get("password_hash");
            (user, password_hash)
        });

        tracing::debug!(
            "Loaded user by username: {} (found: {})",
            username,
            result.is_some()
        );
        Ok(result)
    }

    async fn load_user_by_id(&self, user_id: Uuid) -> Result<Option<User>> {
        let row = sqlx::query("SELECT id, username, email FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?;

        let result = row.map(|r| parse_user_from_row(&r));

        tracing::debug!(
            "Loaded user by ID: {} (found: {})",
            user_id,
            result.is_some()
        );
        Ok(result)
    }

    async fn save_usage_stats(&self, user_id: Uuid, usage_stats: &UsageStats) -> Result<()> {
        let data = serialize_to_json(usage_stats)?;

        sqlx::query(
            "INSERT INTO usage_stats (user_id, data) VALUES ($1, $2)
             ON CONFLICT (user_id) DO UPDATE SET data = $2, updated_at = NOW()",
        )
        .bind(user_id)
        .bind(&data)
        .execute(&self.pool)
        .await?;

        tracing::debug!("Saved usage stats to postgres: {}", user_id);
        Ok(())
    }

    async fn load_usage_stats(&self, user_id: Uuid) -> Result<Option<UsageStats>> {
        let row = sqlx::query("SELECT data FROM usage_stats WHERE user_id = $1")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?;

        let result = row
            .map(|r| {
                let data: JsonValue = r.get("data");
                deserialize_from_json(&data)
            })
            .transpose()?;

        tracing::debug!(
            "Loaded usage stats from postgres: {} (found: {})",
            user_id,
            result.is_some()
        );
        Ok(result)
    }

    async fn create_invite_code(&self, invite_code: &InviteCode) -> Result<()> {
        sqlx::query(
            "INSERT INTO invite_codes (code, created_date, used_by, expiration) VALUES ($1, $2, $3, $4)"
        )
        .bind(&invite_code.code)
        .bind(invite_code.created_date)
        .bind(invite_code.used_by)
        .bind(invite_code.expiration)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn load_invite_code(&self, code: &str) -> Result<Option<InviteCode>> {
        tracing::info!("Loading invite code: '{}'", code);

        let row = sqlx::query(
            "SELECT code, created_date, used_by, expiration FROM invite_codes WHERE code = $1",
        )
        .bind(code)
        .fetch_optional(&self.pool)
        .await?;

        let result = row.map(|r| parse_invite_code_from_row(&r));
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
        sqlx::query(
            "INSERT INTO invite_codes (code, created_date, used_by, expiration) VALUES ($1, $2, $3, $4)
             ON CONFLICT (code) DO UPDATE SET used_by = $3, expiration = $4"
        )
        .bind(&invite_code.code)
        .bind(invite_code.created_date)
        .bind(invite_code.used_by)
        .bind(invite_code.expiration)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn list_invite_codes(&self) -> Result<Vec<InviteCode>> {
        let rows = sqlx::query("SELECT code, created_date, used_by, expiration FROM invite_codes")
            .fetch_all(&self.pool)
            .await?;

        Ok(rows
            .into_iter()
            .map(|r| parse_invite_code_from_row(&r))
            .collect())
    }

    async fn delete_invite_code(&self, code: &str) -> Result<()> {
        sqlx::query("DELETE FROM invite_codes WHERE code = $1")
            .bind(code)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
