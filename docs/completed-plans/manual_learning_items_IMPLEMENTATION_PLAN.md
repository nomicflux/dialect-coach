# Manual Learning Items - Full Implementation Plan (PSP)

## Overview

This plan implements a manual learning item entry form in the learning panel, allowing users to create learning items of any type (Mistake, Explanation, Translation, Exploration) with proper field validation and state management.

## Codebase Architecture Context

**Key findings from code analysis:**

1. **Learning Panel Structure** (`frontend/src/components/learning_panel.rs`)
   - Pure function components (6 helper functions for rendering)
   - Two views: `render_collapsed_view()` and `render_expanded_view()`
   - Helper functions: `get_learning_item_id()`, `get_item_content()`, `get_tooltip()`, etc.
   - CSS classes: `learning-panel-content`, `learning-section`, `section-title`, `empty-message`

2. **Data Types** (`shared/src/models/`)
   - `Mistake`: fields = `specific_mistake`, `correction`, `mistake_category` (enum), all REQUIRED
   - `Explained`: fields = `new_phrase`, `explanation`, both REQUIRED
   - `Translated`: fields = `translated_word`, `translated_to` (REQUIRED), `context` (OPTIONAL)
   - `Exploratory`: fields = `point_to_try`, `instructions_for_use`, both REQUIRED
   - `LearningItem::new(item, dialect)` constructor creates item with score=0

3. **State Management** (`frontend/src/app/app_state.rs`)
   - `UserStateAction::AddLearningItems(Vec<Mistake>, Vec<Explained>, Vec<Translated>, Vec<Exploratory>)`
   - Uses `add_learning_items_to_vec()` helper
   - Sets dialect from `state.selected_dialect`
   - Creates items with score=0 via `LearningItem::new()`

4. **Branch/Dialect Context**
   - Active branch stored in `state.active_branch_id`
   - Branch has `dialect: Option<Dialect>`
   - Dialect set from first message in branch
   - Should disable form if `branch.dialect.is_none()`

## Field Name Discrepancies

**CRITICAL: Field names in spec don't match actual code:**

| Type | Spec Says | Code Has | Action |
|------|-----------|----------|--------|
| Mistake | `incorrect` | `specific_mistake` | Use `specific_mistake` |
| Mistake | `correct` | `correction` | Use `correction` |
| Explained | `phrase` | `new_phrase` | Use `new_phrase` |
| Translated | `translated_word` (English) | `translated_word` | Matches |
| Translated | `translated_to` (Target) | `translated_to` | Matches |
| Exploratory | `exploratory_item` | `point_to_try` | Use `point_to_try` |

## Implementation Phases

---

## Phase 1: Form State and Type Selection

**Subagent:** kiss-code-generator

**Deliverables:**
- Add form state to `LearningPanel` component
- Type selection dropdown with 5 options
- Conditional rendering based on selected type
- Hide form when panel collapsed
- Disable form when branch has no dialect

**Code Style Checklist:**
- [ ] All new functions <20 lines
- [ ] Pure helper functions for form logic
- [ ] No defensive coding - known input types
- [ ] Unit tests for new functions

**Files to modify:**
1. `frontend/src/components/learning_panel.rs`
   - Add `use_state` hooks for form state in `LearningPanel` component
   - Create `render_add_item_form()` helper function
   - Create `render_type_selector()` helper function
   - Insert form at top of `render_expanded_view()` (line ~265, before accomplishments section)

**Detailed Implementation Steps:**

1. **Add state hooks to `LearningPanel` component** (after line 304)
   ```rust
   let selected_type = use_state(|| None::<String>);
   let form_visible = use_state(|| true);
   ```

2. **Create `render_type_selector()` helper** (<20 lines)
   - Input: `selected_type: UseStateHandle<Option<String>>`
   - Returns: `Html` with `<select>` element
   - Options: "None" (default), "Mistake", "Explanation", "Translation", "Exploration"
   - On change: Update `selected_type` state
   - CSS class: `learning-item-type-selector`

3. **Create `render_add_item_form()` helper** (<20 lines)
   - Input: `selected_type: &Option<String>`, `branch_dialect: Option<Dialect>`
   - Returns: `Html`
   - If `branch_dialect.is_none()`: Return disabled message
   - Else: Return `<div>` with type selector
   - CSS class: `add-learning-item-form`

4. **Update `render_expanded_view()`** (line ~246)
   - Insert after header (line ~254), before accomplishments section
   - Add: `{render_add_item_form(&selected_type, props.active_branch_dialect)}`
   - Pass `active_branch_dialect: Option<Dialect>` via props

5. **Update `LearningPanelProps`** (line ~14)
   - Add field: `pub active_branch_dialect: Option<Dialect>`

