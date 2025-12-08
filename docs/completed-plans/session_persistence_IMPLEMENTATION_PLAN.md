# Session Cookie Persistence - Implementation Plan

## User Requirements (2025-12-01)

> "How do we set session cookies in the frontend to keep a user logged in? When I reload the page, I want the current user to remain logged in unless they explicitly logout, or before an expiration period (24 hours)."

**User's Exact Design Choices:**
- JWT tokens (stateless, self-contained)
- JS-accessible cookies (frontend can read/write)
- Add password authentication (not username-only)
- WebSocket message for session restoration (not HTTP endpoint)
- 24-hour expiration period

## Current State

**Authentication Flow:**
1. CreateUser: username + email + invite code → User created (no password)
2. SignIn: username only → User returned (no verification)
3. User stored in `AppState.current_user: Option<User>` (in-memory only)
4. Page reload → AppState reset → User lost

**No Session Persistence:**
- No cookies
- No session tokens
- No password storage/verification

## Architecture Changes

### New Shared Types
- `SessionToken` struct with JWT string
- `UserMessage::ValidateSession { token: String }` - sent on page load
- `UserMessage::ValidateSessionResponse(Result<User, String>)` - returns User if valid
- `AuthCredentials::Password(String)` variant
- Add `password_hash` field to User database records (NOT in User type)

### Backend Changes
- Add `jsonwebtoken` and `bcrypt` dependencies
- Password hashing on user creation
- Password verification on sign-in
- JWT generation on successful auth
- JWT validation on ValidateSession message
- 24-hour expiration in JWT claims

### Frontend Changes
- Cookie management via `web-sys` Document.cookie API
- Store JWT in cookie on auth success
- Read JWT from cookie on app load
- Send ValidateSession message if cookie exists
- Clear cookie on logout

---

## Phase 1: Backend Password & JWT Infrastructure

**Subagent:** modular-builder

**Code Style Checklist:**
- [ ] Functions <20 lines
- [ ] Helper functions for complex logic
- [ ] Pure functions for data transformation
- [ ] No defensive coding
- [ ] No dead code
- [ ] Tests for all new functions

**Deliverables:**
1. Dependencies added to `backend/Cargo.toml`
2. JWT token generation/validation functions
3. Password hashing/verification functions
4. Unit tests for crypto functions

**Files to Update/Create:**
- `backend/Cargo.toml` - add `jsonwebtoken = "9"`, `bcrypt = "0.15"`
- `backend/src/crypto/mod.rs` (NEW) - crypto utilities module
- `backend/src/crypto/jwt.rs` (NEW) - JWT functions
- `backend/src/crypto/password.rs` (NEW) - password hashing functions
- `backend/tests/crypto_tests.rs` (NEW) - unit tests

**Tasks:**
1. Add dependencies to `backend/Cargo.toml`:
   ```toml
   jsonwebtoken = "9"
   bcrypt = "0.15"
   ```

2. Create `backend/src/crypto/mod.rs`:
   ```rust
   pub mod jwt;
   pub mod password;
   ```

3. Create `backend/src/crypto/password.rs`:
   - `hash_password(password: &str) -> Result<String>` - uses bcrypt::hash with cost 12
   - `verify_password(password: &str, hash: &str) -> Result<bool>` - uses bcrypt::verify

4. Create `backend/src/crypto/jwt.rs`:
   - `Claims` struct with `sub: String` (user_id), `exp: usize` (expiration)
   - `generate_token(user_id: Uuid) -> Result<String>` - creates JWT with 24hr expiration
   - `validate_token(token: &str) -> Result<Uuid>` - validates and returns user_id
   - Use secret from environment variable `JWT_SECRET`

5. Create `backend/tests/crypto_tests.rs`:
   - Test password hashing (same password → different hashes)
   - Test password verification (correct/incorrect passwords)
   - Test JWT generation (valid format)
   - Test JWT validation (valid token → user_id, expired token → error, invalid token → error)

6. Update `backend/src/lib.rs` or `backend/src/main.rs` to include `mod crypto;`

**Phase End:**
- Run `cargo test` → 100% pass
- Run `cargo clippy` → clean
- Update this document with completion status
- Git add + commit: "Phase 1 (password & JWT infrastructure) complete"
- WAIT for explicit approval before Phase 2

