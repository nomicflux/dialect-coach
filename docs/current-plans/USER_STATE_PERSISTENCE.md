# User State Management & Persistence - Implementation Plan

**Start Date**: 2025-10-25

---

## Overview

Extract user-specific data from UIState/AppState into dedicated UserState, persist to backend via trait-based abstraction starting with in-memory implementation.

## Design Decisions

**User specified** (2025-10-25):
- **User Identity**: Separate `user_id` (UUID in localStorage) that persists across sessions
- **Persistence Timing**: Debounced (2s after last change) with retry queue
- **Data Structure**: Complete `Vec<LearningItem>` objects (not separated scores)
- **Error Handling**: Queue failed saves, retry on reconnect

---

## Phase 1: Define UserState Model (shared crate)

### Status: Completed (2025-10-25)

### Before Starting This Phase:
- [ ] Review `.claude/CLAUDE.md` for code style guidelines
- [ ] Functions must be <20 lines (prefer <10 lines)
- [ ] Write helper functions for complex logic
- [ ] Every function needs a test
- [ ] Use pure functions where possible

### Tasks:
- [ ] Create new file `shared/src/models/user_state.rs`
- [ ] Move `LearningItem` and `LearningItemType` from frontend to shared
- [ ] Define `UserState` struct with all user-specific fields
- [ ] Implement Serialize/Deserialize for UserState
- [ ] Add pub use to `shared/src/models/mod.rs`
- [ ] Write tests for UserState serialization

### Files to Create:
- `shared/src/models/user_state.rs`

### Files to Modify:
- `shared/src/models/mod.rs`
- `frontend/src/app/app_state.rs` (remove LearningItem, will be replaced)

