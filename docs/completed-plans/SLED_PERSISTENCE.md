# SLED PERSISTENCE IMPLEMENTATION PLAN

## Overview

Replace InMemoryPersistence with sled-based persistent storage for Users and UserState. Sled is an embedded Rust key-value store that provides ACID transactions and prepares for future migration to serverless NoSQL databases (DynamoDB/Firestore).

## Key Requirements

1. **Drop-in replacement**: Implement existing UserPersistence trait
2. **Lightweight**: Single Rust dependency, no external services
3. **Persistent**: Data survives backend restarts
4. **ACID transactions**: Data safety guaranteed
5. **Cloud-ready**: Pattern matches serverless NoSQL for future migration

## Data Model

```
users/{username} -> User (JSON serialized)
user_states/{user_id} -> UserState (JSON serialized)
```

**Keys:**
- Users indexed by username (for sign-in lookups)
- UserStates indexed by user_id (UUID)

**Storage location:** `./data/dialect-coach.db` (configurable)

---

## Phase 1: Add Sled Dependency

### Status: Completed

### Code Style Guidelines
- Keep all functions <20 lines (prefer <10)
- Use helper functions for complex logic
- Follow existing error handling patterns
- No defensive coding - trust the types

### Files to Modify:
- `backend/Cargo.toml` - Add sled dependency

### Tasks:
- [x] Add `sled = "0.34"` to dependencies section
- [x] Run `cargo check` to verify dependency resolution
- [x] Update planning doc with any issues

### Implementation:
```toml
[dependencies]
sled = "0.34"
```

### Phase Completion Checklist:
- [x] Dependency added to Cargo.toml
- [x] `cargo check` passes
- [x] Update this planning doc with completion status
- [x] Mark phase status as "Completed" before moving to next phase

### Implementation Notes:
- Added `sled = "0.34"` to `backend/Cargo.toml` in the dependencies section
- Placed under "Persistent storage" comment for clarity
- `cargo check` passed successfully - sled v0.34.7 downloaded and verified
- No issues encountered during dependency resolution

---

## Phase 2: Create SledPersistence Structure

### Status: Completed

### Code Style Guidelines
- Keep all functions <20 lines (prefer <10)
- Use helper functions for complex logic
- Follow existing patterns from InMemoryPersistence
- No defensive coding - trust the types

### Files to Create:
- `backend/src/persistence/sled.rs` - SledPersistence implementation

### Files to Modify:
- `backend/src/persistence/mod.rs` - Export SledPersistence

### Tasks:
- [x] Create `backend/src/persistence/sled.rs`
- [x] Define `SledPersistence` struct with `db: sled::Db` field
- [x] Implement `new(path: &str) -> Result<Self>` constructor
- [x] Add helper function `users_tree() -> sled::Tree`
- [x] Add helper function `user_states_tree() -> sled::Tree`
- [x] Export SledPersistence in mod.rs
- [x] Run `cargo check`

### Implementation Details:

```rust
use anyhow::Result;
use sled::Db;

pub struct SledPersistence {
    db: Db,
}

impl SledPersistence {
    pub fn new(path: &str) -> Result<Self> {
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
```

**Tree structure:**
- "users" tree: Maps username -> User JSON
- "user_states" tree: Maps user_id -> UserState JSON

### Phase Completion Checklist:
- [x] SledPersistence struct created
- [x] Constructor implemented (<10 lines)
- [x] Helper functions implemented (<5 lines each)
- [x] `cargo check` passes
- [x] Update this planning doc with completion status
- [x] Mark phase status as "Completed" before moving to next phase

### Implementation Notes:
- Created `backend/src/persistence/sled.rs` with SledPersistence struct
- Struct contains single field: `db: sled::Db`
- `new()` constructor: 4 lines - opens database at specified path
- `users_tree()` helper: 3 lines - opens "users" tree
- `user_states_tree()` helper: 3 lines - opens "user_states" tree
- Added module declaration and export in `backend/src/persistence/mod.rs`
- `cargo check` passed successfully
- Expected dead_code warnings (struct/methods not yet used) - will be resolved in Phase 3

---

## Phase 3: Implement UserPersistence Trait

### Status: Completed

### Code Style Guidelines
- Keep all functions <20 lines (prefer <10)
- Use helper functions for serialization/deserialization
- Follow existing error handling patterns from InMemoryPersistence
- No defensive coding - trust the types

### Files to Modify:
- `backend/src/persistence/sled.rs` - Implement UserPersistence trait

### Tasks:
- [x] Create helper function `serialize_to_json<T: Serialize>(value: &T) -> Result<Vec<u8>>`
- [x] Create helper function `deserialize_from_json<T: DeserializeOwned>(bytes: &[u8]) -> Result<T>`
- [x] Implement `create_user(&self, user: &User) -> Result<()>`
- [x] Implement `load_user_by_username(&self, username: &str) -> Result<Option<User>>`
- [x] Implement `save_user_state(&self, state: &UserState) -> Result<()>`
- [x] Implement `load_user_state(&self, user_id: Uuid) -> Result<Option<UserState>>`
- [x] Run `cargo check`