---

## Phase 2: Shared Types for Session Messages

**Subagent:** kiss-code-generator

**Code Style Checklist:**
- [ ] Functions <20 lines
- [ ] No defensive coding
- [ ] No dead code
- [ ] Tests for all new types

**Deliverables:**
1. New `AuthCredentials::Password` variant
2. New `UserMessage::ValidateSession` variants
3. Updated tests for message serialization

**Files to Update:**
- `shared/src/models/auth/auth_credentials.rs` - add Password variant
- `shared/src/models/message.rs` - add ValidateSession messages
- `shared/tests/message_tests.rs` - add serialization tests

**Tasks:**
1. Update `shared/src/models/auth/auth_credentials.rs`:
   ```rust
   #[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
   pub enum AuthCredentials {
       InviteCode(String),
       Password(String),  // NEW
   }
   ```

2. Update `shared/src/models/message.rs` UserMessage enum:
   ```rust
   pub enum UserMessage {
       // Existing variants...
       CreateUser { username: String, email: String, credentials: AuthCredentials },
       CreateUserResponse(Result<User, String>),
       SignIn { username: String, password: String },  // ADD password field
       SignInResponse(Result<User, String>),

       // NEW variants
       ValidateSession { token: String },
       ValidateSessionResponse(Result<User, String>),
   }
   ```

