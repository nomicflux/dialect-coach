# Authentication Implementation Status

## User Decisions - Final Agreements

**Auth Method**: Invite code system (database-backed)
- User explicitly specified: "Invite code system with database backing"
- Date: 2025-11-07

**Invite Code Storage**: Full metadata tracking
- Fields: invite_code (String), created_date (i64), used_by (Option<Uuid>), expiration (Option<i64>)
- User explicitly specified: "Database-backed with created_date, used_by, expiration"
- Date: 2025-11-07

**Admin Interface**: CLI/API to generate codes
- User explicitly specified: "CLI/API to generate new codes"
- Date: 2025-11-07

**Future Compatibility**: OAuth2-compatible traits
- User explicitly specified: "Design traits for OAuth2 compatibility (don't implement OAuth2 yet)"
- Date: 2025-11-07

**User Creation**: Username + Email + Invite Code
- User explicitly specified: "Username (unique), Email (new field in UserState), Invite code verification"
- User explicitly specified: "No backwards compatibility - blow away existing data and start fresh"
- Date: 2025-11-07

## Explicitly Rejected

- OAuth2 implementation (deferred for future - only trait design now)
- Password authentication (not mentioned, not chosen)
- Backwards compatibility / migration (user said: "blow it away and start fresh")
- Collecting unnecessary user data (user said: "minimal data collection")

## Architecture Analysis

### Current State
- **User model**: Has `id: Uuid` and `username: String` only (shared/src/models/user.rs)
- **UserState model**: Large struct with learning data, no email field (shared/src/models/user_state.rs)
- **Persistence**: UserPersistence trait with Sled/InMemory implementations (backend/src/persistence/)
- **Frontend**: Unknown (needs research)
- **Backend**: Axum WebSocket, no auth service yet

### Required Changes
1. **Shared types** (affects all 3 crates):
   - Add `email: String` to User or UserState
   - Add InviteCode model
   - Add auth credential types (design for OAuth2 compatibility)

2. **Backend**:
   - AuthService trait (OAuth2-compatible design)
   - InviteCodeAuthService implementation
   - Extend UserPersistence trait for invite codes
   - Extend Sled implementation
   - Extend InMemory implementation

3. **Frontend**:
   - User creation UI (button → full form)
   - Sign-in UI (username + invite code)

4. **Admin tools**:
   - CLI command to generate invite codes

### Trait Design for OAuth2 Compatibility

**AuthService trait** should support:
- Invite code flow (implement now)
- OAuth2 flow (implement later)

Methods needed:
- `create_user` - User creation with credentials
- `authenticate` - Verify credentials
- `authorize` - Check permissions

**Credentials enum** (OAuth2-ready):
```rust
enum AuthCredentials {
    InviteCode(String),
    OAuth2Token(String),  // Future
}
```

## Implementation Phases

### Phase 1: Shared Types - InviteCode Model

**Subagent**: kiss-code-generator (single file, simple structs)

**Code Style Checklist**:
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] Helper functions for complex logic
- [ ] Tests for all functions

**Status**: COMPLETED 2025-11-07

**Files created**:
- `/Users/demouser/Code/dialect-coach/shared/src/models/auth/invite_code.rs`
- `/Users/demouser/Code/dialect-coach/shared/src/models/auth/mod.rs`

**Files modified**:
- `/Users/demouser/Code/dialect-coach/shared/src/models/mod.rs` - Added `pub mod auth;` and `pub use auth::*;`

**Implementation Summary**:

Created InviteCode model with all specified methods:

```rust
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct InviteCode {
    pub code: String,
    pub created_date: i64,
    pub used_by: Option<Uuid>,
    pub expiration: Option<i64>,
}

impl InviteCode {
    pub fn new(code: String, expiration: Option<i64>) -> Self  // 9 lines
    pub fn is_expired(&self, current_time: i64) -> bool        // 5 lines
    pub fn is_used(&self) -> bool                              // 2 lines
    pub fn mark_used(&mut self, user_id: Uuid)                 // 2 lines
    pub fn is_valid(&self, current_time: i64) -> bool          // 2 lines
}
```

