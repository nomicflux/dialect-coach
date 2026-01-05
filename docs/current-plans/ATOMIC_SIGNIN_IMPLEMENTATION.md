# Atomic Sign-In Implementation Plan

## Problem Statement

After user creation, header shows signed-in but main content shows WelcomeScreen. This persists after hard reload.

**Root Cause**: Two-phase loading gap where `current_user` is set immediately but `session.user` (UserState) loads asynchronously via a separate WebSocket round-trip.

**Current Flow**:
1. Auth handler returns `(User, token)` or `User`
2. Frontend sets `current_user` via `SetUser` action
3. `SetUser` triggers `load_user_state()` which sends `UserStateMessage::Load(user_id)` over separate WebSocket
4. Backend returns `LoadResponse(Some(UserState))` or `LoadResponse(None)` for new users
5. Frontend sets `session.user` via `SessionAction::UpdateUser`

**The Gap**: Between steps 2 and 5, `current_user.is_some()` (header shows signed-in) but `session.user.is_none()` (main content shows WelcomeScreen).

## State Management Principles (User-Specified - Inviolable)

1. USER IS CREATED. THIS INVOLVES CREATING INITIAL USER STATE. BOTH CREATED. DONE.
2. USER SIGNS IN. USER AND USER STATE LOADING IN ATOMIC STEP. DONE.
3. USER STATE IS DIRTY IN THE FRONTEND. FRONTEND SAVES USER STATE TO PERSISTENCE. DONE.
4. SOME ADDITIONAL STATS ARE CREATED FOR USAGE DURING CALLS. STATS SAVED INDEPENDENTLY. DONE.

**Key Guarantees**:
- A User ALWAYS has UserState - rock-solid guarantee for the entire app
- ONLY exception: brief moment during User creation as part of atomic step that rest of app cannot see
- User and UserState are SEPARATE data structures (do NOT merge them)
- "There is no 'current user' vs 'session user'. There is just the user."

## DO NOT

- Change persistence layer
- Change database schema
- Remove user state WebSocket (still needed for saves/stats)
- Add fallback logic for "missing" UserState
- Merge User/UserState structures

## Solution Architecture

### Backend Changes (`backend/src/websocket/user.rs`)

**Core principle**: Session restoration on page reload is just signin via session token. ONE function signs in, all paths use it.

**New helper function** `sign_in_user(state: &AppState, user_id: Uuid) -> Result<(User, UserState, String), String>`:
- Load User from persistence
- Load UserState from persistence
- Populate `user_state.is_admin` from user record
- Generate JWT token
- Return `(User, UserState, token)`

**handle_create_user**:
- Create User
- Create initial UserState with `initial_settings`
- Save both to persistence
- Call `sign_in_user(state, user.id)`
- Return result

**handle_sign_in**:
- Authenticate username/password (get user_id)
- Call `sign_in_user(state, user_id)`
- Return result

**handle_validate_session**:
- Validate token (get user_id)
- Call `sign_in_user(state, user_id)`
- Return result

All three handlers return identical types: `(User, UserState, token)`

### Shared Types Changes (`shared/src/models/message.rs`)

Modify `UserMessage` enum:
```rust
// BEFORE:
CreateUserResponse(Result<(User, String), String>),
SignInResponse(Result<(User, String), String>),
ValidateSessionResponse(Result<User, String>),

// AFTER:
CreateUserResponse(Result<(User, UserState, String), String>),
SignInResponse(Result<(User, UserState, String), String>),
ValidateSessionResponse(Result<(User, UserState), String>),
```

### Frontend Changes (`frontend/src/app/callbacks.rs`)

**on_user_create_response**:
- Receives `(User, UserState, token)`
- Sets token in cookies
- Sets `current_user` AND `session.user` atomically (or in immediate succession without WebSocket gap)

**on_user_signin_response**:
- Receives `(User, UserState, token)`
- Sets token in cookies
- Sets `current_user` AND `session.user` atomically

**on_validate_session_response**:
- Receives `(User, UserState)`
- Sets `current_user` AND `session.user` atomically

### Frontend WebSocket Changes

Update `UserWebSocketService` to handle new response types in `process_message`.

## Implementation Phases

### Phase 1: Shared Types Update

**Agent**: kiss-code-generator

**Code Style Checklist**:
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No dead code

**Files to modify**:
- `shared/src/models/message.rs`

**Tasks**:
1. Change `CreateUserResponse` to `Result<(User, UserState, String), String>`
2. Change `SignInResponse` to `Result<(User, UserState, String), String>`
3. Change `ValidateSessionResponse` to `Result<(User, UserState), String>`
4. Update existing tests to use new tuple types

**Deliverables**:
- All message types updated
- All tests in `shared` pass

**End of Phase**:
- Run `cargo test -p dialect-coach-shared`
- Run `cargo clippy -p dialect-coach-shared` - fix all errors
- Update this document with status
- `git add shared && git commit -m "Phase 1 (shared types for atomic signin) complete"`

### Phase 2: Backend Handler Updates

**Agent**: kiss-code-generator

**Code Style Checklist**:
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No dead code

**Files to modify**:
- `backend/src/websocket/user.rs`

**Tasks**:
1. Update `handle_create_user`:
   - After creating user, create `UserState::with_initial_settings(user.id, initial_settings)`
   - Save the UserState via `state.user_persistence.save(&user_state)`
   - Return `CreateUserResponse(Ok((user, user_state, token)))`

2. Update `handle_sign_in`:
   - After authenticating, load UserState via `state.user_persistence.load(user.id)`
   - Handle the case where UserState doesn't exist (shouldn't happen, but be safe)
   - Return `SignInResponse(Ok((user, user_state, token)))`

