# User Admin Flags and Initial Settings Implementation Plan

## Overview

Add two features to the Dialect Coach application:
1. **Admin flag on invite codes and users** - Invite codes can be marked as "admin", users registering with admin codes become admins
2. **Initial user settings during registration** - Collect language, dialect, CEFR level, and gender at signup to initialize UserState

## User Decisions (Captured 2025-12-19)
- **Settings UX**: Collect initial settings on registration form (not separate setup step)
- **Data model**: Initialize UserState with settings (not stored in User schema)

## Explicitly Rejected
- Storing language/dialect/level/gender preferences in User struct (only is_admin goes in User)
- Separate setup wizard after account creation
- Changing admin status after user creation

---

## Phase 1: Admin Flag on InviteCode

### Code Style Checklist
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No dead code
- [ ] Tests for new functions

### Subagent: kiss-code-generator

### Files to Modify

#### 1. `shared/src/models/auth/invite_code.rs`

**Current state:**
```rust
pub struct InviteCode {
    pub code: String,
    pub created_date: i64,
    pub used_by: Option<Uuid>,
    pub expiration: Option<i64>,
}
```

**Changes:**
1. Add `pub is_admin: bool` field to struct
2. Add new constructor:
```rust
pub fn new_with_admin(code: String, expiration: Option<i64>, is_admin: bool) -> Self {
    let created_date = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    Self { code, created_date, used_by: None, expiration, is_admin }
}
```
3. Modify existing `new()` to call `Self::new_with_admin(code, expiration, false)`
4. Update all test assertions and constructors to include `is_admin: false`

#### 2. `shared/src/models/admin.rs`

**Changes:**
1. Add to `CreateInviteRequest`:
```rust
pub is_admin: Option<bool>,  // defaults to false when None
```
2. Add to `InviteResponse`:
```rust
pub is_admin: bool,
```
3. Add to `InviteListItem`:
```rust
pub is_admin: bool,
```

### Deliverables
- [ ] InviteCode struct has `is_admin: bool` field
- [ ] `new_with_admin()` constructor exists
- [ ] Admin API types include is_admin
- [ ] All tests in invite_code.rs pass
- [ ] All tests in admin.rs pass (if any)

### Phase End Reminder
1. Run `cargo test` in shared crate - 100% pass required
2. Run `cargo clippy` - fix ALL errors, remove dead code
3. Update this document with status
4. `git add . && git commit -m "Phase 1 (InviteCode admin flag) complete"`
5. STOP and wait for approval before Phase 2

---

## Phase 2: Admin Flag on User with Version Migration

### Code Style Checklist
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No dead code
- [ ] Tests for new functions

### Subagent: modular-builder

### Files to Modify

#### 1. `shared/src/models/user.rs`

**Current state:**
```rust
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
}
```

**Changes:**
1. Add `pub is_admin: bool` field
2. Add constructor:
```rust
pub fn new_with_admin(id: Uuid, username: String, email: String, is_admin: bool) -> Self {
    Self { id, username, email, is_admin }
}
```
3. Modify `new()` to call `Self::new_with_admin(id, username, email, false)`
4. Update all tests to include is_admin

#### 2. `shared/src/models/versioning.rs`

**Current state:**
```rust
pub enum UserVersion {
    #[default]
    V1,
}
pub const CURRENT_USER_VERSION: UserVersion = UserVersion::V1;
```

**Changes:**
1. Add V2_Admin variant and make it default:
```rust
pub enum UserVersion {
    V1,
    #[default]
    V2_Admin,
}
```

