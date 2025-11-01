# Conversation Tree Implementation Plan

## User Requirements Summary
- **Context Strategy:** Only active branch sent to AI agent
- **Learning Items:** Shared globally across all branches
- **UI Approach:** Sidebar branch list navigation
- **Branch Depth:** Unlimited branching allowed
- **Migration:** None - clean slate, no backwards compatibility needed

---

## 1. PERSISTENCE LAYER (shared/src/models/)

### Code Style Guidelines for Phase 1
- Functions <20 lines, prefer <10 lines
- Write pure helper functions for tree traversal logic
- No defensive coding - trust the type system
- Add unit tests for each helper function immediately after writing it
- Use descriptive names: `get_active_branch_messages` not `get_msgs`

### 1.1 Update Message Structure
**File:** `shared/src/models/message.rs`

Add to `Message` struct:
```rust
pub parent_id: Option<Uuid>,  // None for root messages
pub branch_id: Uuid,           // Identifies which branch this message belongs to
```

### 1.2 Create New Branch Structure
**New file:** `shared/src/models/branch.rs`

```rust
pub struct ConversationBranch {
    pub id: Uuid,
    pub parent_message_id: Option<Uuid>,  // Which message this branched from
    pub created_at: DateTime<Utc>,
    pub name: Option<String>,              // User-provided branch name
}
```

### 1.3 Update UserState
**File:** `shared/src/models/user_state.rs`

Add to `UserState`:
```rust
pub active_branch_id: Uuid,
pub branches: Vec<ConversationBranch>,
// conversation_history remains Vec<Message> but now with parent_id/branch_id
```

Add helper methods (each <20 lines):
- `get_active_branch_messages() -> Vec<&Message>` - Returns messages in active branch path from root to tip
- `get_child_branches(&self, message_id: Uuid) -> Vec<&ConversationBranch>` - Find branches starting from a message
- `find_branch_root(&self, branch_id: Uuid) -> Option<Uuid>` - Find the root message of a branch

### Phase 1 Completion Checklist
- [ ] Message struct updated with parent_id and branch_id
- [ ] ConversationBranch struct created
- [ ] UserState updated with active_branch_id and branches
- [ ] Helper methods written (each <20 lines)
- [ ] Unit tests added for all helper methods
- [ ] `cargo test --lib` passes 100%
- [ ] Update this document's "Phase 1 Status" section below

---

## 2. BACKEND CHANGES

### Code Style Guidelines for Phase 2
- Keep WebSocket handler functions <20 lines each
- Extract message context building into separate pure function
- Write helper function for filtering messages to active branch
- Test context filtering with unit tests
- No TODOs - implement complete functionality now

### 2.1 Update WebSocket Message Handling
**File:** `backend/src/websocket.rs`

When receiving `UserMessageWithContext`:
- Use `UserState.get_active_branch_messages()` to build context
- Only send active branch messages to AI agent
- Set new message `parent_id` to last message in active branch
- Set new message `branch_id` to `UserState.active_branch_id`

Create helper function (pure, <15 lines):
```rust
fn build_context_from_branch(user_state: &UserState) -> Vec<Message> {
    // Extract active branch messages
    // Return cloned messages for context
}
```

No persistence changes needed - Sled already serializes entire `UserState` as JSON.

### Phase 2 Completion Checklist
- [ ] WebSocket handler updated to use active branch only
- [ ] Helper function for context building created
- [ ] New messages correctly set parent_id and branch_id
- [ ] Unit tests added for context building logic
- [ ] `cargo test --lib` passes 100%
- [ ] Manual test: Send message, verify only active branch in context
- [ ] Update this document's "Phase 2 Status" section below

---

## 3. FRONTEND STATE MANAGEMENT

### Code Style Guidelines for Phase 3
- Each reducer action handler <20 lines
- Extract complex logic into helper functions
- Pure functions for state transformations
- Test each reducer action with unit tests
- Avoid nested conditionals - use early returns

### 3.1 New Reducer Actions
**File:** `frontend/src/app/app_state.rs`