**Tests to add:**
- `test_render_type_selector_default_none()`
- `test_render_type_selector_shows_all_types()`
- `test_form_disabled_when_no_dialect()`
- `test_form_visible_when_dialect_present()`

**Phase completion checklist:**
- [ ] Run `cargo test` - 100% pass required
- [ ] Run `cargo clippy` - ALL warnings fixed
- [ ] Remove ALL dead code
- [ ] Update `docs/current-plans/manual_learning_items_IMPLEMENTATION_STATUS.md` with progress
- [ ] Git add and commit: `git commit -m "Phase 1 (form state and type selection) complete"`
- [ ] STOP and wait for explicit approval before Phase 2

---

## Phase 2: Field Rendering for Each Type

**Subagent:** modular-builder

**Deliverables:**
- Render correct fields based on selected type
- Field state management (one state hook per field)
- Required/optional field indicators
- MistakeCategory dropdown for Mistake type

**Code Style Checklist:**
- [ ] All new functions <20 lines
- [ ] One helper function per learning item type
- [ ] Pure functions for field extraction
- [ ] Unit tests for all field rendering functions

**Files to modify:**
1. `frontend/src/components/learning_panel.rs`
   - Add state hooks for all fields
   - Create `render_mistake_fields()` helper
   - Create `render_explanation_fields()` helper
   - Create `render_translation_fields()` helper
   - Create `render_exploration_fields()` helper
   - Update `render_add_item_form()` to conditionally render fields

**Detailed Implementation Steps:**

1. **Add field state hooks** (in `LearningPanel` component)
   ```rust
   // Mistake fields
   let specific_mistake = use_state(String::new);
   let correction = use_state(String::new);
   let mistake_category = use_state(|| None::<MistakeCategory>);

   // Explained fields
   let new_phrase = use_state(String::new);
   let explanation = use_state(String::new);

   // Translated fields
   let translated_word = use_state(String::new);
   let translated_to = use_state(String::new);
   let context = use_state(String::new);

   // Exploratory fields
   let point_to_try = use_state(String::new);
   let instructions_for_use = use_state(String::new);
   ```

2. **Create `render_mistake_fields()` helper** (<20 lines)
   - Inputs: State handles for `specific_mistake`, `correction`, `mistake_category`
   - Returns: `Html` with 4 inputs:
     - Text input: "Incorrect" (specific_mistake) - required
     - Text input: "Correct" (correction) - required
     - Dropdown: Category - required (5 options)
     - Note: Explanation removed (not in actual Mistake struct)
   - CSS class: `mistake-fields`

3. **Create `render_explanation_fields()` helper** (<20 lines)
   - Inputs: State handles for `new_phrase`, `explanation`
   - Returns: `Html` with 2 inputs:
     - Text input: "Phrase" (new_phrase) - required
     - Text input: "Explanation" (explanation) - required
   - CSS class: `explanation-fields`

4. **Create `render_translation_fields()` helper** (<20 lines)
   - Inputs: State handles for `translated_word`, `translated_to`, `context`
   - Returns: `Html` with 3 inputs:
     - Text input: "English" (translated_word) - required
     - Text input: "Translation" (translated_to) - required
     - Text input: "Context" (context) - optional
   - CSS class: `translation-fields`

5. **Create `render_exploration_fields()` helper** (<20 lines)
   - Inputs: State handles for `point_to_try`, `instructions_for_use`
   - Returns: `Html` with 2 inputs:
     - Text input: "Item" (point_to_try) - required
     - Text input: "Instructions" (instructions_for_use) - required
   - CSS class: `exploration-fields`

6. **Create `render_mistake_category_dropdown()` helper** (<15 lines)
   - Input: State handle for `mistake_category`
   - Returns: `Html` with `<select>` for MistakeCategory
   - Options: SpellingError, VocabularyError, GrammarError, DialectUsageError, Other
   - Each needs a context string - use placeholder "user-provided context"

7. **Update `render_add_item_form()`**
   - Add match on `selected_type`
   - Render appropriate field helper based on type
   - Pass all state handles as parameters

**Tests to add:**
- `test_render_mistake_fields_all_required()`
- `test_render_explanation_fields_all_required()`
- `test_render_translation_fields_context_optional()`
- `test_render_exploration_fields_all_required()`
- `test_mistake_category_dropdown_five_options()`

**Phase completion checklist:**
- [ ] Run `cargo test` - 100% pass required
- [ ] Run `cargo clippy` - ALL warnings fixed
- [ ] Remove ALL dead code
- [ ] Update `docs/current-plans/manual_learning_items_IMPLEMENTATION_STATUS.md` with progress
- [ ] Git add and commit: `git commit -m "Phase 2 (field rendering for each type) complete"`
- [ ] STOP and wait for explicit approval before Phase 3

---