2. Add V1 data struct for migration:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserV1Data {
    pub id: Uuid,
    pub username: String,
    pub email: String,
}
```

3. Update `CURRENT_USER_VERSION`:
```rust
pub const CURRENT_USER_VERSION: UserVersion = UserVersion::V2_Admin;
```

4. Update `migrate_user_to_current()`:
```rust
pub fn migrate_user_to_current(from_version: UserVersion, data: serde_json::Value) -> User {
    match from_version {
        UserVersion::V1 => {
            let v1: UserV1Data = serde_json::from_value(data).expect("Valid V1 user data");
            User::new_with_admin(v1.id, v1.username, v1.email, false)
        }
        UserVersion::V2_Admin => {
            serde_json::from_value(data).expect("Valid V2 user data")
        }
    }
}
```

5. Update `test_user_structure_snapshot` expected fields:
```rust
let expected_fields = vec!["email", "id", "is_admin", "username"];
```

6. Add migration test:
```rust
#[test]
fn test_migrate_user_v1_to_v2_admin() {
    let v1_data = serde_json::json!({
        "id": "550e8400-e29b-41d4-a716-446655440000",
        "username": "testuser",
        "email": "test@example.com"
    });
    let user = migrate_user_to_current(UserVersion::V1, v1_data);
    assert_eq!(user.username, "testuser");
    assert!(!user.is_admin);
}
```

### Deliverables
- [ ] User struct has `is_admin: bool` field
- [ ] UserVersion::V2_Admin exists and is default
- [ ] Migration from V1 sets is_admin = false
- [ ] Snapshot test updated with new fields
- [ ] Migration test passes
- [ ] All shared crate tests pass

### Phase End Reminder
1. Run `cargo test` - 100% pass required
2. Run `cargo clippy` - fix ALL errors, remove dead code
3. Update this document with status
4. `git add . && git commit -m "Phase 2 (User V2_Admin migration) complete"`
5. STOP and wait for approval before Phase 3

---

## Phase 3: Backend Admin Flag Handling

### Code Style Checklist
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No dead code
- [ ] Tests for new functions

### Subagent: modular-builder

### Files to Modify

#### 1. `backend/src/admin_invites.rs`

**Changes in `create_invite()`:**
```rust
let is_admin = req.is_admin.unwrap_or(false);
let invite = InviteCode::new_with_admin(code.clone(), expires_at, is_admin);
```

**Changes in response construction:**
```rust
InviteResponse {
    code: invite.code,
    created_at: invite.created_date,
    expires_at: invite.expiration,
    is_admin: invite.is_admin,
}
```

**Changes in `list_invites()`:**
Include `is_admin: invite.is_admin` in InviteListItem construction.

#### 2. `backend/src/auth_service.rs`

**Changes in `create_and_persist_user()` (around line 187):**
- Add `is_admin: bool` parameter
- Use `User::new_with_admin(Uuid::new_v4(), username, email, is_admin)`

**Changes in `create_user()` (around line 88-109):**
- After validating invite code, extract `invite.is_admin`
- Pass `invite.is_admin` to `create_and_persist_user()`

#### 3. `backend/src/persistence/postgres.rs`

**Changes in `parse_user_from_row()`:**
```rust
let is_admin: bool = r.get("is_admin");
User { id, username, email: email.unwrap_or_default(), is_admin }
```

**Changes in `parse_invite_code_from_row()`:**
```rust
let is_admin: bool = r.get("is_admin");
InviteCode { code, created_date, used_by, expiration, is_admin }
```

**Changes in `create_user()` INSERT:**
```sql
INSERT INTO users (id, username, email, password_hash, is_admin) VALUES ($1, $2, $3, $4, $5)
```

**Changes in `create_invite_code()` INSERT:**
```sql
INSERT INTO invite_codes (code, created_date, used_by, expiration, is_admin) VALUES ($1, $2, $3, $4, $5)
```

**Changes in `save_invite_code()` UPSERT:**
Include is_admin in both INSERT and UPDATE clauses.

#### 4. `backend/src/persistence/sled.rs`

**Changes in `deserialize_versioned_user_record()`:**
Use `migrate_user_to_current(wrapper.version, wrapper.data)` to handle V1→V2 migration.

The UserRecord struct wraps User, so after deserializing the versioned data, extract password_hash separately from the raw JSON before migration.

#### 5. `backend/sql/init.sql`

**Add columns to CREATE TABLE statements:**

For users table:
```sql
is_admin BOOLEAN NOT NULL DEFAULT FALSE,
```

For invite_codes table:
```sql
is_admin BOOLEAN NOT NULL DEFAULT FALSE
```

### Files to Create

#### 6. `backend/sql/migrations/001_add_admin_flags.sql`

```sql
-- Migration: Add is_admin columns to existing installations
ALTER TABLE users ADD COLUMN IF NOT EXISTS is_admin BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE invite_codes ADD COLUMN IF NOT EXISTS is_admin BOOLEAN NOT NULL DEFAULT FALSE;
```

### Deliverables
- [ ] Admin invites create admin codes
- [ ] Auth service passes is_admin from invite to user
- [ ] PostgreSQL persistence reads/writes is_admin for both tables
- [ ] Sled persistence uses version migration for User
- [ ] init.sql has is_admin columns
- [ ] Migration script exists for existing databases
- [ ] All backend tests pass

### Phase End Reminder
1. Run `cargo test` - 100% pass required
2. Run `cargo clippy` - fix ALL errors, remove dead code
3. Update this document with status
4. `git add . && git commit -m "Phase 3 (Backend admin flag handling) complete"`
5. STOP and wait for approval before Phase 4

---

## Phase 4: Initial Settings Types in Shared Crate

### Code Style Checklist
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No dead code
- [ ] Tests for new functions

### Subagent: kiss-code-generator

### Files to Modify

#### 1. `shared/src/models/message.rs`

**Add new struct after imports:**
```rust
use super::{Dialect, Language, LanguageLevel, UserGender};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InitialUserSettings {
    pub language: Language,
    pub dialect: Dialect,
    pub level: LanguageLevel,
    pub gender: UserGender,
}