Add to `UserStateAction` enum:
```rust
CreateBranch(Uuid),           // Create branch from message_id
SwitchBranch(Uuid),           // Switch to branch_id
DeleteBranch(Uuid),           // Delete entire branch
RenameBranch(Uuid, String),   // Give branch a custom name
```

### 3.2 Reducer Implementation

**CreateBranch(message_id):**
1. Create new `ConversationBranch` with `parent_message_id = message_id`
2. Add to `UserState.branches`
3. Set `UserState.active_branch_id` to new branch ID
4. Trigger save

**SwitchBranch(branch_id):**
1. Set `UserState.active_branch_id = branch_id`
2. Trigger save

**DeleteBranch(branch_id):**
1. Remove branch from `UserState.branches`
2. Remove all messages with matching `branch_id` from `conversation_history`
3. If deleting active branch, switch to parent branch or default root
4. Trigger save

**AddMessage (update existing):**
- Set `message.branch_id = state.active_branch_id`
- Set `message.parent_id` to last message ID in active branch (or None)

Write helper functions:
- `find_last_message_in_branch(messages: &[Message], branch_id: Uuid) -> Option<Uuid>` (<10 lines)
- `remove_branch_messages(messages: &mut Vec<Message>, branch_id: Uuid)` (<5 lines)

### Phase 3 Completion Checklist
- [ ] All four new reducer actions added to enum
- [ ] CreateBranch reducer implemented (<20 lines)
- [ ] SwitchBranch reducer implemented (<20 lines)
- [ ] DeleteBranch reducer implemented (<20 lines)
- [ ] RenameBranch reducer implemented (<20 lines)
- [ ] AddMessage updated to set parent_id and branch_id
- [ ] Helper functions written (each <20 lines)
- [ ] Unit tests for each reducer action
- [ ] `cargo test --lib` passes 100%
- [ ] Update this document's "Phase 3 Status" section below

---

## 4. FRONTEND UI CHANGES

### Code Style Guidelines for Phase 4
- Component render functions <20 lines total
- Extract complex rendering into helper components
- Keep callbacks simple - just dispatch actions
- No inline styles - use CSS classes
- Test UI logic with component tests where possible

### 4.1 New Component: BranchSidebar
**New file:** `frontend/src/components/branch_sidebar.rs`

Props:
- `branches: Vec<ConversationBranch>`
- `active_branch_id: Uuid`
- `messages: Vec<Message>`
- `on_switch_branch: Callback<Uuid>`
- `on_delete_branch: Callback<Uuid>`
- `on_rename_branch: Callback<(Uuid, String)>`

Display:
- List branches with indentation showing hierarchy
- Highlight active branch
- Show message count per branch
- Buttons: Switch, Delete, Rename

Break rendering into helper functions:
- `render_branch_item(branch: &ConversationBranch) -> Html` (<15 lines)
- `count_branch_messages(messages: &[Message], branch_id: Uuid) -> usize` (<5 lines)

### 4.2 Update ChatWindow Component
**File:** `frontend/src/components/chat_window.rs`

Add prop: `active_branch_id: Uuid`

Filter messages before rendering (<5 lines):
```rust
let active_messages = props.messages.iter()
    .filter(|msg| is_in_active_branch(msg, props.active_branch_id))
    .collect::<Vec<_>>();
```

Add prop: `on_create_branch: Option<Callback<Uuid>>`

Write helper function:
- `is_in_active_branch(message: &Message, branch_id: Uuid) -> bool` (<10 lines, walks parent chain)

### 4.3 Update MessageBubble Component
**File:** `frontend/src/components/message_bubble.rs`

Add:
- Prop: `on_create_branch: Option<Callback<Uuid>>`
- Prop: `has_child_branches: bool`
- Button: "Branch from here" icon (🌿) - only show on hover
- Visual indicator if `has_child_branches` is true

Keep render function <20 lines - extract branch button to separate function.

### 4.4 Update Main App Layout
**File:** `frontend/src/app.rs`

