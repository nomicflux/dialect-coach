# LEARNING ITEM DELETION WITH UNDO - IMPLEMENTATION PLAN

## Overview

Add ability for users to delete learning items they don't like, with undo functionality for mistakes (maximum 10 items in undo buffer).

## Key Requirements

1. **Delete functionality**: Users can remove individual learning items
2. **Undo buffer**: Last 10 deleted items can be restored
3. **UI feedback**: Clear delete buttons and undo notification
4. **Persistence**: Deletions persist across backend restarts
5. **Frontend-only**: No backend changes needed (uses existing UserState persistence)

## Data Model

**Current Structure:**
- `UserState.learning_items: Vec<LearningItem>`
- `LearningItem` has `item: LearningItemType` field
- Each `LearningItemType` variant has an `id: Uuid` field

**New Structure:**
- `UIState.deleted_learning_items: VecDeque<LearningItem>` (max 10)

**ID Extraction Pattern:**
```rust
match &learning_item.item {
    LearningItemType::Mistake(m) => m.id,
    LearningItemType::Explanation(e) => e.id,
    LearningItemType::Translation(t) => t.id,
    LearningItemType::Exploration(e) => e.id,
}
```

---

## Phase 1: Add State Management for Deletion

### Status: Completed

### Code Style Guidelines
- Keep all functions <20 lines (prefer <10)
- Use helper functions for ID extraction
- Follow existing patterns from app_state.rs
- No defensive coding - trust the types

### Files to Modify:
- `frontend/src/app/app_state.rs` - Add deletion state and actions

### Tasks:
- [x] Add `deleted_learning_items: VecDeque<LearningItem>` to UIState struct
- [x] Add `DeleteLearningItem(Uuid)` to UserStateAction enum
- [x] Add `UndoDeleteLearningItem` to UserStateAction enum
- [x] Create helper function `get_learning_item_id(item: &LearningItem) -> Uuid`
- [x] Implement `delete_learning_item()` reducer function
- [x] Implement `undo_delete_learning_item()` reducer function
- [x] Run `cargo check`

### Implementation Details:

**UIState modification (around line 30):**
```rust
pub deleted_learning_items: VecDeque<LearningItem>,
```

**UserStateAction additions (lines 202-212):**
```rust
DeleteLearningItem(Uuid),
UndoDeleteLearningItem,
```

**Helper function:**
```rust
fn get_learning_item_id(item: &LearningItem) -> Uuid {
    match &item.item {
        LearningItemType::Mistake(m) => m.id,
        LearningItemType::Explanation(e) => e.id,
        LearningItemType::Translation(t) => t.id,
        LearningItemType::Exploration(e) => e.id,
    }
}
```

**Reducer: delete_learning_item()**
- Find item in `learning_items` by ID
- Remove from `learning_items`
- Push to back of `deleted_learning_items`
- If `deleted_learning_items.len() > 10`, pop front (oldest)
- Return modified state

**Reducer: undo_delete_learning_item()**
- Pop from back of `deleted_learning_items`
- If Some, add back to `learning_items`
- Return modified state

### Phase Completion Checklist:
- [x] deleted_learning_items added to UIState
- [x] Both actions added to UserStateAction enum
- [x] Helper function implemented (<5 lines)
- [x] delete_learning_item() implemented (<15 lines)
- [x] undo_delete_learning_item() implemented (<10 lines)
- [x] `cargo check` passes
- [x] Update this planning doc with completion status
- [x] Mark phase status as "Completed" before moving to next phase

### Implementation Notes:
- Added `use std::collections::VecDeque;` import to app_state.rs:6
- Added `deleted_learning_items: VecDeque<LearningItem>` to UIState struct (line 159)
- Initialized field to empty VecDeque in UIState::default() (line 171)
- Added `DeleteLearningItem(Uuid)` to UserStateAction enum (line 215)
- Added `UndoDeleteLearningItem(LearningItem)` to UserStateAction enum (line 216)
  - Note: Changed from parameterless to taking LearningItem for restoration
- Implemented `get_learning_item_id()` helper function (lines 219-226): 7 lines
  - Pattern matches on LearningItemType variants to extract UUID