3. Update `handle_validate_session`:
   - After loading user, load UserState via `state.user_persistence.load(user_id)`
   - Populate `is_admin` from user record (already done in `load_user_state_response`)
   - Return `ValidateSessionResponse(Ok((user, user_state)))`

4. Update tests to use new response types

**Deliverables**:
- All three handlers return `(User, UserState, ...)` tuples
- UserState created and saved during user creation
- All tests in `backend` pass

**End of Phase**:
- Run `cargo test -p dialect-coach-backend`
- Run `cargo clippy -p dialect-coach-backend` - fix all errors
- Update this document with status
- `git add backend && git commit -m "Phase 2 (backend atomic signin) complete"`

### Phase 3: Frontend Callback Updates

**Agent**: kiss-code-generator

**Code Style Checklist**:
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No dead code

**Files to modify**:
- `frontend/src/app/callbacks.rs`
- `frontend/src/services/user_websocket.rs` (if needed for type changes)

**Tasks**:
1. Update `on_user_create_response`:
   - Change callback type from `Result<(User, String), String>` to `Result<(User, UserState, String), String>`
   - Set token in cookies
   - Dispatch `AppStateAction::SetUser(user)` - but modify to NOT trigger `load_user_state()` since we already have it
   - Dispatch `SessionAction::Login(user_state)` or `SessionAction::UpdateUser(user_state)` to set session.user

2. Update `on_user_signin_response`:
   - Same pattern as create

3. Update `on_validate_session_response`:
   - Change callback type from `Result<User, String>` to `Result<(User, UserState), String>`
   - Same pattern as above

4. Update `UserWebSocketService::process_message` to handle new types

**Considerations**:
- Currently `SetUser` calls `load_user_state()` which we no longer need for signin flows
- Options:
  a. Create `SetUserWithState(User, UserState)` action that sets both without WS call
  b. Modify `SetUser` to optionally skip WS call
  c. Have callbacks set `current_user` and `session.user` separately in immediate succession

**Deliverables**:
- All auth callbacks set both `current_user` and `session.user` atomically
- No WelcomeScreen flash after signin

**End of Phase**:
- Run `cargo test -p dialect-coach-frontend`
- Run `cargo clippy -p dialect-coach-frontend` - fix all errors
- Update this document with status
- `git add frontend && git commit -m "Phase 3 (frontend atomic signin) complete"`

### Phase 4: Remove Obsolete Frontend UserState Creation

**Agent**: kiss-code-generator

**Code Style Checklist**:
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No dead code

**Files to modify**:
- `frontend/src/app/app_state/callbacks.rs`
- `frontend/src/app/app_state/app.rs`

**Tasks**:
1. In `on_user_state_load_response` (`app_state/callbacks.rs`):
   - Remove the "New user - create initial UserState" branch (lines 46-53)
   - This is now handled by backend during user creation
   - Keep the existing user path that dispatches `UpdateUser`

2. In `AppStateAction::SetUser` handler (`app_state/app.rs`):
   - Consider whether `load_user_state()` should still be called
   - For atomic signin, we don't need it (UserState comes with auth response)
   - But we may still want it for edge cases (reconnection?)
   - Decision: Keep `load_user_state()` for now - it will receive `LoadResponse(Some(user_state))` and update, which is fine even if already set

3. Remove `pending_initial_settings` handling since it's no longer needed:
   - Remove `StorePendingInitialSettings` action usage in signin flows
   - Keep the field for now if it's used elsewhere

**Deliverables**:
- No frontend-side UserState creation for new users
- Clean up obsolete code paths

**End of Phase**:
- Run `cargo test -p dialect-coach-frontend`
- Run `cargo clippy -p dialect-coach-frontend` - fix all errors
- Update this document with status
- `git add frontend && git commit -m "Phase 4 (cleanup obsolete user state creation) complete"`

### Phase 5: Integration Testing

**Agent**: modular-builder

**Tasks**:
1. Manual testing:
   - Create new user -> should show main content immediately, no WelcomeScreen flash
   - Sign in as existing user -> should show main content immediately
   - Reload page with valid token -> should show main content immediately

2. Verify:
   - UserState persists correctly after creation
   - UserState loads correctly on signin
   - UserState WebSocket still works for saves
   - Usage stats still work

**End of Phase**:
- Run full test suite: `cargo test`
- Run `cargo clippy` - fix all errors
- Update this document with final status
- `git add . && git commit -m "Phase 5 (atomic signin complete) complete"`

## Status

- [ ] Phase 1: Shared Types Update - NOT STARTED
- [ ] Phase 2: Backend Handler Updates - NOT STARTED
- [ ] Phase 3: Frontend Callback Updates - NOT STARTED
- [ ] Phase 4: Remove Obsolete Frontend UserState Creation - NOT STARTED
- [ ] Phase 5: Integration Testing - NOT STARTED

## Notes

### Key Files Reference

- Backend handlers: `backend/src/websocket/user.rs`
- Shared message types: `shared/src/models/message.rs`
- Frontend callbacks: `frontend/src/app/callbacks.rs`
- Frontend app state: `frontend/src/app/app_state/app.rs`
- Frontend session callbacks: `frontend/src/app/app_state/callbacks.rs`
- UserState model: `shared/src/models/user_state.rs`
- User persistence: `backend/src/persistence/` (DO NOT MODIFY)

### Existing Tests to Update

Backend tests in `backend/src/websocket/user.rs`:
- `test_send_user_message_sign_in_response_err`
- `test_send_user_message_serialization`
- `test_send_user_message_create_user_response_ok`

Shared tests in `shared/src/models/message.rs`:
- `test_user_message_create_user_response_ok`
- `test_user_message_sign_in_response_ok`
- `test_user_message_validate_session_response_ok`