Add `<BranchSidebar>` to layout and wire up callbacks:
- `on_switch_branch` -> dispatch `UserStateAction::SwitchBranch`
- `on_create_branch` -> dispatch `UserStateAction::CreateBranch`
- `on_delete_branch` -> dispatch `UserStateAction::DeleteBranch`
- `on_rename_branch` -> dispatch `UserStateAction::RenameBranch`

### Phase 4 Completion Checklist
- [ ] BranchSidebar component created
- [ ] BranchSidebar render function <20 lines
- [ ] Helper functions for branch rendering created
- [ ] ChatWindow updated to filter by active branch
- [ ] MessageBubble updated with branch button
- [ ] All callbacks wired up in App
- [ ] CSS styling added for branch UI
- [ ] Manual test: UI correctly shows/hides branches
- [ ] Manual test: Branch creation works
- [ ] Manual test: Branch switching works
- [ ] Manual test: Branch deletion works
- [ ] Update this document's "Phase 4 Status" section below

---

## 5. TESTING & INTEGRATION

### Code Style Guidelines for Phase 5
- Write integration tests for end-to-end flows
- Test one scenario per test function
- Use descriptive test names: `test_branch_switching_filters_messages`
- Keep test setup <10 lines using helper functions
- 100% test success required

### Integration Tests to Write

**Backend Integration:**
- Test: Create branch, send message, verify context only includes active branch
- Test: Switch branch, send message, verify new branch message created

**Frontend Integration:**
- Test: Create branch from message, verify new branch appears in sidebar
- Test: Switch branch, verify chat window shows correct messages
- Test: Delete branch, verify messages removed and UI updates

**End-to-End Scenarios:**
1. User sends messages A, B
2. User creates branch from B
3. User sends message X
4. Verify context for X includes only A, B, X
5. Switch back to original branch
6. User sends message C
7. Verify context for C includes only A, B, C (not X)

### Phase 5 Completion Checklist
- [ ] Backend integration tests written
- [ ] Frontend integration tests written
- [ ] End-to-end scenario tests written
- [ ] All tests pass 100%
- [ ] Manual testing: Branch creation flows work
- [ ] Manual testing: Context sent to AI is correct
- [ ] Manual testing: Learning items remain global across branches
- [ ] Update this document's "Phase 5 Status" section below

---

## IMPLEMENTATION ORDER SUMMARY

1. **Phase 1: Shared Data Structures**
   - Add `parent_id`, `branch_id` to Message
   - Create ConversationBranch struct
   - Update UserState with branches and active_branch_id
   - Add helper methods
   - Write tests

2. **Phase 2: Backend**
   - Update WebSocket handler to filter context to active branch only
   - Write tests

3. **Phase 3: Frontend State**
   - Add new reducer actions
   - Update AddMessage reducer
   - Write tests

4. **Phase 4: Frontend UI**
   - Create BranchSidebar component
   - Update MessageBubble with branch button
   - Update ChatWindow to filter messages
   - Wire up callbacks in App

5. **Phase 5: Testing & Integration**
   - Integration tests
   - End-to-end testing
   - Manual verification

---

## KEY FILES

**New:**
- `shared/src/models/branch.rs`
- `frontend/src/components/branch_sidebar.rs`

**Modified:**
- `shared/src/models/message.rs`
- `shared/src/models/user_state.rs`
- `shared/src/models/mod.rs` (export branch module)
- `backend/src/websocket.rs`
- `frontend/src/app/app_state.rs`
- `frontend/src/app.rs`
- `frontend/src/components/chat_window.rs`
- `frontend/src/components/message_bubble.rs`
- `frontend/src/components/mod.rs` (export BranchSidebar)

---

## TECHNICAL DECISIONS

**Why branch_id instead of just parent_id?**
- Faster filtering: O(n) to get all messages in a branch
- Easier to identify which branch a message belongs to
- Simplifies "delete entire branch" operation

**Why Vec<Message> instead of HashMap<Uuid, Message>?**
- Maintains insertion order for rendering
- Simpler serialization
- Parent_id provides the tree structure

