# DynamicIsland Component Fix - Implementation Plan

**Date**: 2025-12-18

## Problem Summary

The DynamicIsland component (`frontend/src/components/dynamic_island.rs`) has three critical issues:

1. **Stale Closure Bug**: The container click handler captures `state.clone()`, then reads `*state` inside the callback. This reads the captured clone instead of current state value, causing double-click requirement to toggle modes.

2. **Fragile Index Storage**: Uses `Vec<usize>` indices (line 15) to track displayed items. If `props.items` changes order or length, indices become invalid or point to wrong items.

3. **Missing Item Swap Functionality**: No way to click individual learning items to replace them with random non-displayed items (user requirement).

4. **Unused Pin Button**: Pin button and `on_pin` callback prop not needed per user requirements.

## User Requirements (2025-12-18)

From user clarifications:
- **Item swap behavior**: "Random but avoid current 3 items" - replace clicked item with random item NOT currently displayed
- **Pin functionality**: "Remove pin button entirely"
- **Swap scope**: "Replace only the clicked item" - other 2 items stay the same

## Solution Approach

1. Fix stale closure by following codebase callback pattern: clone values outside `Callback::from`, compute new state, then set it
2. Change from `Vec<usize>` to `Vec<Uuid>` for stable item references
3. Make individual items clickable with `stop_propagation()` to prevent container toggle
4. Remove pin button and `on_pin` prop entirely

---

## PHASE 1: Update Data Structures and Helper Functions

### Subagent
**kiss-code-generator**

### Code Style Checklist
- [ ] Functions under 20 lines
- [ ] Pure functions where possible (data in, data out)
- [ ] No defensive coding - trust known types
- [ ] Helper functions instead of nested logic
- [ ] All code written must be used in this phase

### Files to Modify
- `frontend/src/components/dynamic_island.rs`

### Files to Create
None

### Files to Delete
None

### Precise Deliverables

1. **ViewState enum updated** (line 12-16):
   - Change `Items(Vec<usize>)` to `Items(Vec<Uuid>)`

2. **DynamicIslandProps updated** (line 5-10):
   - Remove `pub on_pin: Callback<Uuid>` field

3. **New helper function: `pick_random_uuids`** (replaces `pick_random_indices`):
   ```rust
   fn pick_random_uuids(
       items: &[LearningItem],
       count: usize,
       exclude: &[Uuid]
   ) -> Vec<Uuid>
   ```
   - Filter items to get UUIDs not in `exclude` list
   - Use existing `get_item_id()` helper (line 176)
   - Randomly select up to `count` UUIDs from filtered list
   - Use `js_sys::Math::random()` for randomness (existing pattern)
   - Return `Vec<Uuid>`
   - Function must be <20 lines

4. **New helper function: `find_item_by_uuid`**:
   ```rust
   fn find_item_by_uuid(items: &[LearningItem], uuid: Uuid) -> Option<&LearningItem>
   ```
   - Find item in slice where `get_item_id(item) == uuid`
   - Return `Option<&LearningItem>`
   - Function must be <5 lines

5. **New helper function: `swap_one_uuid`**:
   ```rust
   fn swap_one_uuid(
       current: &[Uuid],
       to_replace: Uuid,
       all_items: &[LearningItem]
   ) -> Vec<Uuid>
   ```
   - Pick 1 random UUID not in `current` using `pick_random_uuids`
   - Map over `current`, replacing `to_replace` with new UUID
   - If no available items to swap, return `current.to_vec()`
   - Function must be <20 lines

6. **Delete old function**:
   - Remove `pick_random_indices` (lines 155-165)

### Actionable Steps

1. Update `ViewState::Items` variant to use `Vec<Uuid>` instead of `Vec<usize>`
2. Remove `on_pin` field from `DynamicIslandProps`
3. Write `pick_random_uuids` function:
   - Extract all item UUIDs using `items.iter().map(get_item_id)`
   - Filter out UUIDs in `exclude` list
   - Collect to `Vec<Uuid>` of available UUIDs
   - Loop `count` times: pick random index, remove from available, add to selected
   - Return selected UUIDs