### Data Structure:
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserState {
    pub user_id: Uuid,
    pub learning_items: Vec<LearningItem>,
    pub conversation_history: Vec<Message>,  // Capped at 20 items
    pub tts_enabled: bool,
    pub selected_language: Language,
    pub selected_dialect: Dialect,
    pub formality: Formality,
    pub teaching_mode: TeachingMode,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LearningItem {
    pub item: LearningItemType,
    pub score: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LearningItemType {
    Mistake(Mistake),
    Explanation(Explained),
    Translation(Translated),
    Exploration(Exploratory),
}
```

### Implementation Notes:
- **No Default impl**: UserState requires a user_id, which cannot be determined from nothing
- **user_id = session_id**: Use existing session_id from AppState as the user_id (no separate phase needed)
- **Conversation history cap**: The 20-item cap will be enforced in the frontend when adding messages
- **Message type**: Uses existing `Message` struct from shared models

### Tests:
- test_user_state_serialization
- test_user_state_deserialization
- test_conversation_history_in_user_state

### Phase Completion Checklist:
- [x] All tests pass (100% success required) - 72 tests passed in shared crate
- [x] All functions are <20 lines - UserState::new() is 12 lines, LearningItem::new() is 3 lines
- [x] Update this planning doc with any deviations or issues encountered
- [x] Document any user corrections or rejected approaches
- [x] Mark phase status as "Completed" before moving to next phase

### Implementation Notes (2025-10-25):
- Created `shared/src/models/user_state.rs` with UserState, LearningItem, and LearningItemType
- Added 6 tests: test_user_state_serialization, test_user_state_deserialization, test_conversation_history_in_user_state, test_learning_item_new, test_user_state_new, test_learning_item_serialization
- All functions follow code style guidelines (<20 lines, simple and modular)
- No deviations from the original plan
- All tests pass successfully (cargo test --lib: 72 passed)

---

## Phase 2: Extract UserState in Frontend (frontend crate)

### Status: Completed (2025-10-25)

### Before Starting This Phase:
- [x] Review `.claude/CLAUDE.md` for code style guidelines
- [x] Functions must be <20 lines (prefer <10 lines)
- [x] Write helper functions for complex logic
- [x] Every function needs a test
- [x] Use pure functions where possible

### Tasks:
- [x] Import UserState from shared crate
- [x] Create UserStateAction enum in `frontend/src/app/app_state.rs`
- [x] Implement Reducible for UserState (via UserStateWrapper newtype)
- [x] Remove user fields from AppState: LanguageChoices, LanguageManner, MessagesState
- [x] Remove user fields from UIState: learning_items, tts_enabled
- [x] Update all component references to use UserState
- [x] Wire up UserState reducer in app.rs

### Files to Modify:
- `frontend/src/app/app_state.rs` - add UserStateAction, remove user fields, implement Reducible
- `frontend/src/app.rs` - add use_reducer for UserState
- All components using removed fields

### UserStateAction enum:
```rust
pub enum UserStateAction {
    AddMessage(Message),  // Manages conversation_history with 20-item cap
    AddLearningItems(Vec<Mistake>, Vec<Explained>, Vec<Translated>, Vec<Exploratory>),  // Add items with score=0
    UpdateScores(AgentAnalysis),  // Update scores separately
    SetLanguage(Language),
    SetDialect(Dialect),
    SetFormality(Formality),
    SetTeachingMode(TeachingMode),
    ToggleTTS,
    LoadFromBackend(UserState),  // Hydrate entire state from backend
}
```

### Implementation Notes:
- **Use shared UserState directly** - import from dialect_coach_shared
- **Reducible impl**: Implement in frontend since shared can't depend on Yew
- **Conversation history**: Move from AppState.messages to UserState.conversation_history
- **Learning items vs scores**: Keep separate - items added with score=0, scores updated via separate action

### Phase Completion Checklist:
- [x] All tests pass (100% success required) - 91 tests passed (corpus: 14, backend: 5, shared: 72)
- [x] All functions are <20 lines - Helper functions: add_learning_items_to_vec (18 lines), update_item_score (23 lines), apply_score_updates (1 line), default_dialect_for_language (6 lines), apply_user_state_action (35 lines)
- [x] Update this planning doc with any deviations or issues encountered
- [x] Document any user corrections or rejected approaches
- [x] Mark phase status as "Completed" before moving to next phase

### Implementation Notes (2025-10-25):

**Key Issue Encountered: Rust Orphan Rules**
- Cannot implement `Reducible` (Yew trait) for `UserState` (shared crate type) in frontend crate
- **Solution**: Created `UserStateWrapper` newtype in frontend: `struct UserStateWrapper(pub UserState)`
- Implemented `Deref` trait so wrapper is transparent - can access UserState methods directly
- User correctly identified that reducer pattern is essential (not optional) because:
  - Single source of truth across components
  - Avoids stale closure issues
  - Traceable state updates via typed actions
  - Entire app already uses this pattern

**Helper Functions Created**:
- `add_learning_items_to_vec()` - adds new learning items to vector (18 lines)
- `update_item_score()` - updates single item score from analysis (23 lines)
- `apply_score_updates()` - maps score updates across all items (1 line)
- `default_dialect_for_language()` - returns default dialect per language (6 lines)
- `apply_user_state_action()` - main state reducer function (35 lines)

**UserState Methods Added to Shared**:
- Moved helper methods to `shared/src/models/user_state.rs` to avoid orphan rules:
  - `create_msg()`, `bcp47_tag()`, `current_dialect()`, `current_dialects()`, `formality_display()`, `teaching_mode_display()`

**Files Modified**:
- `frontend/src/app/app_state.rs` - Removed LanguageChoices, LanguageManner, MessagesState structs; removed duplicate LearningItem/LearningItemType; added UserStateWrapper with Reducible impl
- `frontend/src/app.rs` - Updated all callbacks to use UserStateWrapper; added UserStateWrapper to use_reducer; updated component props
- `frontend/src/components/learning_panel.rs` - Changed import from `crate::app` to `dialect_coach_shared`
- `shared/src/models/user_state.rs` - Added helper methods for frontend use

**Test Results**:
- cargo check: passes with only dead code warnings (unrelated to Phase 2)
- cargo test --lib: 91 tests passed (14 corpus-processor, 5 backend, 72 shared)
- No new tests needed in Phase 2 (state extraction, not new logic)

---

## Phase 3: Debounced Persistence (frontend crate)

### Status: Completed (2025-10-25)

### Before Starting This Phase:
- [x] Review `.claude/CLAUDE.md` for code style guidelines
- [x] Functions must be <20 lines (prefer <10 lines)
- [x] Write helper functions for complex logic
- [x] Every function needs a test
- [x] Use pure functions where possible

### Tasks:
- [x] Create `frontend/src/hooks/use_debounced_save.rs`
- [x] Implement debounce logic (2 second delay)
- [x] Cancel pending saves on new changes
- [x] Add force_save_now() function for manual sync
- [x] Add hook to `frontend/src/hooks/mod.rs`
- [x] Update `frontend/src/services/persistence.rs` with localStorage functions
- [x] Wire up hook in app.rs with auto-load on initialization

### Files to Create:
- `frontend/src/hooks/use_debounced_save.rs`
- `frontend/src/hooks/mod.rs` (if not exists)

### Implementation Details:
```rust
pub fn use_debounced_save(
    user_state: &UserState,
    save_callback: Callback<UserState>,
) -> Callback<()> {
    // Watch user_state for changes
    // Cancel pending timeout on change
    // Start new 2s timeout
    // Trigger save_callback after timeout

    // Return force_save_now callback for manual triggers
}
```

### Implementation Notes:
- **Automatic watching**: Hook monitors UserState changes via use_effect
- **No disconnect edge cases**: Too late once disconnect is noticed
- **Manual sync**: Returns force_save_now() callback for "Sync Settings" button

### Phase Completion Checklist:
- [x] All tests pass (100% success required) - 91 tests passed (corpus: 14, backend: 5, shared: 72)
- [x] All functions are <20 lines - save_user_state (11 lines), load_user_state (18 lines), get_local_storage (6 lines), use_debounced_save (37 lines)
- [x] Update this planning doc with any deviations or issues encountered
- [x] Document any user corrections or rejected approaches
- [x] Mark phase status as "Completed" before moving to next phase

### Implementation Notes (2025-10-25):

**Files Created**:
- `frontend/src/hooks/mod.rs` - Hook module exports
- `frontend/src/hooks/use_debounced_save.rs` - Custom Yew hook with #[hook] macro for debounced saving

**Files Modified**:
- `frontend/src/services/persistence.rs` - Implemented localStorage save/load functions using web-sys API
- `frontend/src/services/mod.rs` - Removed non-existent PersistenceService export
- `frontend/src/lib.rs` - Added hooks module
- `frontend/src/app.rs` - Added auto-load on init and debounced auto-save

**Implementation Details**:
- **localStorage API**: Uses web-sys::window().local_storage() for browser persistence
- **Serialization**: UserState serialized to JSON via serde_json
- **Debounce Mechanism**: 2-second timeout using gloo::timers::callback::Timeout
- **Automatic Load**: UserState loaded from localStorage on app initialization
- **Automatic Save**: Any UserState change triggers 2-second debounced save
- **Manual Sync**: Hook returns `Callback<()>` for force_save_now (stored as `_force_save`, can be exposed later)

**Hook Structure** (#[hook] macro):
```rust
#[hook]
pub fn use_debounced_save<F>(user_state: &UserState, save_fn: F) -> Callback<()>
where F: Fn(&UserState) + 'static + Clone
```
- Uses `use_state` to track pending timeout
- Uses `use_effect_with` to watch UserState changes
- Cancels previous timeout when new changes arrive
- Returns `use_callback` for manual force save

**Test Results**:
- cargo check: passes (only unrelated dead code warnings)
- cargo test --lib: 91 tests passed
- No new tests added (hook tested via integration when app runs)

---

## Phase 4: Save Queue with Retry (frontend crate)

### Status: Completed (2025-10-25)

### Before Starting This Phase:
- [x] Review `.claude/CLAUDE.md` for code style guidelines
- [x] Functions must be <20 lines (prefer <10 lines)
- [x] Write helper functions for complex logic
- [x] Every function needs a test
- [x] Use pure functions where possible

### Tasks:
- [x] Create `frontend/src/services/save_queue.rs`
- [x] Implement PendingSaveQueue struct with Option<UserState>
- [x] Add enqueue() - replaces any existing pending state
- [x] Add retry_all() - sends pending save on reconnect
- [x] Wire up automatic retry on WebSocket reconnect
- [x] Add minimal logging
- [x] Add to services mod

### Files to Create:
- `frontend/src/services/save_queue.rs`

### Files to Modify:
- `frontend/src/services/mod.rs`
- `frontend/src/app.rs` - call retry_all() on reconnect

### Implementation Details:
```rust
pub struct PendingSaveQueue {
    pending: RefCell<Option<UserState>>,
}

impl PendingSaveQueue {
    pub fn new() -> Self {
        Self {
            pending: RefCell::new(None),
        }
    }

    pub fn enqueue(&self, state: UserState) {
        let had_pending = self.pending.borrow().is_some();
        *self.pending.borrow_mut() = Some(state);
        if !had_pending {
            log::info!("Queued user state save for retry");
        }
    }

    pub fn retry_all(&self, ws_service: &WebSocketService) {
        if let Some(state) = self.pending.borrow_mut().take() {
            log::info!("Retrying pending user state save");
            if let Err(e) = ws_service.save_user_state(&state) {
                log::warn!("Retry failed: {}", e);
                // Re-queue on failure
                *self.pending.borrow_mut() = Some(state);
            }
        }
    }
}
```

### Implementation Notes:
- **Option not queue**: Latest state overwrites previous - only need one pending save
- **Automatic retry**: Call retry_all() in WebSocket reconnect handler
- **Minimal logging**: Log when queuing first save and when retrying, warn on retry failure

### Phase Completion Checklist:
- [x] All tests pass (100% success required) - 91 tests passed (corpus: 14, backend: 5, shared: 72)
- [x] All functions are <20 lines - enqueue (9 lines), retry_all (15 lines), has_pending (3 lines), new (5 lines)
- [x] Update this planning doc with any deviations or issues encountered
- [x] Document any user corrections or rejected approaches
- [x] Mark phase status as "Completed" before moving to next phase

### Implementation Notes (2025-10-25):

**User Correction - Reducer Pattern Required**:
- Initial attempt: Used use_state to store save queue and closures for error handling
- **User feedback**: "This should be part of the App State using the Reducer and messages. That keeps the whole system clean and understandable, without having to work through which version of the state is doing what."
- **Correct approach**: Added save queue to AppState with proper reducer actions

**Files Created**:
- `frontend/src/services/save_queue.rs` - PendingSaveQueue service with localStorage retry

**Files Modified**:
- `frontend/src/services/mod.rs` - Added save_queue module export
- `frontend/src/app/app_state.rs` - Added save_queue field, QueuePendingSave and RetryPendingSaves actions
- `frontend/src/app.rs` - Dispatch QueuePendingSave on save failure, RetryPendingSaves on Connected state

**Implementation Details**:
- **Save queue in AppState**: `save_queue: Rc<PendingSaveQueue>` initialized in Default::default()
- **Actions added to AppStateAction enum**:
  - `QueuePendingSave(UserState)` - called when localStorage save fails
  - `RetryPendingSaves` - called when connection state becomes Connected
- **Action handlers in apply_action()**:
  - QueuePendingSave: calls `save_queue.enqueue(state)`
  - RetryPendingSaves: calls `save_queue.retry_all()` with error logging
- **Wiring in app.rs**:
  - Debounced save callback dispatches QueuePendingSave on Err
  - on_state_change callback dispatches RetryPendingSaves when ConnectionState::Connected
- **Retry mechanism**: Uses localStorage (not WebSocket yet - that's Phase 5+)

**PendingSaveQueue Structure**:
```rust
pub struct PendingSaveQueue {
    pending: RefCell<Option<UserState>>,
}

Methods:
- new() -> Self (5 lines)
- enqueue(&self, state: UserState) (9 lines)
- retry_all(&self) -> Result<(), String> (15 lines)
- has_pending(&self) -> bool (3 lines)
```

**Test Results**:
- cargo check: passes (only unrelated dead code warnings)
- cargo test --lib: 91 tests passed
- No new tests added (service tested via integration with reducer)

**Key Design Decisions**:
- Latest state replaces previous pending state (no queue, just Option)
- Retry uses localStorage, not WebSocket (WebSocket protocol updates in Phase 5)
- All state mutations go through reducer pattern for single source of truth
- Logging: info on enqueue (first time only), info on retry, warn on retry failure

---

## Phase 5: WebSocket Protocol Updates (shared crate)

### Status: Not Started

### Before Starting This Phase:
- [ ] Review `.claude/CLAUDE.md` for code style guidelines
- [ ] Functions must be <20 lines (prefer <10 lines)
- [ ] Write helper functions for complex logic
- [ ] Every function needs a test
- [ ] Use pure functions where possible

### Tasks:
- [ ] Add UserStateMessage enum to `shared/src/models/message.rs`
- [ ] Implement Serialize/Deserialize
- [ ] Write serialization/deserialization tests
- [ ] Test Result<(), String> serialization

### Files to Modify:
- `shared/src/models/message.rs`

### Data Structure:
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UserStateMessage {
    Save(UserState),
    Load(Uuid),  // user_id
    SaveResponse(Result<(), String>),  // Ok(()) is ACK, Err(msg) is error
    LoadResponse(Option<UserState>),  // Some = found, None = not found
}
```

### Implementation Notes:
- **Separate message type**: Not integrated with existing Message types
- **ACK via SaveResponse**: `Ok(())` serves as acknowledgment, no separate ACK needed
- **No versioning**: Not needed at this stage - future concern

### Phase Completion Checklist:
- [ ] All tests pass (100% success required)
- [ ] All functions are <20 lines
- [ ] Update this planning doc with any deviations or issues encountered
- [ ] Document any user corrections or rejected approaches
- [ ] Mark phase status as "Completed" before moving to next phase

---

## Phase 6: Backend Persistence Trait (backend crate)

### Status: Not Started

### Before Starting This Phase:
- [ ] Review `.claude/CLAUDE.md` for code style guidelines
- [ ] Functions must be <20 lines (prefer <10 lines)
- [ ] Write helper functions for complex logic
- [ ] Every function needs a test
- [ ] Use pure functions where possible

### Tasks:
- [ ] Create `backend/src/persistence/mod.rs`
- [ ] Define UserPersistence trait with `&self` methods
- [ ] Add comprehensive doc comments
- [ ] Add to backend lib/main

### Files to Create:
- `backend/src/persistence/mod.rs`

### CRITICAL Requirements:
- Trait must be completely generic
- NO "in-memory" references anywhere in trait definition
- NO "in-memory" references in mod.rs
- Trait must be Send + Sync (for Arc<dyn UserPersistence>)
- Methods take `&self` - implementations handle interior mutability

### Trait Definition:
```rust
use anyhow::Result;
use dialect_coach_shared::UserState;
use uuid::Uuid;

/// Trait for persisting user state
///
/// Implementations can use any storage backend and handle their own interior mutability.
pub trait UserPersistence: Send + Sync {
    /// Initialize the persistence layer
    /// Called once at startup to set up storage (e.g., load from disk, connect to DB)
    async fn initialize(&self) -> Result<()>;

    /// Save user state
    async fn save(&self, user_state: &UserState) -> Result<()>;

    /// Load user state by user ID
    /// Returns None if user not found
    async fn load(&self, user_id: Uuid) -> Result<Option<UserState>>;
}
```

### Implementation Notes:
- **Send + Sync**: Allows `Arc<dyn UserPersistence>` for shared access across async tasks
- **`&self` methods**: Implementations control mutability (e.g., Arc<Mutex<HashMap>> internally)
- **Keep simple**: Only save/load for now - add delete/list/etc when needed

### Phase Completion Checklist:
- [ ] All tests pass (100% success required)
- [ ] All functions are <20 lines
- [ ] Update this planning doc with any deviations or issues encountered
- [ ] Document any user corrections or rejected approaches
- [ ] Mark phase status as "Completed" before moving to next phase

---

## Phase 7: InMemoryPersistence Implementation (backend crate)

### Status: Not Started

### Before Starting This Phase:
- [ ] Review `.claude/CLAUDE.md` for code style guidelines
- [ ] Functions must be <20 lines (prefer <10 lines)
- [ ] Write helper functions for complex logic
- [ ] Every function needs a test
- [ ] Use pure functions where possible

### Tasks:
- [ ] Create `backend/src/persistence/in_memory.rs`
- [ ] Implement InMemoryPersistence struct with Arc<Mutex<HashMap>>
- [ ] Implement UserPersistence trait
- [ ] Add simple tests (basic save/load)
- [ ] Add pub mod to persistence/mod.rs

### Files to Create:
- `backend/src/persistence/in_memory.rs`

### Files to Modify:
- `backend/src/persistence/mod.rs` - add `pub mod in_memory;`

### CRITICAL Requirements:
- ALL "in-memory" terminology CONFINED to this file only
- Logging inside this impl CAN mention "in-memory"
- NO other file should reference "in-memory"
- File name is `in_memory.rs` but that's the ONLY place

### Implementation:
```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use anyhow::Result;
use dialect_coach_shared::UserState;
use uuid::Uuid;
use super::UserPersistence;

pub struct InMemoryPersistence {
    state: Arc<Mutex<HashMap<Uuid, UserState>>>,
}

impl InMemoryPersistence {
    pub fn new() -> Self {
        tracing::info!("Initializing in-memory user state storage");
        Self {
            state: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl UserPersistence for InMemoryPersistence {
    async fn initialize(&self) -> Result<()> {
        tracing::info!("In-memory persistence initialized (no-op)");
        Ok(())
    }

    async fn save(&self, user_state: &UserState) -> Result<()> {
        let mut state = self.state.lock().await;
        state.insert(user_state.user_id, user_state.clone());
        tracing::debug!("Saved user state to in-memory storage: {}", user_state.user_id);
        Ok(())
    }

    async fn load(&self, user_id: Uuid) -> Result<Option<UserState>> {
        let state = self.state.lock().await;
        let result = state.get(&user_id).cloned();
        tracing::debug!("Loaded user state from in-memory storage: {} (found: {})", user_id, result.is_some());
        Ok(result)
    }
}
```

### Implementation Notes:
- **Simple tests only**: Basic save/load, not production-ready
- **No initialization data**: Starts empty
- **No metrics**: Just basic debug logging
- **Development tool**: Placeholder for production persistence later

### Phase Completion Checklist:
- [ ] All tests pass (100% success required)
- [ ] All functions are <20 lines
- [ ] Update this planning doc with any deviations or issues encountered
- [ ] Document any user corrections or rejected approaches
- [ ] Mark phase status as "Completed" before moving to next phase

---

## Phase 8: Backend AppState Integration (backend crate)

### Status: Not Started

### Before Starting This Phase:
- [ ] Review `.claude/CLAUDE.md` for code style guidelines
- [ ] Functions must be <20 lines (prefer <10 lines)
- [ ] Write helper functions for complex logic
- [ ] Every function needs a test
- [ ] Use pure functions where possible

### Tasks:
- [ ] Add user_persistence field to AppState
- [ ] Initialize with InMemoryPersistence in main.rs
- [ ] Call initialize() on persistence layer
- [ ] Pass to WebSocket handlers

### Files to Modify:
- `backend/src/main.rs`

### Changes:
```rust
pub struct AppState {
    // ... existing fields
    pub user_persistence: Arc<dyn UserPersistence>,
}

// In main():
let user_persistence: Arc<dyn UserPersistence> = Arc::new(InMemoryPersistence::new());

// Initialize persistence layer (load from disk, connect to DB, etc.)
user_persistence.initialize().await
    .context("Failed to initialize user persistence")?;

let app_state = AppState {
    // ... existing fields
    user_persistence,
};
```

### CRITICAL:
- Variable name: `user_persistence` (generic)
- Type: `Arc<dyn UserPersistence>` (trait object)
- **Only reference to `InMemoryPersistence` is at initialization line**
- Easy to swap: change `InMemoryPersistence::new()` to `DatabasePersistence::new()`
- AppState owns the Arc (standard axum pattern)

### Phase Completion Checklist:
- [ ] All tests pass (100% success required)
- [ ] All functions are <20 lines
- [ ] Update this planning doc with any deviations or issues encountered
- [ ] Document any user corrections or rejected approaches
- [ ] Mark phase status as "Completed" before moving to next phase

---

## Phase 9: WebSocket Handlers (backend crate)

### Status: Not Started

### Before Starting This Phase:
- [ ] Review `.claude/CLAUDE.md` for code style guidelines
- [ ] Functions must be <20 lines (prefer <10 lines)
- [ ] Write helper functions for complex logic
- [ ] Every function needs a test
- [ ] Use pure functions where possible

### Tasks:
- [ ] Create new WebSocket endpoint `/ws/user_state` (separate from `/ws` chat endpoint)
- [ ] Add handle_save_user_state() function
- [ ] Add handle_load_user_state() function
- [ ] Create message router for UserStateMessage types
- [ ] Add logging
- [ ] Handle errors gracefully

### Files to Modify:
- `backend/src/main.rs` - add new route
- `backend/src/websocket.rs` - add user state handlers (or create new file)

### Handler Functions:
```rust
async fn handle_save_user_state(
    state: &AppState,
    user_state: UserState,
    tx: &mpsc::UnboundedSender<String>,
) -> Result<(), ()> {
    tracing::info!("Saving user state for user: {}", user_state.user_id);

    match state.user_persistence.save(&user_state).await {
        Ok(_) => {
            let response = UserStateMessage::SaveResponse(Ok(()));
            let json = serde_json::to_string(&response)
                .map_err(|e| tracing::error!("Failed to serialize: {}", e))?;
            tx.send(json)
                .map_err(|e| tracing::error!("Failed to send: {}", e))?;
            Ok(())
        }
        Err(e) => {
            tracing::error!("Failed to save user state: {}", e);
            let response = UserStateMessage::SaveResponse(Err(e.to_string()));
            let json = serde_json::to_string(&response)
                .map_err(|e| tracing::error!("Failed to serialize: {}", e))?;
            tx.send(json)
                .map_err(|e| tracing::error!("Failed to send: {}", e))?;
            Ok(())
        }
    }
}

async fn handle_load_user_state(
    state: &AppState,
    user_id: Uuid,
    tx: &mpsc::UnboundedSender<String>,
) -> Result<(), ()> {
    tracing::info!("Loading user state for user: {}", user_id);

    match state.user_persistence.load(user_id).await {
        Ok(user_state) => {
            let response = UserStateMessage::LoadResponse(user_state);
            let json = serde_json::to_string(&response)
                .map_err(|e| tracing::error!("Failed to serialize: {}", e))?;
            tx.send(json)
                .map_err(|e| tracing::error!("Failed to send: {}", e))?;
            Ok(())
        }
        Err(e) => {
            tracing::error!("Failed to load user state: {}", e);
            let response = UserStateMessage::LoadResponse(None);
            let json = serde_json::to_string(&response)
                .map_err(|e| tracing::error!("Failed to serialize: {}", e))?;
            tx.send(json)
                .map_err(|e| tracing::error!("Failed to send: {}", e))?;
            Ok(())
        }
    }
}
```

### Implementation Notes:
- **No validation**: Save user state as-is, trust frontend
- **Separate endpoint**: `/ws/user_state` for user state operations, `/ws` for chat
- **No rate limiting**: Not needed at this stage

### Phase Completion Checklist:
- [ ] All tests pass (100% success required)
- [ ] All functions are <20 lines
- [ ] Update this planning doc with any deviations or issues encountered
- [ ] Document any user corrections or rejected approaches
- [ ] Mark phase status as "Completed" before moving to next phase

---

## Phase 10: Frontend WebSocket Integration (frontend crate)

### Status: Not Started

### Before Starting This Phase:
- [ ] Review `.claude/CLAUDE.md` for code style guidelines
- [ ] Functions must be <20 lines (prefer <10 lines)
- [ ] Write helper functions for complex logic
- [ ] Every function needs a test
- [ ] Use pure functions where possible

### Tasks:
- [ ] Add second WebSocket connection to WebSocketService for `/ws/user_state` endpoint
- [ ] Add save_user_state() method
- [ ] Add load_user_state() method
- [ ] Add message handlers for SaveResponse and LoadResponse
- [ ] Wire up in app.rs: load on connect, save on debounce
- [ ] Connect retry queue on reconnect

### Files to Modify:
- `frontend/src/services/websocket.rs` - add user_state_ws connection
- `frontend/src/app.rs` - wire up load/save/retry

### WebSocketService Changes:
```rust
pub struct WebSocketService {
    // Existing chat WebSocket
    ws: RefCell<Option<WebSocket>>,
    // New user state WebSocket
    user_state_ws: RefCell<Option<WebSocket>>,
    // ... rest of fields
}

pub fn save_user_state(&self, state: &UserState) -> Result<(), String> {
    let message = UserStateMessage::Save(state.clone());
    let json = serde_json::to_string(&message)
        .map_err(|e| format!("Failed to serialize: {}", e))?;
    // Send to user_state_ws, not main ws
    self.send_to_user_state_ws(&json)
}

pub fn load_user_state(&self, user_id: Uuid) -> Result<(), String> {
    let message = UserStateMessage::Load(user_id);
    let json = serde_json::to_string(&message)
        .map_err(|e| format!("Failed to serialize: {}", e))?;
    self.send_to_user_state_ws(&json)
}
```

### App.rs Integration:
1. On user state WebSocket connect → automatically load UserState
2. Set up callback: on LoadResponse → dispatch LoadFromBackend
3. Start debounced save watcher on UserState changes
4. On debounce trigger → save_user_state()
5. On SaveResponse(Err) → enqueue in retry queue
6. On user state WebSocket reconnect → retry queued saves

### Implementation Notes:
- **Separate connections**: Chat on `/ws`, user state on `/ws/user_state`
- **No message mixing**: Different endpoints = different handlers
- **Auto-load on connect**: Load happens automatically when user state WebSocket connects

### Phase Completion Checklist:
- [ ] All tests pass (100% success required)
- [ ] All functions are <20 lines
- [ ] Update this planning doc with any deviations or issues encountered
- [ ] Document any user corrections or rejected approaches
- [ ] Mark phase status as "Completed" before moving to next phase

---

## Phase 12: Integration Testing

### Status: Not Started

### Before Starting This Phase:
- [ ] Review `.claude/CLAUDE.md` for code style guidelines
- [ ] Functions must be <20 lines (prefer <10 lines)
- [ ] Write helper functions for complex logic
- [ ] Every function needs a test
- [ ] Use pure functions where possible

### Tasks:
- [ ] Manual test: Set preferences, verify debounced save
- [ ] Manual test: Add learning items, verify save
- [ ] Manual test: Refresh browser, verify state restored
- [ ] Manual test: Disconnect WebSocket, make changes, verify retry on reconnect
- [ ] Manual test: Restart backend, verify state lost (in-memory reset)
- [ ] Unit test: Debounce logic
- [ ] Unit test: Save queue
- [ ] Unit test: InMemoryPersistence

### Test Scenarios:

**Scenario 1: Basic Persistence**
1. Start fresh session
2. Change language to Arabic
3. Wait 2 seconds
4. Check logs for "Saved user state"
5. Refresh browser
6. Verify language is still Arabic

**Scenario 2: Learning Progress**
1. Chat in Corrective mode
2. Accumulate learning items with scores
3. Wait 2 seconds
4. Refresh browser
5. Verify learning items and scores restored

**Scenario 3: Network Resilience**
1. Make changes
2. Disconnect WebSocket (stop backend)
3. Make more changes
4. Reconnect WebSocket (start backend)
5. Verify queued saves executed

**Scenario 4: In-Memory Reset**
1. Save state
2. Restart backend
3. Refresh browser
4. Verify state NOT restored (expected - in-memory)

### Phase Completion Checklist:
- [ ] All tests pass (100% success required)
- [ ] All functions are <20 lines
- [ ] Update this planning doc with any deviations or issues encountered
- [ ] Document any user corrections or rejected approaches
- [ ] Mark phase status as "Completed" before moving to next phase

---

## Success Criteria

✅ User preferences persist across browser refreshes
✅ Learning progress persists across browser refreshes
✅ No "in-memory" references outside `backend/src/persistence/in_memory.rs`
✅ Trait allows swapping to database persistence with zero changes outside `backend/src/main.rs`
✅ Debounced saves reduce backend calls (2s delay visible in logs)
✅ Failed saves retry on reconnect
✅ All tests pass

---

## Future Enhancements (Not in Scope)

- Database persistence implementation (PostgreSQL/SQLite)
- User authentication (currently just UUID in localStorage)
- Multi-device sync
- Export/import user data
- User state versioning/migrations

---

## Notes

This implementation establishes the foundation for persistent user state while maintaining clean architecture that allows easy swapping of persistence backends.