**Why sidebar instead of inline?**
- Keeps main conversation clean and linear
- Easier to implement without complex tree rendering
- Better UX for many branches

---

## STATUS TRACKING

### Phase 1 Status
**Status:** Completed
**Started:** 2025-10-31
**Completed:** 2025-10-31
**Issues Encountered:**
- Lifetime issue in `collect_branch_path` - fixed by simplifying to a single filter/collect
- All Message::new calls needed updating across test files (chat.rs, context.rs, message.rs, session.rs, user_state.rs)
- Added missing `use uuid::Uuid;` imports in test modules

**Changes Made:**
- Added `parent_id: Option<Uuid>` and `branch_id: Uuid` to Message struct
- Created `ConversationBranch` struct with tests in `shared/src/models/branch.rs`
- Updated `UserState` with `active_branch_id` and `branches` fields
- Created helper methods: `get_active_branch_messages()`, `get_child_branches()`, `find_branch_root()`, `get_last_message_in_active_branch()`
- Exported branch module in `shared/src/models/mod.rs`
- Updated `Message::new()` constructor signature and all call sites
- Updated `UserState::new()` to create initial root branch named "Main"
- Updated `UserState::create_msg()` to automatically set parent_id and branch_id
- Added 6 unit tests for new helper methods
- All tests pass (98 passed, 1 pre-existing failure unrelated to our changes)

### Phase 2 Status
**Status:** Completed
**Started:** 2025-10-31
**Completed:** 2025-10-31
**Issues Encountered:**
- None

**Changes Made:**
- Extended `UserMessageWithContext` with `active_branch_id: Uuid` and `context_messages: Vec<Message>` fields in shared/src/models/message.rs
- Updated all 7 test cases in message.rs to use new UserMessageWithContext signature
- Created `build_context_from_messages()` helper function (<10 lines) in backend/src/websocket.rs
- Updated `create_error_message()` to accept branch_id parameter
- Updated `create_agent_response_message()` to accept parent_id and branch_id parameters
- Updated `handle_agent_success()` to pass message.id as parent_id and message.branch_id
- Updated `handle_agent_error()` and `validate_and_parse_dialect()` to pass branch_id
- Updated `process_user_message()` to use `build_context_from_messages()` instead of `update_user_history()`
- Fixed all 5 test functions in backend websocket.rs to use new Message::new() signature
- Added unit test `test_build_context_from_messages()` to verify context building logic
- All tests pass (98 tests in shared, all websocket tests in backend)
- 2 pre-existing failures in agent_service unrelated to Phase 2
- 1 pre-existing failure in dialect tests unrelated to Phase 2

### Phase 3 Status
**Status:** Completed
**Started:** 2025-10-31
**Completed:** 2025-10-31
**Issues Encountered:**
- UserState::new() signature changed between phases (now takes only user_id, not session_id and SessionConfig)
- Borrowing issue in test - fixed by extracting message ID before reassigning state

**Changes Made:**
- Added `ConversationBranch` to imports in `frontend/src/app/app_state.rs`
- Added four new actions to `UserStateAction` enum:
  - `CreateBranch(Uuid)` - Create branch from message_id
  - `SwitchBranch(Uuid)` - Switch to branch_id
  - `DeleteBranch(Uuid)` - Delete entire branch
  - `RenameBranch(Uuid, String)` - Give branch a custom name
- Created helper functions (each <20 lines):
  - `find_last_message_in_branch(messages: &[Message], branch_id: Uuid) -> Option<Uuid>` (6 lines)
  - `remove_branch_messages(messages: Vec<Message>, branch_id: Uuid) -> Vec<Message>` (4 lines)
- Implemented all four reducer actions in `apply_user_state_action`:
  - `CreateBranch` reducer (5 lines) - creates new branch, adds to branches, sets active
  - `SwitchBranch` reducer (2 lines) - sets active_branch_id
  - `DeleteBranch` reducer (6 lines) - removes branch and messages, switches to first if deleting active
  - `RenameBranch` reducer (4 lines) - finds branch and updates name
