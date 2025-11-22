# Language-Scoped Branches Implementation Plan

## Overview

Transform branches into language-scoped conversation contexts where each branch has a specific dialect and only shows learning items from that dialect.

## User Requirements Captured

**Date: 2025-11-22**

User specified:
- "Branches = language context works well"
- "Learning items should actually probably be per-dialect, really"
- "No migrations - I'll wipe the DB clean and restart"
- "Cross-language branches are out of scope"

**Core requirement:** Learning items from one dialect should NOT be analyzed/graded when user is speaking in a different dialect.

**Secondary concern:** Conversation history from one language could muddle language directives when switching languages mid-conversation.

## Architectural Decisions

### Decision 1: Branches Own Dialect
**Decision:** Add `dialect: Dialect` field to `ConversationBranch`

**Reasoning:**
- Branches already represent conversation threads
- Natural UX: "Spanish Practice" branch vs "Arabic Study" branch
- Clean data isolation per language
- Prevents conversation history muddling

### Decision 2: Learning Items Per-Dialect
**Decision:** Add `dialect: Dialect` field to `LearningItem`

**Reasoning:**
- User wants per-dialect granularity ("really")
- More precise than per-language
- Mexican Spanish errors stay in Mexican Spanish context
- Allows future cross-dialect comparison features

### Decision 3: No Migrations
**Decision:** Breaking change - existing data incompatible

**Reasoning:**
- User will wipe DB clean
- Simpler implementation
- No backward compatibility needed

### Decision 4: Filter Learning Items by Branch Dialect
**Decision:** When displaying/analyzing learning items, filter by `branch.dialect`

**Reasoning:**
- Solves core requirement (no cross-dialect analysis)
- Clean separation of concerns
- Agent only sees relevant learning context

## Data Structure Changes

### ConversationBranch (shared/src/models/branch.rs)
```rust
// OLD
pub struct ConversationBranch {
    pub id: Uuid,
    pub parent_message_id: Option<Uuid>,
    pub leaf_message_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub name: Option<String>,
}

// NEW
pub struct ConversationBranch {
    pub id: Uuid,
    pub parent_message_id: Option<Uuid>,
    pub leaf_message_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub name: Option<String>,
    pub dialect: Dialect,  // NEW - required field
}
```

### LearningItem (shared/src/models/user_state.rs)
```rust
// OLD
pub struct LearningItem {
    pub item: LearningItemType,
    pub score: u8,
}

// NEW
pub struct LearningItem {
    pub item: LearningItemType,
    pub score: u8,
    pub dialect: Dialect,  // NEW - required field
}
```

### UserState Implications
- `selected_dialect` still exists (for UI state)
- Active branch's dialect determines conversation context
- When switching branches, UI updates to show that branch's dialect

## Implementation Phases

---

## Phase 1: Add dialect field to ConversationBranch

**Goal:** Update branch data structure to include dialect

**Changes:**

1. **Modify `shared/src/models/branch.rs`:**
   - Add `pub dialect: Dialect` field to `ConversationBranch`
   - Update `new()` constructor to accept `dialect: Dialect` parameter
   - Update serialization/deserialization

2. **Update `shared/src/models/user_state.rs`:**
   - Update `rebuild_branches_from_history()` to set dialect from first message in branch
   - Update `create_new_branch()` calls to pass dialect

3. **Update all branch creation sites:**
   - `frontend/src/app/app_state.rs` - CreateBranch action
   - `backend/src/websocket/` - Any server-side branch creation
   - Pass current `selected_dialect` when creating branches

4. **Update all tests:**
   - All test code that creates branches must now provide dialect
   - Use `Dialect::MexicanSpanish` as test default

**Files Modified:**
- `shared/src/models/branch.rs`
- `shared/src/models/user_state.rs`
- `frontend/src/app/app_state.rs`
- All test files creating branches

**Deliverables:**
- ConversationBranch has dialect field
- All branch creation passes dialect
- Tests pass
- Clippy clean

**Phase End Checklist:**
- [x] Run `cargo test` - 100% success required
- [x] Run `cargo clippy` - Fix ALL errors and warnings
- [x] No dead code
- [x] Git commit: `git commit -m "Phase 1: Add dialect to ConversationBranch"`

**Phase 1 Status: COMPLETE**

**Implementation Summary:**
- Added `dialect: Dialect` field to ConversationBranch in shared/src/models/branch.rs
- Updated `new()` constructor to accept dialect parameter
- Updated `rebuild_branches_from_history()` to extract dialect from first message
- Added `branch_dialect()` helper method to get dialect from messages
- Updated CreateBranch action in frontend to pass selected_dialect
- Updated all 7 test files to use Dialect::SpanishMexican
- All tests pass (313 total)
- Clippy clean

---

## Phase 2: Add dialect field to LearningItem

**Goal:** Tag learning items with their dialect

**Changes:**

1. **Modify `shared/src/models/user_state.rs`:**
   - Add `pub dialect: Dialect` field to `LearningItem`
   - Update constructor/builder pattern to accept dialect

2. **Update all learning item creation sites:**
   - `frontend/src/components/main_content.rs` - Save translation with `active_branch.dialect`
   - `backend/src/agent_service/response.rs` - Create learning items with dialect
   - `backend/src/websocket/agents.rs` - Add learning items with dialect
   - Get dialect from active branch or current selected_dialect

3. **Update serialization tests:**
   - Verify LearningItem JSON includes dialect field
   - Test deserialization with dialect

4. **Update all tests:**
   - All test code creating LearningItem must now provide dialect