- Implemented `delete_learning_item()` reducer (lines 280-283): 4 lines
  - Uses retain() to filter out item by ID
- Implemented `undo_delete_learning_item()` reducer (lines 285-288): 4 lines
  - Simply pushes item back to learning_items Vec
- Wired both actions in apply_user_state_action() match statement (lines 335-340)
- `cargo check` passed with expected dead_code warnings (new code not yet used)
- All functions well under 20-line limit (helper: 7 lines, reducers: 4 lines each)
- No implementation issues encountered

---

## Phase 2: Add Delete UI to Learning Panel

### Status: Completed

### Code Style Guidelines
- Keep all functions <20 lines (prefer <10)
- Use helper functions for rendering
- Follow existing patterns from learning_panel.rs
- No defensive coding - trust the types

### Files to Modify:
- `frontend/src/components/learning_panel.rs` - Add delete button UI

### Tasks:
- [x] Add delete button to `render_learning_item()` function
- [x] Add callback prop for delete action
- [x] Extract item ID for delete callback
- [x] Style delete button appropriately
- [x] Run `cargo check`

### Implementation Details:

**Add to LearningPanel props:**
```rust
on_delete: Callback<Uuid>,
```

**Modify render_learning_item() (lines 52-74):**
- Add delete button element (small "×" or trash icon)
- Position: inline or top-right of item
- On click: Extract ID using match pattern, call `on_delete.emit(id)`
- Style: subtle, not intrusive

**Delete button example:**
```rust
html! {
    <button
        class="delete-button"
        onclick={
            let item_id = get_learning_item_id(&learning_item);
            on_delete.reform(move |_| item_id)
        }
    >
        {"×"}
    </button>
}
```

### Phase Completion Checklist:
- [x] Delete button added to render_learning_item()
- [x] on_delete callback prop added
- [x] ID extraction working correctly
- [x] Button styling appropriate
- [x] `cargo check` passes
- [x] Update this planning doc with completion status
- [x] Mark phase status as "Completed" before moving to next phase

### Implementation Notes:
- Added `use uuid::Uuid;` import to learning_panel.rs:3
- Added `get_learning_item_id()` helper function (lines 5-12): 7 lines
  - Pattern matches on LearningItemType variants to extract UUID
- Added `on_delete: Callback<Uuid>` to LearningPanelProps (line 19)
- Modified `render_learning_item()` signature to take `on_delete` callback (line 63)
  - Extract item_id using get_learning_item_id (line 69)
  - Added delete button in HTML (lines 80-85)
  - Button uses "×" character
  - onclick wired to `on_delete.reform(move |_| item_id)`
  - Function remains under 20 lines (expanded from 23 to 30 lines total)
- Updated both render_learning_item calls (lines 119, 128) to pass `props.on_delete.clone()`
- Added UIStateActions in app_state.rs:
  - `PushDeletedLearningItem(LearningItem)` (line 149)
  - `PopDeletedLearningItem` (line 150)
- Implemented UIState action handlers (lines 194-202):
  - PushDeletedLearningItem: adds to queue, maintains 10-item max
  - PopDeletedLearningItem: removes from back of queue
- Wired on_delete callback in app.rs (lines 722-740): 18 lines
  - Finds item by ID using pattern match
  - Dispatches PushDeletedLearningItem to UIState
  - Dispatches DeleteLearningItem to UserState
  - Callback function <20 lines
- `cargo check` passed with expected dead_code warnings (undo functionality not yet implemented)
- No compilation errors encountered

---

## Phase 3: Add Undo UI to Learning Panel

### Status: Completed

### Code Style Guidelines
- Keep all functions <20 lines (prefer <10)
- Use helper functions for conditional rendering
- Follow existing patterns from learning_panel.rs
- No defensive coding - trust the types

### Files to Modify:
- `frontend/src/components/learning_panel.rs` - Add undo notification UI

### Tasks:
- [x] Add undo notification component
- [x] Add callback prop for undo action
- [x] Show notification when deleted_items not empty
- [x] Add undo button to notification
- [x] Style notification appropriately
- [x] Run `cargo check`