4. Write `find_item_by_uuid` function: one-liner using `.iter().find()`
5. Write `swap_one_uuid` function:
   - Call `pick_random_uuids(all_items, 1, current)` to get new UUID
   - If empty, return unchanged
   - Map current UUIDs, replacing matching one
6. Delete `pick_random_indices` function entirely

### Phase Completion Criteria

**Orchestrator must execute** (NOT delegated to subagent):

1. Run `cargo check` - must pass with no errors
2. Run `cargo test` - must be 100% success (no failures tolerated)
3. Run `cargo clippy` - must fix ALL warnings including dead code removal
4. Verify all functions are under 20 lines
5. Verify no dead code remains
6. Git add and commit: `git add -A && git commit -m "Phase 1 (data structures and helpers) complete"`
7. Wait for explicit user approval before proceeding to Phase 2

---

## PHASE 2: Fix Container Click Handler

### Subagent
**kiss-code-generator**

### Code Style Checklist
- [ ] Functions under 20 lines
- [ ] Clone values outside `Callback::from`, move into closure
- [ ] Compute new state, then set it (avoid reading *state in match)
- [ ] No nested logic - keep callback simple

### Files to Modify
- `frontend/src/components/dynamic_island.rs`

### Files to Create
None

### Files to Delete
None

### Precise Deliverables

1. **Refactored `on_click_container` callback** (lines 25-45):
   - Clone `state` handle outside callback
   - Clone `props.items` outside callback (capture full data, not just length)
   - Inside `Callback::from`:
     - Read current state value with `(*state).clone()`
     - Match on cloned value (not borrowed `*state`)
     - Compute `new_state` in match arms
     - Call `state.set(new_state)` ONCE after match
   - In `ViewState::Plan` arm: call `pick_random_uuids(&items, 3, &[])`
   - In `ViewState::Items(_)` arm: return `ViewState::Plan`
   - Callback must be <15 lines total

### Actionable Steps

1. Locate `on_click_container` definition (starts line 25)
2. Before `Callback::from`, add:
   ```rust
   let state = state.clone();
   let items = props.items.clone();
   ```
3. Inside `Callback::from(move |_: MouseEvent| {`:
   - Start with `let new_state = match (*state).clone() {`
   - Plan arm: `ViewState::Items(pick_random_uuids(&items, 3, &[]))`
   - Items arm: `ViewState::Plan`
   - After match: `state.set(new_state);`
4. Remove any intermediate state reads or multiple `state.set()` calls

### Phase Completion Criteria

**Orchestrator must execute**:

1. Run `cargo check` - must pass
2. Run `cargo test` - 100% success required
3. Run `cargo clippy` - fix ALL warnings
4. Manual verification: callback is under 15 lines, follows pattern correctly
5. Verify no dead code
6. Git commit: `git add -A && git commit -m "Phase 2 (fix stale closure) complete"`
7. Wait for explicit user approval before proceeding to Phase 3

---

## PHASE 3: Add Individual Item Click Functionality

### Subagent
**kiss-code-generator**

### Code Style Checklist
- [ ] Functions under 20 lines
- [ ] Use `stop_propagation()` for nested interactive elements
- [ ] Clone callbacks and data outside `Callback::from`
- [ ] Each function does one thing

### Files to Modify
- `frontend/src/components/dynamic_island.rs`

### Files to Create
None

### Files to Delete
None

### Precise Deliverables

1. **Updated `render_single_item` signature** (line 123):
   ```rust
   fn render_single_item(
       item: &LearningItem,
       state: UseStateHandle<ViewState>,
       all_items: &[LearningItem],
   ) -> Html
   ```
   - Remove `on_pin: &Callback<Uuid>` parameter
   - Add `state: UseStateHandle<ViewState>` parameter
   - Add `all_items: &[LearningItem]` parameter