### Implementation Details:

**Helper functions:**
```rust
fn serialize_to_json<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec(value)?)
}

fn deserialize_from_json<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    Ok(serde_json::from_slice(bytes)?)
}
```

**Trait implementation:**
```rust
#[async_trait]
impl UserPersistence for SledPersistence {
    async fn create_user(&self, user: &User) -> Result<()> {
        // Validate username not empty
        // Check if username exists
        // Serialize and insert
        // Return Ok or error
    }

    async fn load_user_by_username(&self, username: &str) -> Result<Option<User>> {
        // Get from users tree
        // Deserialize if exists
        // Return Option<User>
    }

    async fn save_user_state(&self, state: &UserState) -> Result<()> {
        // Serialize UserState
        // Insert to user_states tree with user_id as key
        // Return Ok or error
    }

    async fn load_user_state(&self, user_id: Uuid) -> Result<Option<UserState>> {
        // Get from user_states tree
        // Deserialize if exists
        // Return Option<UserState>
    }
}
```

**Key design decisions:**
- Use `username` as key for users (enables lookup by username for sign-in)
- Use `user_id.to_string()` as key for user_states
- Validate username not empty in create_user (same as InMemoryPersistence)
- Check for duplicate username in create_user
- Return `None` for missing data (not an error)

### Phase Completion Checklist:
- [x] Helper functions implemented (<5 lines each)
- [x] All trait methods implemented (<20 lines each)
- [x] Validation matches InMemoryPersistence behavior
- [x] `cargo check` passes
- [x] Update this planning doc with completion status
- [x] Mark phase status as "Completed" before moving to next phase

### Implementation Notes:
- Created complete UserPersistence trait implementation in `backend/src/persistence/sled.rs`
- Added necessary imports: anyhow, dialect_coach_shared, serde, uuid, UserPersistence trait
- Added logging to SledPersistence::new() constructor with tracing::info
- Implemented two helper functions:
  - `serialize_to_json<T: Serialize>()`: 3 lines - converts value to JSON bytes
  - `deserialize_from_json<T: DeserializeOwned>()`: 3 lines - converts JSON bytes to value
- Implemented all 5 UserPersistence trait methods with #[async_trait::async_trait]:
  - `initialize()`: 3 lines - logs initialization, returns Ok
  - `save()`: 7 lines - serializes UserState, stores with user_id as key, logs operation
  - `load()`: 7 lines - retrieves UserState by user_id, deserializes if exists, logs result
  - `create_user()`: 14 lines - validates username not empty, checks duplicates, stores User, logs operation
  - `load_user_by_username()`: 7 lines - retrieves User by username, deserializes if exists, logs result
- All functions well under 20-line limit (largest is 14 lines)
- Validation patterns match InMemoryPersistence exactly:
  - Empty username check returns error
  - Duplicate username check returns error
  - Missing data returns None (not error)
- `cargo check` passed successfully
- Expected warnings (dead_code for SledPersistence and methods) - will be resolved in Phase 4 when instantiated in main.rs
- No implementation issues encountered - compiled correctly on first attempt

---

## Phase 4: Update Backend to Use SledPersistence

### Status: Completed

### Code Style Guidelines
- Keep all functions <20 lines (prefer <10)
- Use environment variable for database path (with default)
- Follow existing patterns from main.rs
- No defensive coding - trust the types

### Files to Modify:
- `backend/src/main.rs` - Replace InMemoryPersistence with SledPersistence

### Tasks:
- [x] Add import for SledPersistence
- [x] Replace `InMemoryPersistence::new()` with `SledPersistence::new("./data/dialect-coach.db")?`
- [x] Ensure `./data` directory is created (or let sled create it)
- [x] Remove InMemoryPersistence import if no longer used
- [x] Run `cargo check`

### Implementation:

**Before:**
```rust
let persistence = Arc::new(InMemoryPersistence::new());
```

**After:**
```rust
use crate::persistence::SledPersistence;

let db_path = std::env::var("DB_PATH").unwrap_or_else(|_| "./data/dialect-coach.db".to_string());
let persistence = Arc::new(SledPersistence::new(&db_path)?);
```

**Environment variable:**
- Default: `./data/dialect-coach.db`
- Override: Set `DB_PATH` environment variable

### Phase Completion Checklist:
- [x] SledPersistence instantiated in main.rs
- [x] InMemoryPersistence reference removed (if unused)
- [x] Database path configurable via environment variable
- [x] `cargo check` passes
- [x] Update this planning doc with completion status
- [x] Mark phase status as "Completed" before moving to next phase

### Implementation Notes:
- Updated `backend/src/main.rs` line 28: Changed import from `InMemoryPersistence` to `SledPersistence`
- Updated lines 63-73: Replaced InMemoryPersistence initialization with SledPersistence
- Added `DB_PATH` environment variable support (lines 64-65):
  - Default path: `./data/dialect-coach.db`
  - Can be overridden by setting `DB_PATH` environment variable
  - Uses `std::env::var().unwrap_or_else()` pattern
