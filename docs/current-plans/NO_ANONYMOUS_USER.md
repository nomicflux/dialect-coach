# No Anonymous User - Architecture Fix

**Status**: In Progress
**Started**: 2025-11-01

## Problem Statement

Currently, the application creates an anonymous user with full UserState (session, branches, conversation history) immediately on startup. This is architecturally incorrect.

**Current (WRONG) behavior:**
- App loads → immediately creates UserState with user_id
- UserState contains session_id, branches, messages, etc.
- All WebSocket connections established immediately
- Chat system is "active" even though no user is signed in

**Desired (CORRECT) behavior:**
- App loads → NO UserState, NO session exists
- Only sign-in/create account forms available
- After successful authentication → create/load UserState and session
- Only then establish chat WebSocket connections
- On sign-out → completely destroy UserState and session

## Implementation Plan

### Phase 1: Update AppState for Optional Session
**Status**: ✅ Complete

**Files**: `frontend/src/app/app_state.rs`

**Code Style Checklist**:
- ✅ Functions <20 lines
- ✅ Helper functions for complex logic
- ✅ Pure functions where possible
- ✅ No defensive coding
- ✅ Tests for new functions

**Changes Made**:
1. Line 37: Changed `session_id: Uuid` to `session_id: Option<Uuid>`
2. Line 53: Changed default from `Uuid::new_v4()` to `None`
3. Line 82-84: Updated `session_id()` method to return `Option<Uuid>`
4. Added new actions (lines 31-32):
   - `CreateSession(Uuid)` - sets session_id to Some(uuid)
   - `DestroySession` - sets session_id to None
5. Implemented reducers for these actions (lines 125-130)
6. Temporary fix in app.rs:58 to handle Option<Uuid> (will be properly fixed in Phase 2)

**Tests**: ✅ All 12 tests pass

**Completion Criteria**:
- ✅ session_id is Optional in AppState
- ✅ Default creates None
- ✅ CreateSession/DestroySession actions work
- ✅ All tests pass

---

### Phase 2: Make UserState Optional in App
**Status**: ✅ Complete

**Files**: `frontend/src/app.rs`, `frontend/src/app/app_state.rs`, `frontend/src/hooks/use_debounced_save.rs`

**Code Style Checklist**:
- ✅ Functions <20 lines
- ✅ Helper functions for complex logic
- ✅ Pure functions where possible
- ✅ No defensive coding
- ⬜ Tests for new functions (will test after all phases complete)

**Changes Made**:
1. ✅ Created `OptionalUserState` wrapper in app_state.rs (lines 488-510)
   - Implements Reducible for Option<UserState>
   - Handles ReplaceUserState and ClearUserState specially
   - Other actions only apply if Some(state)

2. ✅ Added `ClearUserState` action to UserStateAction enum (line 247)
   - Added exhaustive match case in apply_user_state_action (line 458-462)

3. ✅ Removed `get_or_create_user_id()` function from app.rs

4. ✅ Changed user_state initialization to `OptionalUserState(None)` (line 511-514)

5. ✅ Made OptionalUserState public from app.rs module

6. ✅ Updated `extract_learning_items()` to handle OptionalUserState (line 15)

7. ✅ Updated ALL 18 callback function signatures to use `UseReducerHandle<OptionalUserState>`:
   - `on_send_message`, `on_prompt_click`, `on_language_change`, `on_dialect_change`
   - `on_formality_change`, `on_teaching_mode_change`, `on_tts_toggle`
   - `on_dialect_cycle`, `on_formality_cycle`, `on_teaching_mode_cycle`
   - `on_user_state_load_response`, `on_create_user_click`, `on_signout_click`
   - `on_delete_message_callback`, `on_undo_message_callback`
   - `on_create_branch`, `on_switch_branch`, `on_delete_branch`

8. ✅ Added early return/None checks in all callback bodies

9. ✅ Updated all field accesses to use `.0.as_ref()` pattern with unwrap_or defaults

10. ✅ Updated WebSocket message handler (line 563) - tts_enabled check

11. ✅ Updated user_state_ws connection (line 633-641) - conditional connection based on state

12. ✅ Updated component props with proper None handling:
    - BranchSidebar (lines 765-767)
    - ChatWindow (line 796)
    - SpeechControls (lines 806-809)
    - LearningPanel (lines 866, 877)
    - Settings panel dialect select (lines 941-955)

13. ✅ Updated use_debounced_save hook to handle OptionalUserState
    - Changed signature to accept `UseReducerHandle<OptionalUserState>`
    - Only triggers saves when state is Some

14. ✅ Updated on_signout_click to properly clear state:
    - Dispatches DestroySession
    - Dispatches ClearUserState (not ReplaceUserState)
    - Dispatches ClearUser

**Compilation Status**: ✅ Compiles successfully (0 errors, only backend warnings)

**Completion Criteria**:
- ✅ user_state starts as None
- ✅ get_or_create_user_id removed
- ✅ OptionalUserState type created
- ✅ All callbacks handle None gracefully
- ✅ Code compiles without errors

---

### Phase 3: Conditional WebSocket Connections
**Status**: ⬜ Not Started

**Files**: `frontend/src/app.rs`

**Code Style Checklist** (complete before starting):
- [ ] Functions <20 lines
- [ ] Helper functions for complex logic
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] Tests for new functions

**Changes**:
1. Lines 499-591 (Chat WebSocket effect):
   - Add `user_state.is_some()` to effect dependencies
   - Only establish connection if authenticated
   - Disconnect on None

2. Lines 593-612 (UserState WebSocket effect):
   - Add `user_state.is_some()` to effect dependencies
   - Only establish connection if authenticated
   - Disconnect on None