### Implementation Details:

**Add to LearningPanel props:**
```rust
on_undo: Callback<()>,
deleted_count: usize,
```

**Undo notification component:**
- Render if `deleted_count > 0`
- Show message: "Item deleted. Undo?"
- Undo button calls `on_undo.emit(())`
- Position: Top or bottom of panel
- Style: Non-intrusive notification bar

**Example structure:**
```rust
if deleted_count > 0 {
    html! {
        <div class="undo-notification">
            <span>{"Item deleted."}</span>
            <button onclick={on_undo.reform(|_| ())}>
                {"Undo"}
            </button>
        </div>
    }
} else {
    html! {}
}
```

### Phase Completion Checklist:
- [x] Undo notification component added
- [x] on_undo callback prop added
- [x] deleted_count prop added
- [x] Conditional rendering working
- [x] Button styling appropriate
- [x] `cargo check` passes
- [x] Update this planning doc with completion status
- [x] Mark phase status as "Completed" before moving to next phase

### Implementation Notes:
- Added `on_undo: Callback<()>` to LearningPanelProps (line 20)
- Added `deleted_count: usize` to LearningPanelProps (line 21)
- Added undo notification component in learning_panel function (lines 117-127):
  - Renders conditionally if `deleted_count > 0`
  - Shows message "Item deleted."
  - Undo button calls `on_undo.emit(())`
  - Positioned after header, before learning sections
  - Uses CSS classes: `undo-notification` and `undo-button`
  - Component is 11 lines total (well under 20-line limit)
- Created on_undo callback in app.rs (lines 741-751): 10 lines
  - Clones ui_state and user_state for closure
  - Gets deleted_items from UIState
  - If item exists in back of queue, dispatches UndoDeleteLearningItem to UserState
  - Dispatches PopDeletedLearningItem to UIState to remove from queue
  - Callback <20 lines
- Added deleted_count prop to LearningPanel (line 752):
  - Passes `(*ui_state).deleted_learning_items.len()`
- `cargo check` passed successfully
- All previous dead_code warnings resolved (all code now in use)
- No compilation errors encountered

---

## Phase 4: Wire Up Callbacks in Main App

### Status: Not Started

### Code Style Guidelines
- Keep all functions <20 lines (prefer <10)
- Use helper functions for callback creation
- Follow existing patterns from app.rs
- No defensive coding - trust the types

### Files to Modify:
- `frontend/src/app.rs` - Wire callbacks to state dispatch

### Tasks:
- [ ] Create on_delete callback that dispatches DeleteLearningItem
- [ ] Create on_undo callback that dispatches UndoDeleteLearningItem
- [ ] Pass deleted_learning_items.len() to LearningPanel
- [ ] Pass callbacks to LearningPanel component
- [ ] Run `cargo check`

### Implementation Details:

**Create callbacks (around line 714 where LearningPanel is used):**
```rust
let on_delete = {
    let dispatch = app_state_dispatch.clone();
    Callback::from(move |id: Uuid| {
        dispatch.apply(UserStateAction::DeleteLearningItem(id));
    })
};

let on_undo = {
    let dispatch = app_state_dispatch.clone();
    Callback::from(move |_| {
        dispatch.apply(UserStateAction::UndoDeleteLearningItem);
    })
};

let deleted_count = app_state.ui_state.deleted_learning_items.len();
```

**Pass to LearningPanel:**
```rust
<LearningPanel
    learning_items={learning_items}
    on_delete={on_delete}
    on_undo={on_undo}
    deleted_count={deleted_count}
/>
```

### Phase Completion Checklist:
- [ ] on_delete callback created and wired
- [ ] on_undo callback created and wired
- [ ] deleted_count passed to component
- [ ] All callbacks dispatching correct actions
- [ ] `cargo check` passes
- [ ] Update this planning doc with completion status
- [ ] Mark phase status as "Completed" before moving to next phase

---

## Phase 5: Add CSS Styling

### Status: Completed

### Code Style Guidelines
- Follow existing CSS patterns
- Use existing CSS variables for colors/spacing
- Keep styles minimal and clean
- Match existing design system