**Files Modified:**
- `shared/src/models/user_state.rs`
- `frontend/src/components/main_content.rs`
- `backend/src/agent_service/response.rs`
- `backend/src/websocket/agents.rs`
- All test files creating learning items

**Deliverables:**
- LearningItem has dialect field
- All creation sites pass dialect
- Tests pass
- Clippy clean

**Phase End Checklist:**
- [ ] Run `cargo test` - 100% success required
- [ ] Run `cargo clippy` - Fix ALL errors and warnings
- [ ] No dead code
- [ ] Git commit: `git commit -m "Phase 2: Add dialect to LearningItem"`

---

## Phase 3: Filter learning items by branch dialect

**Goal:** Only show/analyze learning items matching active branch's dialect

**Changes:**

1. **Add filtering method to `UserState`:**
   ```rust
   impl UserState {
       pub fn get_learning_items_for_dialect(&self, dialect: &Dialect) -> Vec<&LearningItem> {
           self.learning_items.iter()
               .filter(|item| item.dialect == *dialect)
               .collect()
       }
   }
   ```

2. **Update learning panel display:**
   - `frontend/src/components/learning_panel.rs`
   - Filter items by active branch's dialect before rendering
   - Get dialect from `user_state.branches[active_branch_id].dialect`

3. **Update analysis agent:**
   - `backend/src/agent_service/analysis.rs` or `response.rs`
   - Filter learning items by dialect before passing to analysis
   - Only analyze items matching current conversation dialect

4. **Update learning item retrieval:**
   - Anywhere learning items are displayed or used
   - Filter by dialect consistently

**Files Modified:**
- `shared/src/models/user_state.rs`
- `frontend/src/components/learning_panel.rs`
- `backend/src/agent_service/analysis.rs` or equivalent
- `backend/src/websocket/agents.rs`

**Deliverables:**
- Learning items filtered by branch dialect
- Analysis only considers dialect-specific items
- UI shows only relevant items
- Tests pass
- Clippy clean

**Phase End Checklist:**
- [ ] Run `cargo test` - 100% success required
- [ ] Run `cargo clippy` - Fix ALL errors and warnings
- [ ] No dead code
- [ ] Git commit: `git commit -m "Phase 3: Filter learning items by dialect"`

---

## Phase 4: Update UI to show branch dialect

**Goal:** Make branch dialect visible in UI

**Changes:**

1. **Update branch sidebar display:**
   - `frontend/src/components/branch_sidebar.rs`
   - Show dialect icon/label next to branch name
   - Example: "🇲🇽 Spanish Practice" or "Mexican Spanish | Conversation 1"

2. **Update branch creation flow:**
   - When creating new branch, use current `selected_dialect`
   - Or: add dialect picker to branch creation (optional enhancement)

3. **Update branch switching:**
   - When switching branches, update UI to reflect branch's dialect
   - Consider: should `selected_dialect` update to match branch dialect?
   - Or: keep them separate (branch dialect vs UI dialect preference)?

4. **Add visual indicators:**
   - Color-code branches by language family?
   - Flag emoji or language name in branch list
   - Active branch shows its dialect clearly

**Files Modified:**
- `frontend/src/components/branch_sidebar.rs`
- `frontend/src/app/app_state.rs` (if updating selected_dialect on switch)
- CSS for visual indicators (optional)

**Deliverables:**
- Branch dialect visible in UI
- Clear which language context user is in
- Tests pass
- Clippy clean

**Phase End Checklist:**
- [ ] Run `cargo test` - 100% success required
- [ ] Run `cargo clippy` - Fix ALL errors and warnings
- [ ] No dead code
- [ ] Manual UI testing - branch dialect displays correctly
- [ ] Git commit: `git commit -m "Phase 4: Show branch dialect in UI"`

---

## Success Criteria

After all phases complete:
- [ ] Each branch has a specific dialect
- [ ] Learning items tagged with dialect
- [ ] Learning panel only shows items from active branch's dialect
- [ ] Analysis agent only grades items from current dialect
- [ ] UI clearly shows which dialect context user is in
- [ ] Switching branches switches language context
- [ ] No cross-dialect data bleeding
- [ ] All tests pass (100% success)
- [ ] No clippy warnings
- [ ] No dead code

## Future Enhancements (Out of Scope)

User indicated these are for "far future":
- Cross-language branches (bilingual practice)
- Learning item migration between dialects
- Cross-dialect comparison features
- Dialect picker on branch creation (vs using current selected_dialect)

## Edge Cases to Consider

1. **What if user changes selected_dialect while in a branch?**
   - Option A: Prevent it (lock UI to branch dialect)
   - Option B: Allow it (selected_dialect and branch dialect can differ)
   - **Recommendation:** Option B - selected_dialect is UI preference, branch owns conversation

2. **Default branch dialect on first load?**
   - Use user's current `selected_dialect`
   - First branch inherits initial dialect choice

3. **Empty branches with no messages?**
   - Still need dialect - use selected_dialect at creation time
   - Or: allow dialect to be None until first message? (No - simpler to require it)

---

## Implementation Status

- [x] Phase 1: Add dialect to ConversationBranch
- [ ] Phase 2: Add dialect to LearningItem
- [ ] Phase 3: Filter learning items by dialect
- [ ] Phase 4: Update UI to show branch dialect

**Current Phase:** Phase 1 complete - ready for Phase 2

**Blockers:** None

**Notes:**
- Phase 1 completed successfully
- All tests pass (313 total)
- Clippy clean
- Ready for Phase 2 upon approval