- SledPersistence instantiation (lines 66-69):
  - Creates SledPersistence with configurable path
  - Wrapped in Arc<dyn UserPersistence> for trait object
  - Added context message: "Failed to create SledPersistence"
- Database directory creation: Sled automatically creates parent directories
- `cargo check` passed successfully
- Previous dead_code warnings for SledPersistence now resolved (struct is being used)
- Minor warning: InMemoryPersistence export in mod.rs is now unused (not critical - kept for potential fallback)
- No compilation errors encountered

---

## Phase 5: Testing & Verification

### Status: Completed

### Code Style Guidelines
- Write tests for happy paths and error cases
- Test data persistence across restarts
- Keep test functions <20 lines
- Use helper functions for test setup

### Tasks:
- [x] Run `cargo check` - must pass
- [x] Run `cargo test` - must pass 100%
- [x] Manual test: Create user, restart backend, verify user persists
- [x] Manual test: Create user, sign in, add conversation, restart, sign in, verify conversation restored
- [x] Manual test: Multiple users with different UserStates
- [x] Verify database file created at correct location
- [x] Verify all functions <20 lines
- [x] Update planning doc with results

### Test Scenarios:

**Scenario 1: User Creation Persistence**
1. Start backend
2. Create user "alice"
3. Stop backend
4. Start backend
5. Sign in as "alice"
6. Verify: Sign in successful (user persisted)

**Scenario 2: UserState Persistence**
1. Create user "bob"
2. Have conversation, change settings
3. Stop backend
4. Start backend
5. Sign in as "bob"
6. Verify: Conversation history and settings restored

**Scenario 3: Multiple Users**
1. Create users "alice", "bob", "charlie"
2. Each has unique UserState data
3. Stop backend
4. Start backend
5. Sign in as each user
6. Verify: Each user's data correctly restored

**Scenario 4: Error Handling**
1. Try creating user with empty username
2. Verify: Error returned (same as InMemoryPersistence)
3. Try creating duplicate username
4. Verify: Error returned
5. Try loading non-existent user
6. Verify: Returns None (not error)

### Phase Completion Checklist:
- [x] All tests pass (100% success required)
- [x] Manual testing confirms data persistence
- [x] Error handling matches InMemoryPersistence
- [x] All functions <20 lines
- [x] Update this planning doc with test results
- [x] Mark phase status as "Completed" before moving to next phase

### Implementation Notes:
- **Automated tests**: `cargo test` passed with 32 passed; 0 failed; 3 ignored (100% success rate)
- **cargo check**: Passed successfully
- **Manual testing**: All test scenarios passed
  - User creation persistence across backend restarts: Passed
  - UserState persistence (conversations, settings): Passed
  - Multiple users with different data: Passed
  - Error handling (empty username, duplicates, missing users): Passed
- **Database file creation**: Verified at correct location
- **Code quality**: All functions confirmed under 20 lines
- **No issues encountered**: SledPersistence fully functional as drop-in replacement for InMemoryPersistence
- **SLED implementation complete**: All phases (1-5) successfully completed

---

## Implementation Order

Execute phases in order:
1. **Phase 1**: Add Sled Dependency
2. **Phase 2**: Create SledPersistence Structure
3. **Phase 3**: Implement UserPersistence Trait
4. **Phase 4**: Update Backend to Use SledPersistence
5. **Phase 5**: Testing & Verification

---

## Code Guidelines Checklist

For EVERY phase:
- [ ] All functions <20 lines (prefer <10)
- [ ] Use helper functions for complex logic
- [ ] Follow existing patterns from InMemoryPersistence
- [ ] Run `cargo check` before marking complete
- [ ] Update planning doc with deviations
- [ ] Document any issues encountered

---

## Migration Notes

### From InMemoryPersistence to SledPersistence
- No API changes required (same trait)
- Data is now persistent across restarts
- Database file stored at `./data/dialect-coach.db`

### Future Migration to Cloud (DynamoDB/Firestore)
- Create new persistence implementation (e.g., `DynamoPersistence`)
- Key patterns already match NoSQL model
- Swap implementation in main.rs
- No changes to application code

---

## Database File Management

### Location
- Default: `./data/dialect-coach.db`
- Configurable via `DB_PATH` environment variable

### Backup
- Copy entire `./data` directory to backup
- Sled is crash-safe and ACID-compliant

### Reset/Clear Data
- Delete `./data` directory to start fresh
- Backend will recreate on next start

---

## Out of Scope (Future Work)

- Database migrations/versioning
- Data compression
- Encryption at rest
- Replication/clustering
- Backup automation
- Database admin UI
- Performance optimization
- Index tuning

---

## Notes

- Sled is embedded - no external services required
- Data persists in single directory
- ACID transactions ensure data safety
- File-based storage makes backup simple
- Pattern prepares for NoSQL cloud migration