## Phase 3: Validation and Save Logic

**Subagent:** kiss-code-generator

**Deliverables:**
- Field validation (trim, non-empty check)
- Save button enable/disable based on validation
- Create LearningItem from form data
- Dispatch AddLearningItems action
- Clear form after save

**Code Style Checklist:**
- [ ] All new functions <20 lines
- [ ] Pure validation functions
- [ ] Pure construction functions
- [ ] Unit tests for validation logic
- [ ] Unit tests for construction logic

**Files to modify:**
1. `frontend/src/components/learning_panel.rs`
   - Create `is_valid_mistake()` helper
   - Create `is_valid_explanation()` helper
   - Create `is_valid_translation()` helper
   - Create `is_valid_exploration()` helper
   - Create `create_mistake_from_form()` helper
   - Create `create_explanation_from_form()` helper
   - Create `create_translation_from_form()` helper
   - Create `create_exploration_from_form()` helper
   - Add save button to form
   - Add save click handler

**Detailed Implementation Steps:**

1. **Create validation helpers** (4 pure functions, each <10 lines)
   ```rust
   fn is_valid_mistake(specific_mistake: &str, correction: &str, category: Option<MistakeCategory>) -> bool
   fn is_valid_explanation(new_phrase: &str, explanation: &str) -> bool
   fn is_valid_translation(translated_word: &str, translated_to: &str) -> bool
   fn is_valid_exploration(point_to_try: &str, instructions_for_use: &str) -> bool
   ```
   - Trim all strings
   - Check non-empty after trim
   - For Mistake: Also check category is Some

2. **Create construction helpers** (4 pure functions, each <15 lines)
   ```rust
   fn create_mistake_from_form(specific_mistake: &str, correction: &str, category: MistakeCategory) -> Mistake
   fn create_explanation_from_form(new_phrase: &str, explanation: &str) -> Explained
   fn create_translation_from_form(word: &str, translation: &str, ctx: &str) -> Translated
   fn create_exploration_from_form(point: &str, instructions: &str) -> Exploratory
   ```
   - Call `.trim()` on all inputs
   - Call `Mistake::new()`, `Explained::new()`, etc.
   - For Translated: Pass `Some(ctx)` if non-empty, else `None`

3. **Create `clear_form_fields()` helper** (<15 lines)
   - Takes all state handles
   - Resets all to default values
   - Resets `selected_type` to None

4. **Create `handle_save_click()` callback** (<20 lines)
   - Check validation based on `selected_type`
   - Create appropriate struct using construction helper
   - Create `Vec` with single item
   - Dispatch `UserStateAction::AddLearningItems` with appropriate vec populated
   - Call `clear_form_fields()`

5. **Add save button to form**
   - Button text: "Save"
   - Disabled when: validation fails for current type OR no type selected
   - On click: Call `handle_save_click`
   - CSS class: `save-learning-item-button`

6. **Add cancel button to form**
   - Button text: "Cancel"
   - Always enabled
   - On click: Call `clear_form_fields()`
   - CSS class: `cancel-learning-item-button`

7. **Update `LearningPanelProps`** (if needed)
   - Add: `pub on_add_learning_items: Callback<(Vec<Mistake>, Vec<Explained>, Vec<Translated>, Vec<Exploratory>)>`

**Tests to add:**
- `test_is_valid_mistake_all_fields_required()`
- `test_is_valid_mistake_trims_whitespace()`
- `test_is_valid_explanation_both_required()`
- `test_is_valid_translation_context_optional()`
- `test_create_mistake_from_form_trims_fields()`
- `test_create_translation_from_form_empty_context_none()`
- `test_clear_form_fields_resets_all_state()`

**Phase completion checklist:**
- [ ] Run `cargo test` - 100% pass required
- [ ] Run `cargo clippy` - ALL warnings fixed
- [ ] Remove ALL dead code
- [ ] Update `docs/current-plans/manual_learning_items_IMPLEMENTATION_STATUS.md` with progress
- [ ] Git add and commit: `git commit -m "Phase 3 (validation and save logic) complete"`
- [ ] STOP and wait for explicit approval before Phase 4

---

## Phase 4: Integration and Polish

**Subagent:** modular-builder

**Deliverables:**
- Wire up form to parent component callbacks
- Add CSS styling for form elements
- Visual feedback on save (item appears in list)
- Error handling for edge cases
- Full end-to-end manual testing

**Code Style Checklist:**
- [ ] All functions <20 lines
- [ ] No dead code
- [ ] All clippy warnings resolved
- [ ] Integration tests pass

**Files to modify:**
1. `frontend/src/components/learning_panel.rs`
   - Final integration with parent callbacks
   - Add error states if needed
2. `frontend/styles/components/learning_panel.css` (if exists)
   - CSS for form elements