3. Lines 614-632 (User WebSocket effect):
   - Keep unchanged (needed for authentication flow)

**Tests**: Manual testing of WebSocket connection lifecycle

**Completion Criteria**:
- [ ] Chat WS only connects when authenticated
- [ ] UserState WS only connects when authenticated
- [ ] User WS connects on mount
- [ ] WebSockets disconnect on sign-out

---

### Phase 4: Update Authentication Flow
**Status**: ⬜ Not Started

**Files**: `frontend/src/app.rs`, `frontend/src/app/app_state.rs`

**Code Style Checklist** (complete before starting):
- [ ] Functions <20 lines
- [ ] Helper functions for complex logic
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] Tests for new functions

**Changes in app_state.rs**:
1. Add `ClearUserState` action to UserStateAction enum
2. Implement reducer that sets state to None (not fresh state)

**Changes in app.rs**:
1. Lines 349-366 (`on_user_create_response`):
   - After `SetUser`, create session: `app_state.dispatch(AppStateAction::CreateSession(Uuid::new_v4()))`
   - Create UserState: `user_state.dispatch(UserStateAction::ReplaceUserState(UserState::new(user.id)))`

2. Lines 368-385 (`on_user_signin_response`):
   - After `SetUser`, create session: `app_state.dispatch(AppStateAction::CreateSession(Uuid::new_v4()))`
   - Create UserState: `user_state.dispatch(UserStateAction::ReplaceUserState(UserState::new(user.id)))`
   - UserState will be loaded from backend via WebSocket

3. Lines 387-396 (`on_signout_click`):
   - Destroy session: `app_state.dispatch(AppStateAction::DestroySession)`
   - Clear UserState: `user_state.dispatch(UserStateAction::ClearUserState)`
   - Clear user: `app_state.dispatch(AppStateAction::ClearUser)`

**Tests**: Manual testing of auth flow

**Completion Criteria**:
- [ ] Sign-in creates session and UserState
- [ ] Create account creates session and UserState
- [ ] Sign-out destroys session and UserState
- [ ] No errors during auth flow

---

### Phase 5: Conditional UI Rendering
**Status**: ⬜ Not Started

**Files**: `frontend/src/app.rs`

**Code Style Checklist** (complete before starting):
- [ ] Functions <20 lines
- [ ] Helper functions for complex logic
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] Tests for new functions

**Changes**:
1. Lines 726-956 (entire `<main>` section):
   - Wrap in: `{if let Some(us) = user_state.as_ref() {`
   - Inside: render all chat UI (BranchSidebar, ChatWindow, etc.)
   - Use `us` reference instead of `user_state` for component props
   - After closing brace: add `} else { /* welcome message */ }`

2. Welcome message when None:
   ```rust
   html! {
       <div class="welcome-container">
           <h2>{"Welcome to Dialect Coach"}</h2>
           <p>{"Please sign in or create an account above to start practicing."}</p>
       </div>
   }
   ```

**Tests**: Manual testing of UI visibility

**Completion Criteria**:
- [ ] Fresh load shows only sign-in forms
- [ ] After auth, chat UI appears
- [ ] After sign-out, chat UI disappears
- [ ] Welcome message shows when not authenticated

---

### Phase 6: Final Testing & Verification
**Status**: ⬜ Not Started

**Testing Scenarios**:
1. **Fresh page load**:
   - ✅ No chat UI visible
   - ✅ Only sign-in/create forms shown
   - ✅ No WebSocket connections except User WS

2. **Sign in flow**:
   - ✅ Enter username, click Sign In
   - ✅ UserState created with user's user_id
   - ✅ Session created with new UUID
   - ✅ Chat UI appears
   - ✅ Chat and UserState WebSockets connect

3. **Create account flow**:
   - ✅ Enter username, click Create Account
   - ✅ UserState created with user's user_id
   - ✅ Session created with new UUID
   - ✅ Chat UI appears
   - ✅ Chat and UserState WebSockets connect

4. **Using the chat**:
   - ✅ Send messages
   - ✅ Create branches
   - ✅ Change settings
   - ✅ Everything works normally

5. **Page refresh while signed in**:
   - ✅ Sign-in forms show briefly
   - ✅ User signs in again
   - ✅ UserState loaded from backend
   - ✅ All data persists

6. **Sign out flow**:
   - ✅ Click Sign Out
   - ✅ Chat UI disappears
   - ✅ Back to sign-in forms
   - ✅ WebSockets disconnected (except User WS)
   - ✅ No errors in console

7. **Run all tests**:
   - ✅ `cargo test --lib` in frontend
   - ✅ `cargo test --lib` in shared
   - ✅ All tests pass

**Completion Criteria**:
- [ ] All test scenarios pass
- [ ] No errors in browser console
- [ ] No errors in backend logs
- [ ] 100% test pass rate

---

## Files Modified

1. `frontend/src/app/app_state.rs`:
   - session_id: Option<Uuid>
   - CreateSession, DestroySession actions
   - ClearUserState action

2. `frontend/src/app.rs`:
   - user_state: Option<UserStateWrapper>
   - Removed get_or_create_user_id
   - Updated 18+ callback functions
   - Conditional WebSocket effects
   - Updated auth handlers
   - Conditional UI rendering

## Breaking Changes

- `AppState.session_id` is now `Option<Uuid>` instead of `Uuid`
- `user_state` is now `Option<UserStateWrapper>` instead of `UserStateWrapper`
- localStorage user_id persistence removed (no more anonymous users)
- All callbacks must handle None case

## Notes

- Old approach: Every visitor was an "anonymous user" with full state
- New approach: No user/session until authentication
- This is a fundamental architectural fix, not just a cosmetic change