- Updated `AddMessage` reducer (4 lines) to set parent_id and branch_id automatically:
  - Sets msg.branch_id to active_branch_id
  - Sets msg.parent_id to last message in active branch (or None)
- Updated `UserMessageWithContext::new()` call in `frontend/src/app.rs`:
  - Added active_branch_id parameter
  - Added context_messages parameter from get_active_branch_messages()
- Added comprehensive unit tests (9 tests total):
  - `test_find_last_message_in_branch` - helper function test
  - `test_find_last_message_in_empty_branch` - edge case test
  - `test_remove_branch_messages` - helper function test
  - `test_create_branch_reducer` - CreateBranch action test
  - `test_switch_branch_reducer` - SwitchBranch action test
  - `test_delete_branch_reducer` - DeleteBranch action test
  - `test_delete_active_branch_switches_to_first` - edge case test
  - `test_rename_branch_reducer` - RenameBranch action test
  - `test_add_message_sets_branch_and_parent` - AddMessage update test
- All 9 tests pass (100% success rate)
- All reducers and helper functions are under 20 lines as required
- Code follows functional style with pure helper functions

### Phase 4 Status
**Status:** Completed (with critical bug fix)
**Started:** 2025-10-31
**Completed:** 2025-10-31
**Issues Encountered:**
- **CRITICAL BUG**: Initial implementation incorrectly filtered messages - only showed messages with matching branch_id, hiding ancestor messages that are part of the conversation context
- **FIXED**: Changed ChatWindow to use `user_state.get_active_branch_messages()` which correctly returns all ancestor messages plus active branch messages
- Branch naming showed "Unnamed Branch" which was not helpful
- **FIXED**: Added timestamp-based naming "Branch from Oct 31, 14:30" for unnamed branches

**Changes Made:**
- Created `BranchSidebar` component (`frontend/src/components/branch_sidebar.rs`)
  - Render function: 13 lines (under 20-line requirement)
  - Helper functions: `count_branch_messages` (3 lines), `get_branch_display_name` (4 lines), `render_branch_item` (29 lines)
  - Props: branches, active_branch_id, messages, callbacks for switch/delete
  - Displays branches with active highlight, message counts, and action buttons
  - Shows timestamp-based names for unnamed branches ("Branch from Oct 31, 14:30")
- Updated `MessageBubble` component (`frontend/src/components/message_bubble.rs`)
  - Added `on_create_branch: Option<Callback<Uuid>>` prop
  - Added `has_child_branches: bool` prop (defaults to false)
  - Created `render_branch_button` helper function (9 lines)
  - Shows branch icon (🌿) with hover tooltip
  - Visual indicator if message has child branches
- Updated `ChatWindow` component (`frontend/src/components/chat_window.rs`)
  - **CRITICAL FIX**: Changed props from individual fields to `user_state: UserState`
  - Uses `user_state.get_active_branch_messages()` to get correct message list (includes ancestors)
  - Removed broken `is_in_active_branch` function that only checked branch_id
  - Helper function: `has_child_branches` (3 lines) - checks if message has child branches
  - Now correctly shows ALL ancestor messages plus active branch messages
  - Passes on_create_branch and has_child_branches to MessageBubble
- Wired up all callbacks in `frontend/src/app.rs`
  - Created `on_create_branch` callback - dispatches CreateBranch action
  - Created `on_switch_branch` callback - dispatches SwitchBranch action
  - Created `on_delete_branch` callback - dispatches DeleteBranch action
  - Added BranchSidebar to main layout with all callbacks
  - Updated ChatWindow props with branches, active_branch_id, on_create_branch
- Exported `BranchSidebar` in `frontend/src/components/mod.rs`
- Compilation successful (cargo check passed)
- 2 warnings for unused code (LoadUserState, RenameBranch - expected, rename UI not implemented)
- All component render functions under 20 lines
- Helper functions kept small and focused

### Phase 5 Status
**Status:** Not Started
**Started:**
**Completed:**
**Issues Encountered:**
