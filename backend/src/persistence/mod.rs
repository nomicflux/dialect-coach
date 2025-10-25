use anyhow::Result;
use dialect_coach_shared::UserState;
use uuid::Uuid;

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
}
