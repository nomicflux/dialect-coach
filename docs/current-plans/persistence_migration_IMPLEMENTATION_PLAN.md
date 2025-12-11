# Persistence Migration System - Implementation Plan

## Summary
Implement versioned persistence for `User` and `UserState` with bidirectional migrations, enabling safe schema evolution.

## User Agreements (Exact Quotes)
- "Separate version enums, separate migration services (with a common trait)"
- "Migrations eagerly on data load (the entire point is to deal with data saved on one version and loaded on another; it literally CANNOT BE MADE INTO A USER STATE without the migration)"
- "Bidirectional migrations"
- "For this step, we will delete the database and start fresh. No backwards compatibility for these changes."
- "Tests are compile time" - migration trait impls enforced by type system
- Structure snapshot tests break on field changes, forcing version bump + migration

## Explicitly Rejected
- Storing version as field inside `UserState` (use wrapper metadata instead)
- Single version enum for both User and UserState (use separate enums)
- Lazy migration on save (must migrate eagerly on load)
- Forward-only migration (must support bidirectional)
- Backward compatibility with pre-versioned data (delete DB and start fresh)

## Architecture Overview

### Storage Format (Wrapper Approach)
```rust
// Stored in sled as JSON
struct VersionedData<V> {
    version: V,
    data: serde_json::Value,  // Raw JSON, deserialized based on version
}
```

**Load flow:**
1. Deserialize `VersionedData` wrapper from sled
2. Inspect `version` field
3. Deserialize `data` into correct version struct (e.g., `UserStateV1`)
4. Run migration chain to current version
5. Return current `UserState`

**Save flow:**
1. Wrap current `UserState` with current version tag
2. Serialize wrapper to sled

### Migration Trait (Common Interface)
```rust
pub trait Migration<From, To> {
    fn migrate_forward(from: From) -> To;
    fn migrate_backward(to: To) -> From;
}
```

### Compile-Time Enforcement
- Each version variant requires migration impl to adjacent versions
- Adding V3 requires `impl Migration<UserStateV2, UserStateV3>`

### Test Enforcement (Structure Snapshots)
- Tests assert exact field structure of `User` and `UserState`
- Any field change breaks test
- Test failure message instructs: bump version, add migration, update snapshot

---

## Phase 1: Version Enums and Wrapper Types

**Subagent**: `kiss-code-generator`

### Code Style Checklist
- [ ] Functions < 20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No future-proofing

### Deliverables
1. `UserVersion` enum with V1 variant
2. `UserStateVersion` enum with V1 variant
3. `VersionedData<V>` generic wrapper struct
4. Unit tests for serialization/deserialization of wrapper

### Files to Create
- **CREATE** `shared/src/models/versioning.rs`:
  - `UserVersion` enum (V1)
  - `UserStateVersion` enum (V1)
  - `VersionedData<V>` struct with `version: V` and `data: serde_json::Value`
  - Derive `Serialize`, `Deserialize`, `Clone`, `Debug`, `PartialEq` for all
  - `impl Default` for both version enums returning V1

### Files to Modify
- **MODIFY** `shared/src/models/mod.rs`:
  - Add `pub mod versioning;`
  - Add re-exports for `UserVersion`, `UserStateVersion`, `VersionedData`

### Phase End Checklist
- [ ] Run `cargo test -p dialect-coach-shared`
- [ ] Run `cargo clippy --all` - fix ALL warnings including dead code
- [ ] Update status document with progress
- [ ] Commit: `git commit -m "Phase 1 (version enums and wrapper types) complete"`

---

## Phase 2: Migration Trait Framework

**Subagent**: `kiss-code-generator`

### Code Style Checklist
- [ ] Functions < 20 lines
- [ ] Pure functions where possible
- [ ] Trait is minimal - only required methods

### Deliverables
1. `Migration<From, To>` trait with `migrate_forward` and `migrate_backward`
2. Type aliases for versioned structs: `UserV1`, `UserStateV1` (currently same as `User`, `UserState`)
3. Constants for current versions: `CURRENT_USER_VERSION`, `CURRENT_USER_STATE_VERSION`
4. Migration chain runner functions

### Design Details

**Migration Trait (per-transition):**
```rust
/// Trait for bidirectional migration between version transitions.
/// Each implementation handles ONE transition (e.g., V1→V2 or V2→V1).
/// Implementations must be pure functions with no side effects.
pub trait Migration<From, To> {
    fn migrate_forward(from: From) -> To;
    fn migrate_backward(to: To) -> From;
}
```

**Migration Chain Architecture:**
- Migrations are defined per transition: V1→V2, V2→V3, etc.
- Chain runner executes sequential migrations: V1→V2→V3 (forward) or V3→V2→V1 (backward)
- NO version matching/branching - just run the chain based on start/end versions