impl Default for InitialUserSettings {
    fn default() -> Self {
        Self {
            language: Language::Spanish,
            dialect: Dialect::SpanishArgentinian,
            level: LanguageLevel::B1,
            gender: UserGender::NonBinary,
        }
    }
}
```

**Modify UserMessage::CreateUser:**
```rust
CreateUser {
    username: String,
    email: String,
    credentials: AuthCredentials,
    password: String,
    initial_settings: Option<InitialUserSettings>,  // NEW
},
```

**Add test:**
```rust
#[test]
fn test_initial_user_settings_default() {
    let settings = InitialUserSettings::default();
    assert_eq!(settings.language, Language::Spanish);
    assert_eq!(settings.dialect, Dialect::SpanishArgentinian);
    assert_eq!(settings.level, LanguageLevel::B1);
    assert_eq!(settings.gender, UserGender::NonBinary);
}
```

#### 2. `shared/src/models/user_state.rs`

**Add new constructor:**
```rust
pub fn with_initial_settings(
    user_id: Uuid,
    initial_settings: Option<super::message::InitialUserSettings>,
) -> Self {
    let initial_branch = ConversationBranch::new(None, None, None, None, vec![]);
    let initial_branch_id = initial_branch.id;
    let show_experimental_dialects = false;

    let (language, dialect, gender, dialect_levels) = match initial_settings {
        Some(s) => {
            let dl = DialectLevel::new(s.dialect, s.level);
            (s.language, s.dialect, s.gender, vec![dl])
        }
        None => {
            let lang = Language::Spanish;
            let dial = Self::default_dialect_for_language(lang, show_experimental_dialects);
            (lang, dial, UserGender::NonBinary, Vec::new())
        }
    };

    Self {
        user_id,
        learning_items: Vec::new(),
        conversation_history: Vec::new(),
        tts_enabled: false,
        selected_language: language,
        selected_dialect: dialect,
        formality: Formality::Informal,
        teaching_mode: TeachingMode::Immersive,
        user_gender: gender,
        active_branch_id: initial_branch_id,
        branches: vec![initial_branch],
        learning_goals: Vec::new(),
        language_plans: Vec::new(),
        active_plan_id: None,
        usage_stats: UsageStats::default(),
        language_options: LanguageOptions::default(),
        show_experimental_dialects,
        dialect_levels,
    }
}
```

**Modify existing new():**
```rust
pub fn new(user_id: Uuid) -> Self {
    Self::with_initial_settings(user_id, None)
}
```

**Add test:**
```rust
#[test]
fn test_user_state_with_initial_settings() {
    use super::message::InitialUserSettings;
    let settings = InitialUserSettings {
        language: Language::Japanese,
        dialect: Dialect::JapaneseStandard,
        level: LanguageLevel::A2,
        gender: UserGender::Female,
    };
    let state = UserState::with_initial_settings(Uuid::new_v4(), Some(settings));
    assert_eq!(state.selected_language, Language::Japanese);
    assert_eq!(state.selected_dialect, Dialect::JapaneseStandard);
    assert_eq!(state.user_gender, UserGender::Female);
    assert_eq!(state.dialect_levels.len(), 1);
    assert_eq!(state.dialect_levels[0].level, LanguageLevel::A2);
}
```

#### 3. `shared/src/models/mod.rs`

**Add export:**
```rust
pub use message::InitialUserSettings;
```

### Deliverables
- [ ] InitialUserSettings struct exists with Default impl
- [ ] UserMessage::CreateUser has optional initial_settings field
- [ ] UserState::with_initial_settings constructor works
- [ ] UserState::new() delegates to with_initial_settings
- [ ] InitialUserSettings exported from shared crate
- [ ] All tests pass

### Phase End Reminder
1. Run `cargo test` - 100% pass required
2. Run `cargo clippy` - fix ALL errors, remove dead code
3. Update this document with status
4. `git add . && git commit -m "Phase 4 (InitialUserSettings types) complete"`
5. STOP and wait for approval before Phase 5

---

## Phase 5: Backend Initial Settings Flow

### Code Style Checklist
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No dead code
- [ ] Tests for new functions

### Subagent: kiss-code-generator

### Files to Modify

#### 1. `backend/src/websocket/user.rs`

**Update handle_create_user signature:**
```rust
pub async fn handle_create_user(
    state: &AppState,
    username: String,
    email: String,
    credentials: dialect_coach_shared::AuthCredentials,
    password: String,
    initial_settings: Option<dialect_coach_shared::InitialUserSettings>,  // NEW
    tx: &mpsc::UnboundedSender<String>,
) -> Result<(), ()>
```

The initial_settings is not used in the backend - it passes through in the response for the frontend to use when creating UserState.

**Update websocket message handler** to extract initial_settings from UserMessage::CreateUser and pass to handle_create_user.

### Deliverables
- [ ] handle_create_user accepts initial_settings parameter
- [ ] Message handler extracts and passes initial_settings
- [ ] All backend tests pass

### Phase End Reminder
1. Run `cargo test` - 100% pass required
2. Run `cargo clippy` - fix ALL errors, remove dead code
3. Update this document with status
4. `git add . && git commit -m "Phase 5 (Backend initial settings flow) complete"`
5. STOP and wait for approval before Phase 6

---

## Phase 6: Frontend Initial Settings UI

### Code Style Checklist
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No dead code
- [ ] Tests for new functions

### Subagent: modular-builder

### Files to Modify

#### 1. `frontend/src/components/user_creation.rs`

**Add state hooks for settings:**
```rust
let selected_language = use_state(|| Language::Spanish);
let selected_dialect = use_state(|| Dialect::SpanishArgentinian);
let selected_level = use_state(|| LanguageLevel::B1);
let selected_gender = use_state(|| UserGender::NonBinary);
```

**Update on_submit to build InitialUserSettings:**
```rust
let settings = InitialUserSettings {
    language: (*selected_language).clone(),
    dialect: (*selected_dialect).clone(),
    level: (*selected_level).clone(),
    gender: (*selected_gender).clone(),
};
create_user.emit((
    (*username).clone(),
    (*email).clone(),
    (*password).clone(),
    (*invite_code).clone(),
    Some(settings),
));
```

**Add select inputs to HTML** for language, dialect, level, and gender. Use existing UI patterns from settings components if available.

#### 2. `frontend/src/app/callbacks.rs`

**Update on_create_user_click** to accept and pass initial_settings:
```rust
pub fn on_create_user_click(
    app_state: UseReducerHandle<AppState>,
    _session: UseReducerHandle<SessionState>,
) -> Callback<(String, String, String, String, Option<InitialUserSettings>)>
```

Pass initial_settings to the websocket create_user call.

#### 3. `frontend/src/services/user_websocket.rs`

**Update create_user method:**
```rust
pub fn create_user(
    &self,
    username: String,
    email: String,
    credentials: AuthCredentials,
    password: String,
    initial_settings: Option<InitialUserSettings>,
) -> Result<(), String>
```

Include initial_settings in UserMessage::CreateUser construction.

#### 4. `frontend/src/app/app_state/callbacks.rs`

**Store initial_settings for use in UserState creation.** This may require:
- Adding initial_settings to AppState temporarily, OR
- Passing through the callback chain

**Update on_user_state_load_response:**
When `loaded_state` is None (new user), use:
```rust
let new_state = UserState::with_initial_settings(user.id, initial_settings);
```

### Deliverables
- [ ] Registration form has dropdowns for language, dialect, level, gender
- [ ] Callbacks pass initial_settings through the chain
- [ ] WebSocket service includes initial_settings in CreateUser message
- [ ] UserState created with initial_settings for new users
- [ ] Frontend compiles with trunk build
- [ ] Registration flow works end-to-end

### Phase End Reminder
1. Run `cargo test` - 100% pass required
2. Run `cargo clippy` - fix ALL errors, remove dead code
3. Run `trunk build` - verify frontend compiles
4. Manual test: register new user with non-default settings
5. Update this document with status
6. `git add . && git commit -m "Phase 6 (Frontend initial settings UI) complete"`
7. STOP and wait for approval

---

## Status Tracking

| Phase | Status | Date | Notes |
|-------|--------|------|-------|
| 1 | Complete | 2025-12-19 | InviteCode admin flag + backend/persistence |
| 2 | Complete | 2025-12-19 | User V2_Admin migration |
| 3 | Complete | 2025-12-19 | Backend admin flag handling |
| 4 | Complete | 2025-12-19 | InitialUserSettings types |
| 5 | Complete | 2025-12-19 | Backend settings flow |
| 6 | Pending | | Frontend settings UI |

---

## Issues Encountered

(To be filled during implementation)

---

## Test Summary

After all phases complete:
- Shared crate: ~XXX tests (update with actual count)
- Backend: ~XXX tests
- Frontend: compiles with trunk build

All tests must pass before marking implementation complete.