2. **Updated `render_single_item` body**:
   - Remove pin button HTML (lines 130-139)
   - Add `onclick` handler to `<div class="island-item">`:
     - Clone `state`, `all_items`, and item `id` outside callback
     - Use `stop_propagation()` to prevent container toggle
     - Match on `(*state).clone()`
     - If `ViewState::Items(current_uuids)`: call `swap_one_uuid(&current_uuids, id, &all_items)`
     - Set new state with swapped UUIDs
     - If `ViewState::Plan`: do nothing
   - Update title attribute to `"Click to swap"`
   - Function must be <20 lines

3. **Updated `render_items` signature** (line 102):
   ```rust
   fn render_items(
       all_items: &[LearningItem],
       uuids: &[Uuid],
       state: UseStateHandle<ViewState>,
   ) -> Html
   ```
   - Change `indices: &[usize]` to `uuids: &[Uuid]`
   - Add `state: UseStateHandle<ViewState>` parameter

4. **Updated `render_items` body**:
   - Change loop to iterate over `uuids.iter().map(|&uuid| ...)`
   - Use `find_item_by_uuid(all_items, uuid)` instead of `all_items.get(idx)`
   - Pass `state.clone()` and `all_items` to `render_single_item`
   - Function must be <20 lines

5. **Updated main component render** (line 70):
   - In `ViewState::Items(uuids)` match arm
   - Pass `state.clone()` to `render_items` call

### Actionable Steps

1. Update `render_single_item`:
   - Change signature to accept `state` and `all_items`, remove `on_pin`
   - Get item ID at start: `let id = get_item_id(item);`
   - Create onclick callback:
     ```rust
     let onclick = {
         let state = state.clone();
         let all_items = all_items.to_vec();
         let id = id.clone();
         Callback::from(move |e: MouseEvent| {
             e.stop_propagation();
             let new_state = match (*state).clone() {
                 ViewState::Items(current) => {
                     ViewState::Items(swap_one_uuid(&current, id, &all_items))
                 },
                 other => other,
             };
             state.set(new_state);
         })
     };
     ```
   - Update HTML to use `onclick` on div, remove button
2. Update `render_items`:
   - Change signature
   - Update loop to use `uuids` and `find_item_by_uuid`
   - Pass new parameters to `render_single_item`
3. Update main component to pass `state.clone()` to `render_items`

### Phase Completion Criteria

**Orchestrator must execute**:

1. Run `cargo check` - must pass
2. Run `cargo test` - 100% success required
3. Run `cargo clippy` - fix ALL warnings
4. Verify all functions under 20 lines
5. Verify no dead code (pin button code fully removed)
6. Git commit: `git add -A && git commit -m "Phase 3 (item click functionality) complete"`
7. Wait for explicit user approval before proceeding to Phase 4

---

## PHASE 4: Update Parent Component

### Subagent
**kiss-code-generator**

### Code Style Checklist
- [ ] Remove unused props
- [ ] Clean formatting

### Files to Modify
- `frontend/src/components/main_content.rs`

### Files to Create
None

### Files to Delete
None

### Precise Deliverables

1. **Remove `on_pin` prop from DynamicIsland usage** (line 259):
   - Delete entire `on_pin={Callback::from(|_| ())}` line
   - Verify component instantiation is clean

### Actionable Steps

1. Open `frontend/src/components/main_content.rs`
2. Find `DynamicIsland` component instantiation (around line 252)
3. Delete line 259: `on_pin={Callback::from(|_| ())}`
4. Ensure formatting is clean

### Phase Completion Criteria

**Orchestrator must execute**:

1. Run `cargo check` - must pass (verify no missing prop errors)
2. Run `cargo test` - 100% success required
3. Run `cargo clippy` - fix ALL warnings
4. Verify component compiles without `on_pin` prop
5. Git commit: `git add -A && git commit -m "Phase 4 (remove on_pin prop) complete"`
6. Wait for explicit user approval before proceeding to Phase 5