**Example (when V2 exists in future):**
```rust
struct UserStateV1ToV2;
impl Migration<UserStateV1, UserStateV2> for UserStateV1ToV2 {
    fn migrate_forward(from: UserStateV1) -> UserStateV2 { ... }
    fn migrate_backward(to: UserStateV2) -> UserStateV1 { ... }
}
```

For V1 (initial version), no migration impls exist yet. The framework is set up for future versions.

### Files to Modify
- **MODIFY** `shared/src/models/versioning.rs`:
  - Add `Migration<From, To>` trait
  - Add type aliases: `pub type UserV1 = User;` and `pub type UserStateV1 = UserState;`
  - Add constants: `pub const CURRENT_USER_VERSION: UserVersion = UserVersion::V1;`
  - Add constants: `pub const CURRENT_USER_STATE_VERSION: UserStateVersion = UserStateVersion::V1;`
  - Add `migrate_user_state_to_current(from_version: UserStateVersion, data: UserStateV1) -> UserState`
    - For V1: no-op, just returns data (V1 is current)
    - When V2 added: runs V1→V2 migration chain
  - Add `migrate_user_to_current(from_version: UserVersion, data: UserV1) -> User`
    - Same pattern for User
  - Add tests for migration chain (currently just tests V1 no-op)

### Phase End Checklist
- [ ] Run `cargo test -p dialect-coach-shared`
- [ ] Run `cargo clippy --all` - fix ALL warnings including dead code
- [ ] Update status document with progress
- [ ] Commit: `git commit -m "Phase 2 (migration trait framework) complete"`

---

## Phase 3: Structure Snapshot Tests

**Subagent**: `kiss-code-generator`

### Code Style Checklist
- [ ] Test names clearly indicate purpose
- [ ] Failure messages are actionable for AI agents
- [ ] Tests are simple assertions, not complex logic

### Deliverables
1. `test_user_structure_snapshot` - breaks if `User` fields change
2. `test_user_state_structure_snapshot` - breaks if `UserState` fields change
3. Clear failure messages directing to migration creation

### Test Design

Tests will assert on the exact set of field names in serialized JSON. When a field is added, removed, or renamed, the test fails.

**Failure message format:**
```
STRUCTURE CHANGE DETECTED in UserState!

Expected fields: ["user_id", "learning_items", ...]
Actual fields:   ["user_id", "learning_items", "new_field", ...]

TO FIX THIS TEST:
1. Add new version variant to UserStateVersion enum (e.g., V2)
2. Create migration in shared/src/models/versioning.rs:
   impl Migration<UserStateV1, UserStateV2> for UserStateMigrations {
       fn migrate_forward(from: UserStateV1) -> UserStateV2 { ... }
       fn migrate_backward(to: UserStateV2) -> UserStateV1 { ... }
   }
3. Update migrate_user_state_to_current() to handle new version
4. Update CURRENT_USER_STATE_VERSION constant
5. Update this test's expected fields to match new structure
```

### Files to Modify
- **MODIFY** `shared/src/models/versioning.rs`:
  - Add `CURRENT_USER_VERSION: UserVersion = UserVersion::V1`
  - Add `CURRENT_USER_STATE_VERSION: UserStateVersion = UserStateVersion::V1`
  - Add `#[cfg(test)] mod tests` with structure snapshot tests
  - Add helper function `extract_field_names(value: &serde_json::Value) -> Vec<String>`

### Phase End Checklist
- [ ] Run `cargo test -p dialect-coach-shared`
- [ ] Run `cargo clippy --all` - fix ALL warnings including dead code
- [ ] Update status document with progress
- [ ] Commit: `git commit -m "Phase 3 (structure snapshot tests) complete"`

---

## Phase 4: Persistence Layer Integration

**Subagent**: `modular-builder`

### Code Style Checklist
- [ ] Functions < 20 lines
- [ ] Helper functions for serialize/deserialize with version
- [ ] No defensive coding
- [ ] Error messages are clear

### Deliverables
1. `serialize_versioned<T, V>()` helper function
2. `deserialize_versioned_user_state()` helper function
3. `deserialize_versioned_user()` helper function
4. Updated `SledPersistence::save()` to use versioned wrapper
5. Updated `SledPersistence::load()` to use versioned wrapper with migration
6. Updated `SledPersistence::create_user()` to use versioned wrapper
7. Updated `SledPersistence::load_user_by_username()` to use versioned wrapper with migration
8. Updated `SledPersistence::load_user_by_id()` to use versioned wrapper with migration

