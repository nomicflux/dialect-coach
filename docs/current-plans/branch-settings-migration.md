# Branch Settings Migration Plan

## Problem Statement

**BUG**: When switching branches, settings (formality, teaching_mode, language, level, language_options) do NOT update to match the branch. This is because these settings are stored at the **UserState** level, not per-branch.

**User's exact words**: "When I switch branches, the settings do not belong to the dialect for the branch I switch to."

**Example**: Switch from Tokyo Japanese (N5, Professional Casual) to Levantine Arabic branch → Shows Hokkaido Japanese, Professional Casual, N1 instead of Arabic settings.

## Root Cause

**UserState** (shared/src/models/user_state.rs:217-228) stores at USER level:
- `selected_language: Language`
- `formality: Formality`
- `teaching_mode: TeachingMode`
- `language_options: LanguageOptions`

**ConversationBranch** (shared/src/models/branch.rs:9-20) only stores:
- `dialect: Dialect`
- `active_plan_id: Option<Uuid>`

**reducer.rs line 135-137** confirms: Branch switch ONLY changes `active_branch_id`:
```rust
Switch(branch_id) => {
    next.active_branch_id = branch_id;
}
```

## Solution Overview

1. Create `BranchSettings` struct for dialect-dependent options
2. Move settings INTO `ConversationBranch`
3. Remove redundant settings from `UserState`
4. Update branch switching to be the ONLY source of truth for conversation settings
5. Update settings UI to indicate "per-branch" nature
6. Add V4 migration for UserState

## User Requirements (Exact Quotes)

- "all dialect-dependent options (formality, teaching mode, level, specific language options) should be put together in their own struct"
- "each branch should have its own settings (with forked branches inheriting from their parent, but if they change subsequently, the parent does not change)"
- "On user creation, settings are set for the initial branch"
- "The settings panel should be clear that conversation settings are per branch"
- "fix test_change_language_reuses_existing_branch()"
- "switching branches also updates language/dialect"

## Explicitly Rejected

- Race conditions or re-render logic as the cause (user explicitly stated this is NOT the problem)
- Any solution that keeps settings at UserState level

---

## New Data Structure

### BranchSettings (NEW)

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BranchSettings {
    pub formality: Formality,
    pub teaching_mode: TeachingMode,
    pub language_level: LanguageLevel,
    pub language_options: LanguageOptions,
}

impl Default for BranchSettings {
    fn default() -> Self {
        Self {
            formality: Formality::Informal,
            teaching_mode: TeachingMode::Immersive,
            language_level: LanguageLevel::default(),
            language_options: LanguageOptions::default(),
        }
    }
}

