pub mod in_memory;
pub mod sled;

use anyhow::Result;
use dialect_coach_shared::{InviteCode, UsageStats, User, UserState};
use uuid::Uuid;

pub use sled::SledPersistence;

pub fn get_db_path() -> String {
    std::env::var("DB_PATH").unwrap_or_else(|_| {
        let workspace_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("Failed to get workspace root")
            .to_path_buf();
        workspace_root
            .join("data")
            .join("dialect-coach.db")
            .to_str()
            .expect("Invalid path")
            .to_string()
    })
}

/// Trait for persisting user state
///
/// Implementations can use any storage backend and handle their own interior mutability.
/// This trait is designed to be used with `Arc<dyn UserPersistence>` for shared access
/// across async tasks.
///
/// # Example
///
/// ```ignore
/// async fn example(persistence: Arc<dyn UserPersistence>) -> Result<()> {
///     // Initialize storage
///     persistence.initialize().await?;
///
///     // Save user state
///     let user_state = UserState::new(user_id);
///     persistence.save(&user_state).await?;
///
///     // Load user state
///     if let Some(state) = persistence.load(user_id).await? {
///         println!("Loaded state for user: {}", state.user_id);
///     }
///
///     Ok(())
/// }
/// ```
#[async_trait::async_trait]
pub trait UserPersistence: Send + Sync {
    /// Initialize the persistence layer
    ///
    /// Called once at startup to set up storage. Implementations may use this to:
    /// - Load data from disk
    /// - Connect to database
    /// - Verify storage is accessible
    /// - Perform migrations
    ///
    /// # Errors
    ///
    /// Returns an error if initialization fails (e.g., cannot access storage)
    async fn initialize(&self) -> Result<()>;

    /// Save user state
    ///
    /// Saves the complete user state. If a state for this user already exists,
    /// it should be replaced.
    ///
    /// # Arguments
    ///
    /// * `user_state` - The user state to save
    ///
    /// # Errors
    ///
    /// Returns an error if the save operation fails
    async fn save(&self, user_state: &UserState) -> Result<()>;

    /// Load user state by user ID
    ///
    /// # Arguments
    ///
    /// * `user_id` - The UUID of the user to load
    ///
    /// # Returns
    ///
    /// - `Ok(Some(UserState))` if the user exists
    /// - `Ok(None)` if the user does not exist
    /// - `Err(_)` if the load operation fails
    ///
    /// # Errors
    ///
    /// Returns an error if the load operation fails (but not if user is not found)
    async fn load(&self, user_id: Uuid) -> Result<Option<UserState>>;

    /// Create a new user
    ///
    /// Saves the user to persistent storage. If a user with this username already exists,
    /// should return an error.
    ///
    /// # Arguments
    ///
    /// * `user` - The user to create
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - A user with this username already exists
    /// - The username is empty
    /// - The create operation fails
    async fn create_user(&self, user: &User) -> Result<()>;

    /// Load user by username
    ///
    /// # Arguments
    ///
    /// * `username` - The username to search for
    ///
    /// # Returns
    ///
    /// - `Ok(Some(User))` if the user exists
    /// - `Ok(None)` if the user does not exist
    /// - `Err(_)` if the load operation fails
    ///
    /// # Errors
    ///
    /// Returns an error if the load operation fails (but not if user is not found)
    async fn load_user_by_username(&self, username: &str) -> Result<Option<User>>;

    /// Save usage stats for a user
    ///
    /// Saves only the usage statistics separately from UserState.
    /// This is called frequently during API operations to track usage.
    ///
    /// # Arguments
    ///
    /// * `user_id` - The UUID of the user
    /// * `usage_stats` - The usage statistics to save
    ///
    /// # Errors
    ///
    /// Returns an error if the save operation fails
    async fn save_usage_stats(&self, user_id: Uuid, usage_stats: &UsageStats) -> Result<()>;

    /// Load usage stats for a user
    ///
    /// # Arguments
    ///
    /// * `user_id` - The UUID of the user
    ///
    /// # Returns
    ///
    /// - `Ok(Some(UsageStats))` if stats exist
    /// - `Ok(None)` if no stats exist (return default)
    /// - `Err(_)` if the load operation fails
    ///
    /// # Errors
    ///
    /// Returns an error if the load operation fails (but not if stats don't exist)
    async fn load_usage_stats(&self, user_id: Uuid) -> Result<Option<UsageStats>>;

    async fn create_invite_code(&self, invite_code: &InviteCode) -> Result<()>;

    async fn load_invite_code(&self, code: &str) -> Result<Option<InviteCode>>;

    async fn save_invite_code(&self, invite_code: &InviteCode) -> Result<()>;

    async fn list_invite_codes(&self) -> Result<Vec<InviteCode>>;

    async fn delete_invite_code(&self, code: &str) -> Result<()>;
}