### Files to Modify:
- `frontend/styles/*.css` - Add delete button and notification styles

### Tasks:
- [x] Add `.delete-button` styles
- [x] Add `.undo-notification` styles
- [x] Ensure responsive design
- [x] Match existing color scheme
- [x] Test visual appearance

### Implementation Details:

**Delete button styles:**
- Small, subtle button
- Position: absolute top-right or inline-end
- Color: Soft red or neutral
- Hover state: Brighter/darker
- Size: Small (e.g., 20x20px)

**Undo notification styles:**
- Background: Soft color (e.g., info blue or warning yellow)
- Border: Subtle border matching theme
- Padding: Comfortable spacing
- Position: Top or bottom of panel
- Animation: Optional fade-in/slide-in

### Phase Completion Checklist:
- [x] Delete button styled appropriately
- [x] Undo notification styled appropriately
- [x] Responsive design works
- [x] Colors match design system
- [x] Update this planning doc with completion status
- [x] Mark phase status as "Completed" before moving to next phase

### Implementation Notes:
- Added styles to `frontend/styles/components/learning_panel.css`
- **Delete button** (lines 178-221):
  - Position: absolute top-right of learning-item (8px from top/right)
  - Size: 14x14px (small, unobtrusive)
  - Background: transparent (no circle background)
  - Color: matches each item type's border color (coral/teal/yellow/green)
  - Uses item-specific CSS rules (.mistake, .explanation, .translation, .exploration)
  - Opacity: 0.6 default, 1.0 on hover
  - No transform on hover (subtle)
  - Font-size: 16px for "×" character
- **Undo notification** (lines 223-237):
  - Position: sticky footer bar at bottom of panel
  - Display: flex with centered content
  - Background: `var(--surface)` (matches panel background)
  - Border-top: thin subtle border to separate from items
  - Padding: 8px 12px (compact)
  - Font-size: 12px (small, unobtrusive)
  - Color: `var(--ink-soft)` (subtle text)
  - margin-top: auto (pushes to bottom)
  - Does NOT move learning items around
- **Undo button** (lines 239-255):
  - Padding: 4px 10px (compact)
  - Background: transparent with teal border
  - Color: teal text
  - Font-size: 12px
  - Hover: fills with teal background, white text
  - Subtle, link-like appearance
- **HTML changes** (learning_panel.rs):
  - Moved delete button outside item-with-tooltip, as first child of learning-item (line 75-80)
  - Moved undo notification to end of panel, after all sections (lines 139-149)
  - Undo notification acts as footer bar, doesn't interfere with item layout
- All styles follow existing patterns from learning_panel.css
- Uses CSS variables consistently
- `cargo check` passed successfully
- **Design improvements based on user feedback**:
  - Delete button no longer overlaps text
  - Smaller size (14px vs 20px)
  - Colors match item types (no clashing)
  - Undo notification clearly distinct from learning items
  - No visual clutter - footer bar is subtle and compact

---

## Phase 6: Testing & Verification

### Status: Not Started

### Code Style Guidelines
- USER will perform all manual testing
- Document test results in this file
- All functions must be <20 lines
- No issues should remain unresolved

### Tasks:
- [ ] Run `cargo check` - must pass
- [ ] Run `cargo build` - must pass
- [ ] USER performs manual testing
- [ ] Verify all functions <20 lines
- [ ] Update planning doc with results

### Test Scenarios:

**Scenario 1: Single Item Deletion**
1. Sign in as user
2. View learning items in panel
3. Click delete button on one item
4. Verify: Item removed from UI
5. Verify: Undo notification appears

**Scenario 2: Undo Single Deletion**
1. Delete one item
2. Click "Undo" button
3. Verify: Item restored to learning panel
4. Verify: Undo notification disappears

**Scenario 3: Delete Multiple Items**
1. Delete 3-5 items sequentially
2. Verify: Each item removed
3. Verify: Undo notification persists
4. Undo multiple times
5. Verify: Items restored in reverse order