impl BranchSettings {
    pub fn for_dialect(dialect: Dialect) -> Self {
        Self {
            formality: Formality::Informal,
            teaching_mode: TeachingMode::Immersive,
            language_level: LanguageLevel::default_for_language(dialect.language()),
            language_options: LanguageOptions::default(),
        }
    }
}
```

### ConversationBranch (UPDATED)

```rust
pub struct ConversationBranch {
    pub id: Uuid,
    pub parent_message_id: Option<Uuid>,
    pub leaf_message_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub name: Option<String>,
    pub dialect: Dialect,
    pub message_ids: Vec<Uuid>,
    pub active_plan_id: Option<Uuid>,
    pub settings: BranchSettings,  // NEW FIELD
}
```

### UserState (REMOVE FIELDS)

Remove from UserState:
- `formality: Formality`
- `teaching_mode: TeachingMode`
- `language_options: LanguageOptions`
- `dialect_levels: Vec<DialectLevel>`

Keep in UserState (global, not per-branch):
- `selected_language: Language` (for UI filtering of dialects)
- `user_gender: UserGender` (personal attribute)
- `tts_enabled: bool` (global preference)
- `show_experimental_dialects: bool` (global preference)

---

## Phase 1: Add BranchSettings Struct

**Subagent**: kiss-code-generator

**Code Style Checklist**:
- [ ] Functions <20 lines
- [ ] Pure functions preferred
- [ ] No defensive coding

**Files to Modify**:
- `shared/src/models/branch.rs` - Add BranchSettings struct and update ConversationBranch
- `shared/src/models/mod.rs` - Export BranchSettings

**Deliverables**:
1. `BranchSettings` struct with: formality, teaching_mode, language_level, language_options
2. `ConversationBranch::new()` updated to accept and store settings
3. `BranchSettings::for_dialect(Dialect)` constructor
4. `BranchSettings::default()` implementation
5. Unit tests for BranchSettings

**Phase End**:
- Run `cargo test -p dialect-coach-shared`
- Run `cargo clippy -p dialect-coach-shared -- -D warnings`
- Commit: "Phase 1 (branch settings struct) complete"

---

## Phase 2: Update UserState and Add Accessor Methods

**Subagent**: kiss-code-generator

**Code Style Checklist**:
- [ ] Functions <20 lines
- [ ] Pure functions preferred
- [ ] No defensive coding

**Files to Modify**:
- `shared/src/models/user_state.rs` - Add accessor methods that read from active branch

**Deliverables**:
1. `UserState::active_branch_settings(&self) -> &BranchSettings` method
2. `UserState::active_branch_settings_mut(&mut self) -> &mut BranchSettings` method
3. Update `build_action_context()` to use branch settings
4. Update `create_metadata()` to use branch settings
5. Keep old fields temporarily for migration (will remove in Phase 5)

**Phase End**:
- Run `cargo test -p dialect-coach-shared`
- Run `cargo clippy -p dialect-coach-shared -- -D warnings`
- Commit: "Phase 2 (userstate accessors) complete"

---

## Phase 3: Add V4 Migration

**Subagent**: kiss-code-generator

**Code Style Checklist**:
- [ ] Functions <20 lines
- [ ] Pure functions preferred
- [ ] No defensive coding

**Files to Modify**:
- `shared/src/models/versioning.rs` - Add V4BranchSettings migration

**Deliverables**:
1. Add `V4BranchSettings` variant to `UserStateVersion` enum
2. Add migration step from V3 to V4:
   - Read `formality`, `teaching_mode`, `language_options` from UserState
   - For each branch, look up `dialect_levels` to find level for branch's dialect
   - Create `settings` object on each branch
3. Backward migration (V4 to V3):
   - Extract settings from first branch back to UserState level
4. Update `CURRENT_USER_STATE_VERSION` to V4
5. Update `test_user_state_structure_snapshot` expected fields

**Migration Logic (V3→V4 forward)**:
```rust
fn v3_to_v4_forward(mut data: serde_json::Value) -> serde_json::Value {
    // Extract global settings
    let formality = data.get("formality").cloned().unwrap_or(json!("Informal"));
    let teaching_mode = data.get("teaching_mode").cloned().unwrap_or(json!("Immersive"));
    let language_options = data.get("language_options").cloned().unwrap_or(json!({}));
    let dialect_levels = data.get("dialect_levels").and_then(|v| v.as_array()).cloned().unwrap_or_default();

    // Apply settings to each branch
    if let Some(branches) = data.get_mut("branches").and_then(|v| v.as_array_mut()) {
        for branch in branches {
            let dialect = branch.get("dialect").and_then(|v| v.as_str()).unwrap_or("");
            let level = find_level_for_dialect(&dialect_levels, dialect);

            branch["settings"] = json!({
                "formality": formality,
                "teaching_mode": teaching_mode,
                "language_level": level,
                "language_options": language_options.clone()
            });
        }
    }

    // Remove old fields
    data.as_object_mut().map(|obj| {
        obj.remove("formality");
        obj.remove("teaching_mode");
        obj.remove("language_options");
        obj.remove("dialect_levels");
    });

    data
}
```

**Phase End**:
- Run `cargo test -p dialect-coach-shared`
- Run `cargo clippy -p dialect-coach-shared -- -D warnings`
- Commit: "Phase 3 (V4 migration) complete"

---

## Phase 4: Update Frontend Reducers

**Subagent**: modular-builder

**Code Style Checklist**:
- [ ] Functions <20 lines
- [ ] Pure functions preferred
- [ ] No defensive coding

**Files to Modify**:
- `frontend/src/app/app_state/user/actions.rs` - Update SettingsAction variants
- `frontend/src/app/app_state/user/reducer.rs` - Update reducers to modify branch settings
- `frontend/src/app/app_state/user/helpers.rs` - Update helper functions

**Deliverables**:
1. `reduce_settings` actions write to active branch's settings, not UserState
2. `BranchAction::Switch` also updates `selected_language` to match branch dialect's language
3. `BranchAction::Create` inherits settings from parent branch
4. Remove redundant `ChangeFormality`, `ChangeTeachingMode` etc that modify UserState fields
5. Update `find_or_create_branch_for_dialect` to copy settings from source branch

**Key Changes to reducer.rs**:
```rust
// OLD (broken):
Switch(branch_id) => {
    next.active_branch_id = branch_id;
}