3. Add tests in `shared/tests/message_tests.rs` (or create if doesn't exist):
   - Test ValidateSession serialization/deserialization
   - Test ValidateSessionResponse serialization
   - Test new SignIn with password field

**Phase End:**
- Run `cargo test` → 100% pass (all three crates)
- Run `cargo clippy` → clean
- Update this document
- Git add + commit: "Phase 2 (session message types) complete"
- WAIT for explicit approval

---

## Phase 3: Backend User Storage with Passwords

**Subagent:** modular-builder

**Code Style Checklist:**
- [ ] Functions <20 lines
- [ ] Pure functions for transformations
- [ ] No dead code
- [ ] Tests for persistence

**Deliverables:**
1. `UserRecord` struct with `password_hash` field
2. Updated persistence layer to store/retrieve password hashes
3. Updated AuthService to use passwords
4. Migration strategy for existing users

**Files to Update:**
- `backend/src/persistence/mod.rs` - add UserRecord type
- `backend/src/auth_service.rs` - update InviteCodeAuthService
- `backend/src/persistence/sled_persistence.rs` - store/retrieve password hashes

**Tasks:**
1. Create `UserRecord` in `backend/src/persistence/mod.rs`:
   ```rust
   #[derive(Serialize, Deserialize)]
   struct UserRecord {
       user: User,  // The existing User type (id, username, email)
       password_hash: String,
   }
   ```

2. Update `SledPersistence::create_user()` in `backend/src/persistence/sled_persistence.rs`:
   - Accept `password_hash: String` parameter
   - Create `UserRecord { user, password_hash }`
   - Serialize and store UserRecord (not just User)

3. Update `SledPersistence::get_user_by_username()`:
   - Deserialize UserRecord
   - Return `(User, String)` tuple (user + password_hash)
   - Helper function to extract just User if needed

4. Update `InviteCodeAuthService::create_user()` in `backend/src/auth_service.rs`:
   - Accept `password: String` (plain text from frontend)
   - Hash password using `crypto::password::hash_password()`
   - Pass password_hash to persistence layer
   - Return User (NOT password_hash)

5. Update `InviteCodeAuthService::authenticate()`:
   - Accept `password: &str` parameter
   - Retrieve UserRecord from persistence
   - Verify password using `crypto::password::verify_password()`
   - Return User if valid, error if not

6. Handle existing users without passwords:
   - `get_user_by_username()` returns Result
   - If UserRecord doesn't have password_hash (old format), return error
   - User must be re-created with password (or manual migration via admin tool)

**Phase End:**
- Run `cargo test` → 100% pass
- Run `cargo clippy` → clean
- Update this document
- Git add + commit: "Phase 3 (password storage) complete"
- WAIT for explicit approval

---

## Phase 4: Backend WebSocket Handlers for Session

**Subagent:** kiss-code-generator

**Code Style Checklist:**
- [ ] Functions <20 lines
- [ ] Helper functions for validation logic
- [ ] No dead code
- [ ] Tests for handlers

**Deliverables:**
1. Updated `handle_create_user` with password hashing + JWT generation
2. Updated `handle_sign_in` with password verification + JWT generation
3. New `handle_validate_session` with JWT validation
4. JWT tokens returned in response messages

**Files to Update:**
- `backend/src/websocket/user.rs` - update all handlers
- `backend/src/websocket.rs` - route ValidateSession message

**Tasks:**
1. Update `handle_create_user()` in `backend/src/websocket/user.rs`:
   - Extract password from `AuthCredentials::Password` variant
   - Call `auth_service.create_user(username, email, password)` (updated signature)
   - On success, generate JWT: `crypto::jwt::generate_token(user.id)`
   - Return `CreateUserResponse(Ok(user))` with JWT in message (or add JWT field to response)
   - **DECISION NEEDED:** Should JWT be in a separate field or bundled?
   - **PROPOSED:** Change response to tuple: `CreateUserResponse(Result<(User, String), String>)` where String is JWT

2. Update `handle_sign_in()` in `backend/src/websocket/user.rs`:
   - Accept password parameter
   - Call `auth_service.authenticate(username, password)` (updated signature)
   - On success, generate JWT
   - Return `SignInResponse(Ok((user, jwt)))`

3. Create `handle_validate_session()` in `backend/src/websocket/user.rs`:
   ```rust
   pub async fn handle_validate_session(
       state: &AppState,
       token: String,
       tx: &mpsc::UnboundedSender<String>,
   ) -> Result<(), ()> {
       // Validate JWT
       let user_id = match crypto::jwt::validate_token(&token) {
           Ok(id) => id,
           Err(_) => {
               send_response(tx, UserMessage::ValidateSessionResponse(Err("Invalid token".into())));
               return Err(());
           }
       };

       // Get user from persistence
       let user = state.persistence.get_user_by_id(user_id).await?;
       send_response(tx, UserMessage::ValidateSessionResponse(Ok(user)));
       Ok(())
   }
   ```

4. Update `process_user_message_ws()` in `backend/src/websocket.rs`:
   ```rust
   Ok(UserMessage::ValidateSession { token }) => {
       let _ = user::handle_validate_session(state, token, tx).await;
   }
   ```

5. Update `shared/src/models/message.rs` response types:
   ```rust
   CreateUserResponse(Result<(User, String), String>),  // (User, JWT token)
   SignInResponse(Result<(User, String), String>),      // (User, JWT token)
   ```

**Phase End:**
- Run `cargo test` → 100% pass
- Run `cargo clippy` → clean
- Update this document
- Git add + commit: "Phase 4 (session WebSocket handlers) complete"
- WAIT for explicit approval

---

## Phase 5: Frontend Cookie Management

**Subagent:** kiss-code-generator

**Code Style Checklist:**
- [ ] Functions <20 lines
- [ ] Pure functions for cookie parsing
- [ ] No defensive coding
- [ ] Tests where possible (WASM tests are difficult, manual testing acceptable)

**Deliverables:**
1. Cookie utility functions
2. Cookie storage on auth success
3. Cookie reading on app load
4. Cookie clearing on logout

**Files to Update/Create:**
- `frontend/src/utils/cookies.rs` (NEW) - cookie management utilities
- `frontend/src/utils/mod.rs` - export cookies module
- `frontend/src/app/callbacks.rs` - update auth callbacks to store JWT
- `frontend/src/components/user_creation.rs` - update UI to include password field

**Tasks:**
1. Create `frontend/src/utils/cookies.rs`:
   ```rust
   use web_sys::window;

   const SESSION_COOKIE_NAME: &str = "dialect_coach_session";
   const MAX_AGE_SECONDS: i32 = 86400; // 24 hours

   pub fn set_session_token(token: &str) {
       if let Some(document) = window().and_then(|w| w.document()) {
           let cookie = format!(
               "{}={}; max-age={}; path=/; SameSite=Lax",
               SESSION_COOKIE_NAME, token, MAX_AGE_SECONDS
           );
           document.set_cookie(&cookie).ok();
       }
   }

   pub fn get_session_token() -> Option<String> {
       let cookies = window()?.document()?.cookie().ok()?;
       parse_cookie(&cookies, SESSION_COOKIE_NAME)
   }

   pub fn clear_session_token() {
       if let Some(document) = window().and_then(|w| w.document()) {
           let cookie = format!("{}=; max-age=0; path=/", SESSION_COOKIE_NAME);
           document.set_cookie(&cookie).ok();
       }
   }

   fn parse_cookie(cookies: &str, name: &str) -> Option<String> {
       cookies
           .split(';')
           .map(|s| s.trim())
           .find(|s| s.starts_with(&format!("{}=", name)))
           .and_then(|s| s.split('=').nth(1))
           .map(String::from)
   }
   ```

2. Update `frontend/src/app/callbacks.rs`:
   - `on_user_create_response()`: Store JWT in cookie when `Ok((user, jwt))` received
   - `on_user_signin_response()`: Store JWT in cookie when `Ok((user, jwt))` received
   - Add `on_validate_session_response()`: Handle ValidateSessionResponse

3. Update `frontend/src/components/user_creation.rs`:
   - Add password input field to Create Account form
   - Add password input field to Sign In form
   - Update callbacks to pass password to websocket service

**Phase End:**
- Run `cargo clippy --package dialect-coach-frontend` → clean
- Manual testing (see Phase 6)
- Update this document
- Git add + commit: "Phase 5 (frontend cookies) complete"
- WAIT for explicit approval

---

## Phase 6: Session Restoration on Page Load

**Subagent:** kiss-code-generator

**Code Style Checklist:**
- [ ] Functions <20 lines
- [ ] No defensive coding
- [ ] No dead code

**Deliverables:**
1. Check for cookie on app initialization
2. Send ValidateSession message if cookie exists
3. Restore user state from response

**Files to Update:**
- `frontend/src/app/mod.rs` or `frontend/src/main.rs` - add initialization logic
- `frontend/src/services/user_websocket.rs` - add validate_session method
- `frontend/src/app/callbacks.rs` - add validation response callback

**Tasks:**
1. Update `UserWebSocketService` in `frontend/src/services/user_websocket.rs`:
   ```rust
   pub fn validate_session(&self, token: String) {
       if let Some(sender) = self.sender.borrow().as_ref() {
           let msg = UserMessage::ValidateSession { token };
           let _ = sender.unbounded_send(serde_json::to_string(&msg).unwrap());
       }
   }
   ```

2. Add initialization logic in `frontend/src/app/mod.rs` (or wherever App component is initialized):
   - Use `use_effect_with((), |_| { ... })` hook
   - On mount, check for cookie: `if let Some(token) = cookies::get_session_token()`
   - If exists, call `user_websocket.validate_session(token)`

3. Handle `ValidateSessionResponse` in `on_validate_session_response()` callback:
   - If `Ok(user)`: `app_state.dispatch(AppStateAction::SetUser(user))`
   - If `Err(_)`: Clear cookie (invalid/expired token)

**Phase End:**
- Run full manual test flow:
  1. Create account with password → verify cookie set
  2. Reload page → verify user still logged in
  3. Wait 24+ hours (or manually expire cookie) → verify user logged out
  4. Sign in → verify cookie set
  5. Logout → verify cookie cleared
- Update this document with test results
- Git add + commit: "Phase 6 (session restoration) complete"
- WAIT for explicit approval

---

## Phase 7: Logout Functionality

**Subagent:** kiss-code-generator

**Code Style Checklist:**
- [ ] Functions <20 lines
- [ ] No dead code

**Deliverables:**
1. Logout button/callback
2. Clear cookie on logout
3. Clear AppState user

**Files to Update:**
- `frontend/src/components/` - add logout button to appropriate component
- `frontend/src/app/callbacks.rs` - add logout callback
- `frontend/src/app/app_state.rs` - verify ClearUser action exists

**Tasks:**
1. Add logout callback in `frontend/src/app/callbacks.rs`:
   ```rust
   pub fn on_logout(app_state: UseReducerHandle<AppState>) -> Callback<()> {
       Callback::from(move |_| {
           crate::utils::cookies::clear_session_token();
           app_state.dispatch(AppStateAction::ClearUser);
       })
   }
   ```

2. Add logout button to UI:
   - Determine where to place logout button (header, user menu, etc.)
   - Call `on_logout` callback when clicked

3. Verify `AppStateAction::ClearUser` in `frontend/src/app/app_state.rs`:
   - Should set `current_user: None`
   - Should clear any session-related state

**Phase End:**
- Run `cargo clippy` → clean
- Manual test: Login → Logout → verify cookie cleared and user logged out
- Update this document
- Git add + commit: "Phase 7 (logout) complete"
- WAIT for explicit approval

---

## Phase 8: Integration Testing & Documentation

**Subagent:** modular-builder

**Deliverables:**
1. End-to-end integration tests
2. Update README with new auth flow
3. Environment variable documentation (JWT_SECRET)

**Files to Update/Create:**
- `backend/tests/integration_auth_tests.rs` (NEW) - integration tests
- `README.md` - document auth flow
- `.env.example` - add JWT_SECRET

**Tasks:**
1. Create integration tests:
   - Full CreateUser flow (invite code + password → JWT returned)
   - Full SignIn flow (username + password → JWT returned)
   - ValidateSession flow (valid JWT → User returned)
   - ValidateSession with expired JWT → error
   - ValidateSession with invalid JWT → error

2. Update documentation:
   - Document password requirements (if any)
   - Document JWT expiration (24 hours)
   - Document environment variables needed

3. Create `.env.example`:
   ```
   JWT_SECRET=your-secret-key-here-min-32-characters
   ```

**Phase End:**
- Run `cargo test` → 100% pass
- Run `cargo clippy` → clean
- Update this document with completion
- Git add + commit: "Phase 8 (integration tests & docs) complete"
- DONE

---

## Testing Strategy

### Unit Tests
- Password hashing/verification (Phase 1)
- JWT generation/validation (Phase 1)
- Message serialization (Phase 2)
- Persistence layer (Phase 3)

### Integration Tests
- Full auth flow (Phase 8)
- Session validation (Phase 8)

### Manual Tests
- Cookie persistence across reload (Phase 6)
- 24-hour expiration (manual timer)
- Logout clears cookie (Phase 7)

### Test Success Criteria
- 100% unit test pass rate
- 100% integration test pass rate
- Clippy clean (zero warnings)
- Manual test checklist completed

---

## Explicitly Rejected Approaches

- ❌ **HttpOnly cookies**: User chose JS-accessible cookies for WebSocket token passing
- ❌ **Server-side sessions**: User chose JWT tokens (stateless)
- ❌ **HTTP endpoint for session restore**: User chose WebSocket message
- ❌ **Username-only auth**: User chose to add password authentication

---

## Implementation Status

- [x] Phase 1: Backend Password & JWT Infrastructure ✅ (7/7 tests passing, clippy clean)
- [x] Phase 2: Shared Types for Session Messages ✅ (195/195 tests passing, clippy clean)
- [x] Phase 3: Backend User Storage with Passwords ✅ (31/31 tests passing, clippy clean)
- [x] Phase 4: Backend WebSocket Handlers for Session ✅ (110/110 tests passing, builds successfully)
- [ ] Phase 5: Frontend Cookie Management
- [ ] Phase 6: Session Restoration on Page Load
- [ ] Phase 7: Logout Functionality
- [ ] Phase 8: Integration Testing & Documentation

**Current Phase:** Phase 4 Complete
**Blockers:** None
**Next Steps:** Commit Phase 4, proceed to Phase 5 (Frontend Cookie Management)

**Phase 4 Summary:**
- Updated response types to return JWT tokens: `CreateUserResponse(Result<(User, String), String>)`
- Updated `handle_create_user` to generate JWT and return in response
- Updated `handle_sign_in` to accept password, verify, generate JWT
- Created `handle_validate_session` to validate JWT and return User
- Updated WebSocket routing to handle ValidateSession messages
- Added `mod crypto` to main.rs for binary access
- All 110 backend tests passing, 195 shared tests passing