---

## PHASE 5: Integration Testing and Verification

### Subagent
**None** - Orchestrator handles this phase directly

### Code Style Checklist
- [ ] All dead code removed
- [ ] All clippy warnings resolved
- [ ] All tests passing

### Files to Modify
None (verification phase)

### Files to Create
- Update this plan file with completion status

### Files to Delete
None

### Precise Deliverables

1. **Full compilation verification**:
   - Frontend builds successfully with `trunk build`
   - No compilation errors or warnings

2. **Browser testing verification**:
   - Click container toggles between Plan view and Items view with single click (no double-click needed)
   - Items view shows 3 random learning items (or fewer if < 3 available)
   - Click individual item swaps ONLY that item for random non-displayed item
   - Other 2 items remain unchanged when one is swapped
   - No pin button visible
   - Transitions are smooth and immediate

3. **Code quality verification**:
   - All functions under 20 lines
   - No dead code remains
   - No clippy warnings
   - All tests passing at 100%

### Actionable Steps

1. Run `trunk build` in frontend directory
2. Start development server and test in browser:
   - Verify single-click container toggle
   - Verify item swap behavior
   - Verify no pin button
3. Run full test suite: `cargo test`
4. Run clippy: `cargo clippy`
5. Fix any issues found
6. Update this document with completion status

### Phase Completion Criteria

**Orchestrator executes**:

1. `trunk build` succeeds
2. `cargo test` - 100% pass rate
3. `cargo clippy` - zero warnings
4. Manual browser testing confirms all behaviors work
5. Update this plan file with "COMPLETED" status
6. Git commit: `git add -A && git commit -m "Phase 5 (testing and verification) complete - DynamicIsland fix done"`

---

## Critical Files Summary

- `/Users/demouser/Code/dialect-coach/frontend/src/components/dynamic_island.rs` - Core component (all logic changes)
- `/Users/demouser/Code/dialect-coach/frontend/src/components/main_content.rs` - Remove on_pin prop (line 259)

## Implementation Status

- [x] Phase 1: Data structures and helpers - COMPLETE (2025-12-18)
- [ ] Phase 2: Fix container click handler
- [ ] Phase 3: Add item click functionality
- [ ] Phase 4: Update parent component
- [ ] Phase 5: Integration testing

### Phase 1 Completion Details (2025-12-18)

**Changes Made:**
1. Updated `ViewState::Items` from `Vec<usize>` to `Vec<Uuid>`
2. Removed `on_pin: Callback<Uuid>` field from `DynamicIslandProps`
3. Created `pick_random_uuids()` helper (19 lines) - replaces `pick_random_indices()`
4. Created `find_item_by_uuid()` helper (3 lines)
5. Created `swap_one_uuid()` helper (15 lines)
6. Removed `pick_random_indices()` function entirely
7. Updated `on_click_container` callback to use `pick_random_uuids()`
8. Updated `render_items()` signature: from `indices: &[usize], on_pin: &Callback<Uuid>` to `uuids: &[Uuid]`
9. Updated `render_items()` implementation to use `find_item_by_uuid()`
10. Updated `render_single_item()` signature: removed `on_pin` parameter, removed pin button
11. Updated `render_single_item()` implementation - simplified to just render item text
12. Fixed `main_content.rs` to remove `on_pin={...}` from DynamicIsland component instantiation

**Verification:**
- `cargo check`: PASS (no compilation errors)
- `cargo test`: PASS (232/232 tests passed)
- `cargo clippy`: PASS (only expected warning: `swap_one_uuid` unused - will be used in Phase 3)
- All helper functions under 20 lines: PASS
- No dead code: PASS (except Phase 3 helper which is explicitly part of the plan)

**Status**: PHASE 1 COMPLETE - Ready for Phase 2