**Tests Implemented** (16 total):
- test_invite_code_creation
- test_invite_code_with_expiration
- test_is_expired_with_future_expiration
- test_is_expired_with_past_expiration
- test_is_expired_no_expiration
- test_is_used_when_not_used
- test_is_used_when_used
- test_mark_used
- test_is_valid_when_not_expired_and_not_used
- test_is_valid_when_expired
- test_is_valid_when_used
- test_is_valid_when_both_expired_and_used
- test_serialization
- test_deserialization
- test_clone

**Test Results**:
✓ All 16 InviteCode tests PASSED
✓ All 116 shared crate tests PASSED
✓ 100% success rate

**KISS Compliance**:
✓ All functions under 20 lines
✓ Pure functions for checks (is_expired, is_used, is_valid)
✓ Single mutation function (mark_used)
✓ No defensive coding
✓ No future-proofing abstractions
✓ Comprehensive test coverage

**Next step**: Proceed to Phase 2 (AuthCredentials & Email field)

---

### Phase 2: Shared Types - Auth Credentials & Email

**Status**: COMPLETED 2025-11-07

**Subagent**: kiss-code-generator (modifying existing User model)

**Code Style Checklist**:
- [x] Functions <20 lines
- [x] Pure functions where possible
- [x] No defensive coding
- [x] Helper functions for complex logic
- [x] Tests for all functions

**Files created**:
- `/Users/demouser/Code/dialect-coach/shared/src/models/auth/auth_credentials.rs`

**Files modified**:
- `/Users/demouser/Code/dialect-coach/shared/src/models/user.rs` (added email field)
- `/Users/demouser/Code/dialect-coach/shared/src/models/auth/mod.rs` (export auth_credentials)
- `/Users/demouser/Code/dialect-coach/shared/src/models/message.rs` (updated User::new calls)
- `/Users/demouser/Code/dialect-coach/backend/src/websocket.rs` (updated User::new calls)
- `/Users/demouser/Code/dialect-coach/backend/src/persistence/in_memory.rs` (updated User::new calls)

**Implementation Summary**:

**AuthCredentials enum** (6 lines):
```rust
pub enum AuthCredentials {
    InviteCode(String),
}

impl AuthCredentials {
    pub fn invite_code(code: String) -> Self { ... }     // 2 lines
    pub fn as_invite_code(&self) -> Option<&str> { ... } // 5 lines
}
```

**User struct**:
```rust
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,  // NEW FIELD
}

impl User {
    pub fn new(id: Uuid, username: String, email: String) -> Self { ... } // 3 lines
}
```

**Updated User::new() calls**:
- shared/src/models/user.rs: 5 test calls updated
- shared/src/models/message.rs: 2 test calls updated
- backend/src/websocket.rs: 3 test calls + 1 production call updated
- backend/src/persistence/in_memory.rs: 5 test calls updated

**Total: 16 User::new() calls updated across the codebase**

**Tests Implemented** (6 AuthCredentials tests):
- test_invite_code_construction
- test_as_invite_code_some
- test_as_invite_code_extracts_string
- test_serialization
- test_deserialization
- test_clone

**Tests Updated** (all passing):
- All User tests updated to include email parameter
- All message tests using User::new updated
- All persistence tests using User::new updated

**Test Results**:
✓ AuthCredentials: 6 tests PASSED
✓ Shared crate: 122 total tests PASSED (includes updated User tests)
✓ Backend crate: 130 total tests PASSED (41 lib + 89 main)
✓ Frontend crate: 17 tests PASSED
✓ Corpus processor: 48 total tests PASSED
✓ Integration tests: 80 total tests PASSED
✓ TOTAL: 369+ tests, 0 failures, 100% success rate

**KISS Compliance**:
✓ AuthCredentials::invite_code() - 2 lines
✓ AuthCredentials::as_invite_code() - 5 lines
✓ User::new() - 3 lines
✓ All functions under 20 lines
✓ Pure functions for value construction
✓ No defensive coding
✓ No future-proofing abstractions
✓ Comprehensive test coverage
✓ No dead code left behind
✓ All User constructor calls updated in single phase

**Next step**: Proceed to Phase 3 (Extend UserPersistence Trait)

---

### Phase 3: Backend - Extend UserPersistence Trait

**Subagent**: kiss-code-generator (extending trait in single file)