3. Parent component calling `LearningPanel`
   - Pass `active_branch_dialect` prop
   - Pass `on_add_learning_items` callback

**Detailed Implementation Steps:**

1. **Wire up parent component** (likely `frontend/src/app/mod.rs` or similar)
   - Get current branch dialect from user state
   - Pass as prop: `active_branch_dialect={active_branch.dialect}`
   - Create callback that dispatches `UserStateAction::AddLearningItems`
   - Pass callback: `on_add_learning_items={on_add_items}`

2. **Add CSS styling** (create helper function to generate inline styles if no CSS file)
   - Form container: padding, border, margin
   - Input fields: consistent sizing, padding
   - Buttons: match existing button styles
   - Required indicators: asterisk style
   - Disabled state: gray out form

3. **Add visual feedback**
   - After save, item should appear in "Still Learning" section
   - Form should clear and reset
   - No explicit success message needed (item appearing is feedback)

4. **Handle edge cases**
   - Empty strings after trim → validation fails
   - No type selected → save button disabled
   - Rapid clicking → prevent duplicate submissions
   - No dialect → entire form disabled with message

5. **Manual testing checklist**
   - [ ] Create Mistake with all fields
   - [ ] Create Explanation with both fields
   - [ ] Create Translation with context
   - [ ] Create Translation without context
   - [ ] Create Exploration with both fields
   - [ ] Verify all items appear in "Still Learning"
   - [ ] Verify items have score=0
   - [ ] Verify items have correct dialect
   - [ ] Test cancel button clears form
   - [ ] Test validation prevents empty fields
   - [ ] Test form disabled when no dialect
   - [ ] Test form hidden when panel collapsed

**Tests to add:**
- Integration test: Full flow from form to state
- Edge case test: Empty strings after trim
- Edge case test: No branch dialect
- UI test: Cancel clears all fields

**Phase completion checklist:**
- [ ] Run `cargo test` - 100% pass required (all 315+ tests)
- [ ] Run `cargo clippy` - ALL warnings fixed
- [ ] Remove ALL dead code
- [ ] Manual testing checklist 100% complete
- [ ] Update `docs/current-plans/manual_learning_items_IMPLEMENTATION_STATUS.md` - mark COMPLETE
- [ ] Git add and commit: `git commit -m "Phase 4 (integration and polish) complete"`
- [ ] STOP - Feature complete, await user review

---

## Success Criteria

All items from specification must be satisfied:

- [x] User can select learning item type from dropdown
- [x] Correct fields appear based on selected type
- [x] Required fields are marked and validated
- [x] Optional fields are clearly optional
- [x] Save button disabled until form valid
- [x] Item saved with score=0 and current dialect
- [x] Item appears immediately in learning panel
- [x] Form clears after successful save
- [x] Cancel button resets form
- [x] Disabled when branch has no dialect
- [x] All 315+ tests pass
- [x] Clippy clean

## Architecture Notes

**Modular Design Principles:**
- Each learning item type has its own render function (<20 lines)
- Validation functions are pure (input strings → bool)
- Construction functions are pure (input strings → struct)
- No defensive coding - we know the types
- Form state is local to LearningPanel component
- Dispatching to state is the only side effect

**Data Flow:**
```
User Input → Form State Hooks → Validation Functions
                                      ↓
                                 Construction Functions
                                      ↓
                              LearningItem::new()
                                      ↓
                        UserStateAction::AddLearningItems
                                      ↓
                         add_learning_items_to_vec()
                                      ↓
                            state.learning_items
```

**Testing Strategy:**
- 60% unit tests: Validation, construction, helper functions
- 30% integration tests: Form → state flow
- 10% manual UI testing: Visual verification

## Risk Mitigation

**Risk:** Field name mismatches between spec and code
**Mitigation:** Use exact field names from `shared/src/models/agent.rs` (documented in this plan)

**Risk:** MistakeCategory requires context string
**Mitigation:** Use placeholder "user-provided context" - can enhance later

**Risk:** Form complexity in single component
**Mitigation:** Break into 8+ helper functions, each <20 lines

**Risk:** State management with many fields
**Mitigation:** Use separate state hook per field, clear naming

## Open Questions Resolved

1. **Default score for manual items?** → 0 (consistent with spec)
2. **Field names discrepancy?** → Use code names (specific_mistake, not incorrect)
3. **MistakeCategory context?** → Use placeholder "user-provided context"
4. **Where to insert form?** → Top of expanded view, before accomplishments

## Post-Implementation

After all phases complete:
- Document any deviations from this plan
- Update CLAUDE.md if new patterns emerge
- Consider refactoring if any functions exceed 20 lines
- Consider adding keyboard shortcuts (future enhancement)
- Consider field autocomplete (future enhancement)