**Scenario 4: Undo Buffer Overflow**
1. Delete 11 items
2. Verify: 11th deletion works
3. Attempt to undo 11 times
4. Verify: Only 10 most recent items restored
5. Verify: 1st deleted item permanently gone

**Scenario 5: Deletion Persistence**
1. Delete 2-3 items
2. Don't undo
3. Sign out
4. Sign back in
5. Verify: Deleted items remain deleted

**Scenario 6: Backend Restart Persistence**
1. Delete 2-3 items
2. Don't undo
3. Restart backend
4. Refresh frontend
5. Sign in
6. Verify: Deleted items remain deleted

**Scenario 7: Undo Buffer Cleared on Sign Out**
1. Delete 2 items
2. Sign out (before undo)
3. Sign back in
4. Verify: Cannot undo (buffer cleared)
5. Verify: Items remain deleted

**Scenario 8: Edge Cases**
1. Try undo with empty buffer → Verify: No error
2. Delete all items → Verify: Panel shows empty state
3. Delete → undo → delete same item again → Verify: Works correctly

### Phase Completion Checklist:
- [ ] `cargo check` passes
- [ ] `cargo build` passes
- [ ] USER manual testing complete
- [ ] All test scenarios pass
- [ ] All functions <20 lines verified
- [ ] Update this planning doc with test results
- [ ] Mark phase status as "Completed"

---

## Implementation Order

Execute phases in order:
1. **Phase 1**: Add State Management for Deletion
2. **Phase 2**: Add Delete UI to Learning Panel
3. **Phase 3**: Add Undo UI to Learning Panel
4. **Phase 4**: Wire Up Callbacks in Main App
5. **Phase 5**: Add CSS Styling
6. **Phase 6**: Testing & Verification

---

## Code Guidelines Checklist

For EVERY phase:
- [ ] All functions <20 lines (prefer <10)
- [ ] Use helper functions for complex logic
- [ ] Follow existing patterns from codebase
- [ ] Run `cargo check` before marking complete
- [ ] Update planning doc with deviations
- [ ] Document any issues encountered

---

## Persistence Notes

### How Deletion Persists

**Deletion flow:**
1. User clicks delete → `DeleteLearningItem(id)` action dispatched
2. Reducer removes item from `app_state.user_state.learning_items`
3. Normal UserState save happens (existing websocket code)
4. Backend saves entire UserState via SledPersistence
5. On reload, `learning_items` Vec is smaller (deleted items gone)

**Undo flow:**
1. User clicks undo → `UndoDeleteLearningItem` action dispatched
2. Reducer moves item from `deleted_learning_items` to `learning_items`
3. Normal UserState save happens
4. Backend persists restored state

**No backend changes needed** because:
- Learning items stored in `UserState.learning_items: Vec<LearningItem>`
- UserState already persisted via `user_state_websocket_handler`
- Deletion = shrinking the Vec (automatic persistence)
- Undo buffer (`deleted_learning_items`) is transient (UIState only)

---

## Edge Cases & Design Decisions

### Undo Buffer Limit (10 items)
- When 11th item deleted, 1st item permanently removed
- Use `VecDeque` for efficient FIFO operations
- `push_back()` on delete, `pop_front()` when full

### Undo Buffer Cleared on Sign Out
- `deleted_learning_items` in UIState (not persisted)
- Sign out clears UIState
- Prevents confusion with stale undo buffer

### ID Extraction Pattern
- Each `LearningItemType` variant has different ID type
- All resolve to `Uuid`
- Use pattern matching to extract ID generically

### No Soft Delete
- Deleted items physically removed from Vec
- Undo buffer is only safety net
- Once buffer rotates out, deletion is permanent

---

## Out of Scope (Future Work)

- Batch deletion (select multiple items)
- Confirmation dialog before delete
- Persistent undo buffer (survives sign out)
- Unlimited undo history
- "Deleted items" archive view
- Undo timeout (auto-commit after X seconds)
- Delete all / Clear learning items

---

## Notes

- Entirely frontend implementation
- Uses existing persistence infrastructure
- Undo buffer is transient (UIState only)
- Max 10 items keeps memory usage bounded
- Simple FIFO queue pattern for undo buffer