**Code Style Checklist**:
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] Helper functions for complex logic
- [ ] Tests for all functions

**Files to modify**:
- `/Users/demouser/Code/dialect-coach/backend/src/persistence/mod.rs`

**Specifications**:

Add to UserPersistence trait:
```rust
async fn create_invite_code(&self, invite_code: &InviteCode) -> Result<()>;
async fn load_invite_code(&self, code: &str) -> Result<Option<InviteCode>>;
async fn save_invite_code(&self, invite_code: &InviteCode) -> Result<()>;
async fn list_invite_codes(&self) -> Result<Vec<InviteCode>>;
```

**Error cases**:
- create_invite_code: Error if code already exists
- load_invite_code: Ok(None) if not found
- save_invite_code: Overwrites existing (for mark_used updates)

**Success criteria**:
- Trait compiles
- InMemory and Sled implementations show compile errors (expected - next phase fixes)

**Next step**: Update this doc with status, proceed to Phase 4

---

### Phase 4: Backend - Implement Invite Code Persistence (Sled)

**Subagent**: kiss-code-generator (single file implementation)

**Code Style Checklist**:
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] Helper functions for complex logic
- [ ] Tests for all functions

**Files to modify**:
- `/Users/demouser/Code/dialect-coach/backend/src/persistence/sled.rs`

**Specifications**:

```rust
impl SledPersistence {
    fn invite_codes_tree(&self) -> Result<sled::Tree>  // New helper
}

// Implement 4 new trait methods
impl UserPersistence for SledPersistence {
    async fn create_invite_code(&self, invite_code: &InviteCode) -> Result<()> {
        // Check not exists, serialize, insert into invite_codes tree
        // Key: invite_code.code
        // Value: JSON of InviteCode
    }

    async fn load_invite_code(&self, code: &str) -> Result<Option<InviteCode>> {
        // Load from invite_codes tree, deserialize
    }

    async fn save_invite_code(&self, invite_code: &InviteCode) -> Result<()> {
        // Overwrite (for mark_used updates)
    }

    async fn list_invite_codes(&self) -> Result<Vec<InviteCode>> {
        // Iterate invite_codes tree, collect all
    }
}
```

**Helper functions**:
- Use existing `serialize_to_json` / `deserialize_from_json` helpers
- Keep functions <20 lines

**Tests required**:
- Create new invite code
- Create duplicate code (error)
- Load existing code
- Load non-existent code (None)
- Save code (mark_used scenario)
- List codes (empty, one, multiple)

**Success criteria**:
- All tests pass
- Functions <20 lines
- Clear error messages

**Next step**: Run full test suite, require 100% success, update this doc

---

### Phase 5: Backend - Implement Invite Code Persistence (InMemory)

**Subagent**: kiss-code-generator (single file implementation)

**Code Style Checklist**:
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] Helper functions for complex logic
- [ ] Tests for all functions

**Files to modify**:
- `/Users/demouser/Code/dialect-coach/backend/src/persistence/in_memory.rs`

**Specifications**:

```rust
pub struct InMemoryPersistence {
    // Add new field:
    invite_codes: Arc<RwLock<HashMap<String, InviteCode>>>,
}

// Implement 4 trait methods (similar to Sled but with RwLock HashMap)
```

**Tests required**:
- Same as Phase 4 but for InMemory implementation
- Thread safety (Arc/RwLock usage)

**Success criteria**:
- All tests pass
- Consistent with Sled implementation behavior

**Next step**: Run full test suite, require 100% success, update this doc

---

### Phase 6: Backend - AuthService Trait (OAuth2-Compatible)

**Subagent**: modular-builder (new file, cross-file integration)

**Code Style Checklist**:
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] Helper functions for complex logic
- [ ] Tests for all functions

**Files to create**:
- `/Users/demouser/Code/dialect-coach/backend/src/auth_service.rs`

**Files to modify**:
- `/Users/demouser/Code/dialect-coach/backend/src/lib.rs` (if exists, add mod auth_service)

**Specifications**:

```rust
// backend/src/auth_service.rs

use dialect_coach_shared::{User, AuthCredentials, InviteCode};

#[async_trait::async_trait]
pub trait AuthService: Send + Sync {
    /// Create a new user with authentication credentials
    /// Returns the created User or error
    async fn create_user(
        &self,
        username: String,
        email: String,
        credentials: AuthCredentials
    ) -> Result<User>;

    /// Authenticate existing user
    /// Returns User if successful, error if auth fails
    async fn authenticate(
        &self,
        username: &str,
        credentials: AuthCredentials
    ) -> Result<User>;

    /// Check if user is authorized (for rate limiting integration)
    /// Returns true if user can make requests
    async fn is_authorized(&self, user_id: Uuid) -> Result<bool>;
}

pub struct AuthError {
    // Clear error types
}

// Error types for auth failures:
// - InvalidInviteCode
// - ExpiredInviteCode
// - UsedInviteCode
// - UsernameTaken
// - UserNotFound
// - AuthenticationFailed
```

**Design notes**:
- OAuth2-compatible: `authenticate` accepts enum, can add OAuth2Token variant later
- Rate limiting integration: `is_authorized` method for future rate limiter
- Clear error types for frontend messaging

**Tests**: No implementation yet, just trait definition

**Success criteria**:
- Compiles
- Clear method signatures
- OAuth2-extensible design

**Next step**: Update this doc, proceed to Phase 7

---

### Phase 7: Backend - InviteCodeAuthService Implementation

**Subagent**: modular-builder (implements trait, integrates persistence)

**Code Style Checklist**:
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] Helper functions for complex logic
- [ ] Tests for all functions

**Files to modify**:
- `/Users/demouser/Code/dialect-coach/backend/src/auth_service.rs`

**Specifications**:

```rust
pub struct InviteCodeAuthService {
    persistence: Arc<dyn UserPersistence>,
}

impl InviteCodeAuthService {
    pub fn new(persistence: Arc<dyn UserPersistence>) -> Self

    // Helper functions (keep <20 lines each):
    async fn validate_invite_code(&self, code: &str) -> Result<InviteCode>
    async fn check_username_available(&self, username: &str) -> Result<()>
}

#[async_trait::async_trait]
impl AuthService for InviteCodeAuthService {
    async fn create_user(
        &self,
        username: String,
        email: String,
        credentials: AuthCredentials
    ) -> Result<User> {
        // 1. Extract invite code from credentials (or error)
        // 2. validate_invite_code (exists, not expired, not used)
        // 3. check_username_available
        // 4. Create User with new UUID
        // 5. create_user in persistence
        // 6. mark_used on invite code
        // 7. save_invite_code to persist mark_used
        // 8. Return User
    }

    async fn authenticate(
        &self,
        username: &str,
        credentials: AuthCredentials
    ) -> Result<User> {
        // For invite code auth: just verify user exists
        // (Invite code was verified at creation time)
        // Load user by username, return if exists
        // Note: This is minimal - OAuth2 would do actual token validation here
    }

    async fn is_authorized(&self, user_id: Uuid) -> Result<bool> {
        // Simple check: user exists
        // Future: integrate with rate limiter
    }
}
```

**Helper functions** (pure where possible):
- `validate_invite_code`: loads code, checks expiration, checks used
- `check_username_available`: checks if username exists

**Tests required**:
- create_user: success path
- create_user: invalid invite code
- create_user: expired invite code
- create_user: used invite code
- create_user: username taken
- authenticate: user exists
- authenticate: user not found
- is_authorized: basic check

**Success criteria**:
- All tests pass
- Functions <20 lines
- Clear error messages
- Invite code properly marked as used after user creation

**Next step**: Run full test suite, require 100% success, update this doc

---

### Phase 8: Backend - Admin CLI for Invite Code Generation

**Subagent**: modular-builder (new binary or subcommand)

**Code Style Checklist**:
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] Helper functions for complex logic
- [ ] Tests for all functions

**Files to create**:
- `/Users/demouser/Code/dialect-coach/backend/src/bin/admin.rs` (new binary)
  OR
- Add subcommand to existing main.rs

**Files to modify**:
- `/Users/demouser/Code/dialect-coach/backend/Cargo.toml` (add bin if new binary)

**Specifications**:

```rust
// admin.rs or subcommand

async fn generate_invite_code(
    persistence: Arc<dyn UserPersistence>,
    expiration_days: Option<u32>
) -> Result<String> {
    // 1. Generate random code (UUID or secure random string)
    // 2. Calculate expiration timestamp if provided
    // 3. Create InviteCode
    // 4. create_invite_code in persistence
    // 5. Return code string
}

async fn list_invite_codes(
    persistence: Arc<dyn UserPersistence>
) -> Result<()> {
    // 1. list_invite_codes from persistence
    // 2. Print formatted table (code, created, used_by, expiration)
}

// CLI interface:
// ./admin generate-invite [--expires-days N]
// ./admin list-invites
```

**Helper functions**:
- `generate_random_code()`: creates secure random string
- `calculate_expiration(days: u32) -> i64`: adds days to current timestamp
- `format_invite_table(codes: Vec<InviteCode>) -> String`: formats for display

**Tests required**:
- generate_invite_code: creates valid code
- generate_invite_code: with expiration
- generate_invite_code: without expiration
- list_invite_codes: empty list
- list_invite_codes: multiple codes
- Helper function tests

**Success criteria**:
- CLI runs successfully
- Can generate codes
- Can list codes
- All tests pass

**Next step**: Run full test suite, require 100% success, update this doc

---

### Phase 9: Frontend - User Creation UI (Research First)

**Subagent**: modular-builder (need to research frontend structure first)

**Research required**:
1. Find frontend user creation code (Yew components)
2. Find current username-only creation flow
3. Understand WebSocket message protocol
4. Find shared message types for user creation

**Files to research** (examples, actual paths TBD):
- frontend/src/components/user_creation.rs (or similar)
- frontend/src/services/websocket.rs (or similar)
- shared/src/models/events.rs (WebSocket events)

**Specification** (after research):
- Replace username-only input with button "Create Account"
- Button opens form with fields:
  - Username (text input, required)
  - Email (email input, required)
  - Invite Code (text input, required)
- Form validation (non-empty, valid email format)
- Submit → WebSocket message to backend
- Handle response (success → login, error → display message)

**Phase 9 tasks**: To be defined after research

**Next step**: Research frontend codebase, create detailed spec, update this doc

---

### Phase 10: Frontend - Sign-In UI

**Subagent**: modular-builder

**Prerequisites**: Phase 9 complete (understand frontend structure)

**Specification** (after research):
- Sign-in form with fields:
  - Username (text input, required)
  - Invite Code (text input, required) - Note: might be removed later for returning users
- Submit → WebSocket authenticate message
- Handle response (success → login, error → display)

**Phase 10 tasks**: To be defined after Phase 9 research

**Next step**: After Phase 9, define detailed tasks, update this doc

---

### Phase 11: Backend - WebSocket Integration

**Subagent**: modular-builder (integrate AuthService into WebSocket handlers)

**Prerequisites**: Phases 1-8 complete (backend auth service ready)

**Research required**:
1. Find WebSocket handler code
2. Understand current user creation/login flow
3. Find where UserPersistence is currently used

**Files to research**:
- backend/src/websocket.rs (or similar)
- backend/src/main.rs (startup, dependency injection)

**Specification** (after research):
- Add AuthService to WebSocket handler state
- Create handler for user creation message (calls AuthService::create_user)
- Create handler for authenticate message (calls AuthService::authenticate)
- Return clear error messages to frontend

**Phase 11 tasks**: To be defined after research

**Next step**: After Phases 1-8, research WebSocket code, create detailed spec, update this doc

---

## Current Status

**Phase**: Phase 2 COMPLETED, Phase 3 ready to start
**Last Updated**: 2025-11-07
**Blocker**: None

## Completed Phases

- **Phase 1** (2025-11-07): InviteCode model created in shared/src/models/auth/
  - All 16 tests passing
  - All functions under 20 lines
  - Pure functions for checks, single mutation function

- **Phase 2** (2025-11-07): AuthCredentials enum and email field to User
  - AuthCredentials enum with OAuth2-compatible design (6 tests passing)
  - User struct updated with email field
  - 16 User::new() calls updated across codebase
  - 369+ tests passing, 100% success rate

## Next Actions

1. Phase 3: Extend UserPersistence trait with invite code methods
2. Phase 4: Implement Sled persistence for invite codes
3. Phase 5: Implement InMemory persistence for invite codes

## Questions / Clarifications Needed

None currently - all decisions documented above.
