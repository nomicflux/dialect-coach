# Branch Dialect From First Message - Fix Plan

## User Issue

**Date: 2025-11-23**

User reported:
1. "I start and a branch is already there for Cuban spanish (it should probably wait until the first message)"
2. "Then I write a message in Argentinian spanish, and the branch is still labelled Cuban Spanish"

## Current Behavior

- Initial branch created in `UserState::new()` with hardcoded `Dialect::SpanishCuban`
- Branch dialect is set at creation time and never updates
- When user sends first message in different dialect, branch keeps original dialect

## Root Cause

`ConversationBranch.dialect` is a required field (`pub dialect: Dialect`), so empty branches must have a dialect even though they have no messages yet.

## Desired Behavior

- Initial empty branch should have **no dialect** (None)
- When first message is added to a branch, branch's dialect gets set from that message
- Branch dialect display should show "No dialect yet" or similar when None

## Solution

Make `dialect` optional on `ConversationBranch`:
```rust
pub struct ConversationBranch {
    pub id: Uuid,
    pub parent_message_id: Option<Uuid>,
    pub leaf_message_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub name: Option<String>,
    pub dialect: Option<Dialect>,  // Changed from Dialect to Option<Dialect>
}
```

## Implementation Plan

### Phase 1: Make dialect optional on ConversationBranch

**Changes:**

1. **Modify `shared/src/models/branch.rs`:**
   - Change `pub dialect: Dialect` to `pub dialect: Option<Dialect>`
   - Update `new()` to accept `Option<Dialect>`
   - Update tests

2. **Update `shared/src/models/user_state.rs`:**
   - `UserState::new()` - create initial branch with `dialect: None`
   - `reset_branches_to_root()` - create branch with `dialect: None`
   - `branch_dialect()` - return `Option<Dialect>` instead of `Dialect`
   - `build_branches_from_leaves()` - use `Some(dialect)` when dialect exists

3. **Update frontend CreateBranch:**
   - `frontend/src/app/app_state.rs` - pass `Some(selected_dialect)` or `None`

4. **Update all tests:**
   - Change `Dialect::SpanishMexican` to `Some(Dialect::SpanishMexican)`
   - Add tests for None case

**Files Modified:**
- `shared/src/models/branch.rs`
- `shared/src/models/user_state.rs`
- `frontend/src/app/app_state.rs`
- All test files

**Deliverables:**
- ConversationBranch.dialect is Optional<Dialect>
- Initial branch has dialect: None
- Tests pass
- Clippy clean

**Phase End Checklist:**
- [x] Run `cargo test` - 100% success required
- [x] Run `cargo clippy` - Fix ALL errors and warnings
- [x] No dead code
- [x] Git commit: `git commit -m "Phase 1: Make branch dialect optional"`

**Phase 1 Status: COMPLETE**

---

### Phase 2: Set branch dialect from first message

**Changes:**

1. **Add method to set branch dialect:**
   ```rust
   impl ConversationBranch {
       pub fn set_dialect_if_none(&mut self, dialect: Dialect) {
           if self.dialect.is_none() {
               self.dialect = Some(dialect);
           }
       }
   }
   ```

2. **Update when messages are added:**
   - When adding first message to a branch with dialect: None
   - Set branch dialect to message's dialect
   - Find where messages are added and update branch

3. **Backend: Update branch dialect on message add:**
   - `backend/src/websocket/` - when processing user message
   - Get active branch, check if dialect is None
   - If None, set to message dialect

**Files Modified:**
- `shared/src/models/branch.rs`
- `backend/src/websocket/` (message handling)

**Deliverables:**
- Branch dialect updates from first message
- Tests verify dialect is set correctly
- Tests pass
- Clippy clean

**Phase End Checklist:**
- [x] Run `cargo test` - 100% success required
- [x] Run `cargo clippy` - Fix ALL errors and warnings
- [x] No dead code
- [x] Git commit: `git commit -m "Phase 2: Set branch dialect from first message"`

**Phase 2 Status: COMPLETE**

---

### Phase 3: Update UI to handle optional dialect

**Changes:**

1. **Update `frontend/src/components/main_content.rs`:**
   - `get_active_dialect()` - return `Option<Dialect>` or use selected_dialect as fallback
   - Handle None case when filtering learning items

2. **Update `frontend/src/components/branch_sidebar.rs`:**
   - Display "New conversation" or similar when dialect is None
   - Display dialect name when Some(dialect)

3. **Update learning item filtering:**
   - If branch dialect is None, use selected_dialect as fallback
   - Or: don't filter (show all items) until dialect is set

**Files Modified:**
- `frontend/src/components/main_content.rs`
- `frontend/src/components/branch_sidebar.rs`

**Deliverables:**
- UI handles None dialect gracefully
- Empty branches show appropriate label
- Learning items filter correctly
- Tests pass
- Clippy clean

**Phase End Checklist:**
- [x] Run `cargo test` - 100% success required
- [x] Run `cargo clippy` - Fix ALL errors and warnings
- [x] No dead code
- [x] Manual UI testing
- [x] Git commit: `git commit -m "Phase 3: UI handles optional dialect"`

**Phase 3 Status: COMPLETE**

---

## Success Criteria

- [x] Initial empty branch has dialect: None
- [x] Branch dialect is set from first message's dialect
- [x] UI shows appropriate label for empty branches ("New conversation")
- [x] Cuban Spanish → Argentinian Spanish works correctly
- [x] All tests pass (100% success) - 315 tests passing
- [x] No clippy warnings
- [x] No dead code

## User Decisions

**Date: 2025-11-23**

1. **Learning item filtering when branch dialect is None:**
   - User specified: "No learning items shown"
   - Empty branches (dialect: None) show empty learning panel
   - Only show items when branch has a dialect

2. **Branch creation from UI:**
   - New branches start with dialect: None (for consistency)
   - Dialect gets set from first message

## Implementation Status

- [x] Phase 1: Make dialect optional
- [x] Phase 2: Set dialect from first message
- [x] Phase 3: Update UI

**Current Phase:** ALL PHASES COMPLETE ✅

**Blockers:** None

**Summary:**
- Branch dialect is now optional (Option<Dialect>)
- Initial branches start with dialect: None
- First message sets the branch dialect automatically
- UI shows "New conversation" for empty branches
- Learning items only show when branch has a dialect
- All 315 tests pass
- Clippy clean