### Files to Modify
- **MODIFY** `backend/src/persistence/sled.rs`:
  - Add `use dialect_coach_shared::{UserStateVersion, UserVersion, VersionedData, CURRENT_USER_STATE_VERSION, CURRENT_USER_VERSION, migrate_user_state_to_current, migrate_user_to_current};`
  - Add `serialize_versioned_user_state(state: &UserState) -> Result<Vec<u8>>`
  - Add `deserialize_versioned_user_state(bytes: &[u8]) -> Result<UserState>`
  - Add `serialize_versioned_user(user: &User, password_hash: &str) -> Result<Vec<u8>>`
  - Add `deserialize_versioned_user(bytes: &[u8]) -> Result<(User, String)>`
  - Update `save()` to call `serialize_versioned_user_state()`
  - Update `load()` to call `deserialize_versioned_user_state()`
  - Update `create_user()` to call `serialize_versioned_user()`
  - Update `load_user_by_username()` to call `deserialize_versioned_user()`
  - Update `load_user_by_id()` to call `deserialize_versioned_user()`

- **MODIFY** `backend/src/persistence/mod.rs`:
  - Update `UserRecord` to be wrapped in versioned format, or create `VersionedUserRecord`

### Files to Delete
- **DELETE** existing database file (manual step - user confirmed fresh start)

### Phase End Checklist
- [ ] Delete existing database: `rm -rf data/dialect-coach.db`
- [ ] Run `cargo test --all`
- [ ] Run `cargo clippy --all` - fix ALL warnings including dead code
- [ ] Manual test: start server, create user, verify login works
- [ ] Update status document with progress
- [ ] Commit: `git commit -m "Phase 4 (persistence layer integration) complete"`

---

## Phase 5: In-Memory Persistence Update

**Subagent**: `kiss-code-generator`

### Code Style Checklist
- [ ] Functions < 20 lines
- [ ] Consistent with sled implementation

### Deliverables
1. Updated `InMemoryPersistence` to use same versioned approach (for test consistency)

### Files to Modify
- **MODIFY** `backend/src/persistence/in_memory.rs`:
  - Store `VersionedData<UserStateVersion>` internally (or just use current version since in-memory doesn't persist across restarts)
  - Ensure tests pass with versioned approach

### Phase End Checklist
- [ ] Run `cargo test --all`
- [ ] Run `cargo clippy --all` - fix ALL warnings including dead code
- [ ] Update status document with progress
- [ ] Commit: `git commit -m "Phase 5 (in-memory persistence update) complete"`

---

## Phase 6: Documentation and Final Validation

**Subagent**: `kiss-code-generator`

### Deliverables
1. Add inline documentation to `Migration` trait explaining usage
2. Add example in `versioning.rs` showing how to add V2 (as doc comment)
3. Final integration test verifying full flow

### Example Documentation (to add as doc comment)
```rust
/// # Adding a New Version
///
/// When you need to add a field to UserState (e.g., `is_admin: bool`):
///
/// 1. Add `V2` variant to `UserStateVersion`
/// 2. Create `UserStateV2` type alias (or struct if fields differ significantly)
/// 3. Implement migration:
///    ```
///    impl Migration<UserStateV1, UserStateV2> for UserStateMigrations {
///        fn migrate_forward(from: UserStateV1) -> UserStateV2 {
///            UserStateV2 {
///                ..from,
///                is_admin: false,  // default value for new field
///            }
///        }
///        fn migrate_backward(to: UserStateV2) -> UserStateV1 {
///            // Remove the new field
///            UserStateV1 { ..to }  // is_admin is dropped
///        }
///    }
///    ```
/// 4. Update `CURRENT_USER_STATE_VERSION` to `V2`
/// 5. Update `migrate_user_state_to_current()` to handle V1 -> V2
/// 6. Update structure snapshot test expected fields
```

### Files to Modify
- **MODIFY** `shared/src/models/versioning.rs`:
  - Add module-level doc comment with overview
  - Add doc comments to `Migration` trait with example
  - Add integration test `test_migration_roundtrip`

### Phase End Checklist
- [ ] Run `cargo test --all`
- [ ] Run `cargo clippy --all` - fix ALL warnings including dead code
- [ ] Manual test: full user flow (create account, use app, restart server, verify state loads)
- [ ] Update status document marking feature COMPLETE
- [ ] Final commit: `git commit -m "Phase 6 (persistence migration system complete)"`

---

## Critical Files Summary

| File | Changes |
|------|---------|
| `shared/src/models/versioning.rs` | NEW: Version enums, Migration trait, VersionedData, snapshot tests |
| `shared/src/models/mod.rs` | Add versioning module and re-exports |
| `backend/src/persistence/sled.rs` | Versioned serialize/deserialize, migration on load |
| `backend/src/persistence/mod.rs` | VersionedUserRecord or updated UserRecord |
| `backend/src/persistence/in_memory.rs` | Consistency with versioned approach |

---

## Status Tracking

Create `docs/current-plans/persistence_migration_STATUS.md` after Phase 1 to track:
- [ ] Phase 1: Version enums and wrapper types
- [ ] Phase 2: Migration trait framework
- [ ] Phase 3: Structure snapshot tests
- [ ] Phase 4: Persistence layer integration
- [ ] Phase 5: In-memory persistence update
- [ ] Phase 6: Documentation and final validation