// NEW (fixed):
Switch(branch_id) => {
    next.active_branch_id = branch_id;
    // Update selected_language to match branch's dialect
    if let Some(branch) = next.branches.iter().find(|b| b.id == branch_id) {
        next.selected_language = branch.dialect.language();
    }
}
```

```rust
// Settings actions now modify branch settings
ChangeFormality(formality) => {
    if let Some(branch) = Arc::make_mut(&mut next.branches)
        .iter_mut()
        .find(|b| b.id == next.active_branch_id)
    {
        branch.settings.formality = formality;
    }
}
```

```rust
// Branch creation inherits settings
Create(message_id) => {
    let parent_settings = next.branches
        .iter()
        .find(|b| b.id == next.active_branch_id)
        .map(|b| b.settings.clone())
        .unwrap_or_default();

    let new_branch = ConversationBranch::new(
        Some(message_id),
        None,
        Some(message_id),
        dialect,
        message_ids,
        plan_id,
        parent_settings,  // Inherit from parent
    );
    // ...
}
```

**Phase End**:
- Run `cargo test -p dialect-coach-frontend`
- Run `cargo clippy -p dialect-coach-frontend -- -D warnings`
- Commit: "Phase 4 (frontend reducers) complete"

---

## Phase 5: Update Frontend UI Components

**Subagent**: modular-builder

**Code Style Checklist**:
- [ ] Functions <20 lines
- [ ] Pure functions preferred
- [ ] No defensive coding

**Files to Modify**:
- `frontend/src/components/utility_sidebar/settings.rs` - Read from branch settings
- `frontend/src/app/user_state_callbacks.rs` - Update callbacks if needed

**Deliverables**:
1. Settings panel reads from `user.active_branch_settings()` instead of `user.formality`, `user.teaching_mode`, etc.
2. Add header text: "Conversation Settings (per branch)"
3. Update level display to read from branch settings
4. Update language options display to read from branch settings

**Key Changes to settings.rs**:
```rust
// OLD:
let formality = us.formality;

// NEW:
let settings = us.active_branch_settings();
let formality = settings.formality;
```

**Phase End**:
- Run `cargo test`
- Run `cargo clippy -- -D warnings`
- Commit: "Phase 5 (frontend UI) complete"

---

## Phase 6: Remove Deprecated Fields from UserState

**Subagent**: kiss-code-generator

**Code Style Checklist**:
- [ ] Functions <20 lines
- [ ] No dead code

**Files to Modify**:
- `shared/src/models/user_state.rs` - Remove deprecated fields
- Update all tests that reference removed fields

**Fields to Remove from UserState**:
- `formality: Formality`
- `teaching_mode: TeachingMode`
- `language_options: LanguageOptions`
- `dialect_levels: Vec<DialectLevel>`

**Methods to Remove/Update**:
- Remove `formality_display()` (use branch method)
- Remove `teaching_mode_display()` (use branch method)
- Remove `current_language_option()` (use branch method)
- Remove `get_level_for_dialect()` (obsolete)
- Remove `set_level_for_dialect()` (obsolete)
- Remove `current_language_level()` (use branch method)

**Phase End**:
- Run `cargo test`
- Run `cargo clippy -- -D warnings`
- Commit: "Phase 6 (remove deprecated fields) complete"

---

## Phase 7: Fix Failing Tests

**Subagent**: bug-hunter

**Code Style Checklist**:
- [ ] Functions <20 lines
- [ ] Tests reflect new behavior

**Files to Modify**:
- `frontend/src/app/app_state/user/tests.rs` - Fix `test_change_language_reuses_existing_branch`
- `shared/src/models/versioning.rs` - Update structure snapshot tests

**Specific Test Fixes**:

1. `test_change_language_reuses_existing_branch()` - This test needs to verify:
   - When switching back to Spanish, it finds the existing Spanish branch
   - Branch retains its settings
   - `selected_language` is updated on switch

2. Update all versioning snapshot tests to reflect new structure

**Phase End**:
- Run `cargo test` (100% pass required)
- Run `cargo clippy -- -D warnings`
- Commit: "Phase 7 (fix tests) complete"

---

## Files Changed Summary

| File | Action |
|------|--------|
| `shared/src/models/branch.rs` | Add BranchSettings struct, update ConversationBranch |
| `shared/src/models/mod.rs` | Export BranchSettings |
| `shared/src/models/user_state.rs` | Add accessors, then remove deprecated fields |
| `shared/src/models/versioning.rs` | Add V4 migration |
| `frontend/src/app/app_state/user/actions.rs` | Update actions |
| `frontend/src/app/app_state/user/reducer.rs` | Update reducers |
| `frontend/src/app/app_state/user/helpers.rs` | Update helpers |
| `frontend/src/app/app_state/user/tests.rs` | Fix tests |
| `frontend/src/components/utility_sidebar/settings.rs` | Read from branch |

---

## Testing Verification

After all phases complete:

1. Create new user → Initial branch has default settings
2. Change formality → Only active branch affected
3. Switch branches → Settings change to match branch
4. Fork from message → New branch inherits parent settings
5. Change settings on fork → Parent unaffected
6. Change language → Finds/creates branch for that language, settings preserved

---

## Migration Testing

1. Load V3 user state JSON
2. Run migration to V4
3. Verify each branch has `settings` field populated
4. Verify UserState no longer has `formality`, `teaching_mode`, `language_options`, `dialect_levels`
5. Run backward migration (V4 → V3)
6. Verify UserState fields restored from first branch
