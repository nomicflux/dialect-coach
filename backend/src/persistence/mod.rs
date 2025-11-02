pub mod in_memory;
pub mod sled;

use anyhow::Result;
use dialect_coach_shared::{User, UserState};
use uuid::Uuid;

pub use sled::SledPersistence;

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
}
