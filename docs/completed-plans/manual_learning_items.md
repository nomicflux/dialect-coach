# Manual Learning Items - Specification

## User Requirement

**Date: 2025-11-23**

User wants to manually add learning items in the learning panel:
- "Allow option for user to add learning items manually"
- "This will be an additional section in the learning items panel"
- "Allow them to add any sort of learning item they want"
- "Once they chosen the learning item type, expose textboxes for the fields"
- "Make required fields required, and optional fields optional"
- "Once saved, the learning item is added for the current dialect"

## Current Learning Item Types

From `shared/src/models/agent.rs`:

### 1. Mistake
```rust
pub struct Mistake {
    pub id: Uuid,
    pub incorrect: String,          // REQUIRED
    pub correct: String,             // REQUIRED
    pub mistake_category: MistakeCategory, // REQUIRED
    pub explanation: String,         // REQUIRED
}

pub enum MistakeCategory {
    Grammar,
    Vocabulary,
    Spelling,
    Dialect,
    Other,
}
```

### 2. Explained
```rust
pub struct Explained {
    pub id: Uuid,
    pub phrase: String,              // REQUIRED
    pub explanation: String,         // REQUIRED
}
```

### 3. Translated
```rust
pub struct Translated {
    pub id: Uuid,
    pub translated_word: String,     // REQUIRED - English
    pub translated_to: String,       // REQUIRED - Target language
    pub context: Option<String>,     // OPTIONAL
}
```

### 4. Exploratory
```rust
pub struct Exploratory {
    pub id: Uuid,
    pub exploratory_item: String,    // REQUIRED
    pub instructions_for_use: String, // REQUIRED
}
```

## UI Design Specification

### Location
Add to **Learning Panel** (`frontend/src/components/learning_panel.rs`):
- New section: "Add Learning Item" or "Manual Entry"
- Positioned at the top of the learning panel (above existing items)
- Only visible when panel is expanded (not in collapsed view)

### Interaction Flow

**Step 1: Item Type Selection**
```
┌─────────────────────────────────────┐
│ Add Learning Item                   │
├─────────────────────────────────────┤
│ Type: [Dropdown ▼]                  │
│   - Select type...                  │
│   - Mistake                         │
│   - Explanation                     │
│   - Translation                     │
│   - Exploration                     │
└─────────────────────────────────────┘
```

**Step 2: Form Fields (Based on Selection)**

#### If "Mistake" selected:
```
┌─────────────────────────────────────┐
│ Add Mistake                         │
├─────────────────────────────────────┤
│ Incorrect: [___________________] *  │
│ Correct:   [___________________] *  │
│ Category:  [Dropdown ▼] *           │
│   - Grammar                         │
│   - Vocabulary                      │
│   - Spelling                        │
│   - Dialect                         │
│   - Other                           │
│ Explanation: [________________] *   │
│ [Cancel] [Save]                     │
└─────────────────────────────────────┘
```

#### If "Explanation" selected:
```
┌─────────────────────────────────────┐
│ Add Explanation                     │
├─────────────────────────────────────┤
│ Phrase:      [___________________] *│
│ Explanation: [___________________] *│
│ [Cancel] [Save]                     │
└─────────────────────────────────────┘
```

#### If "Translation" selected:
```
┌─────────────────────────────────────┐
│ Add Translation                     │
├─────────────────────────────────────┤
│ English:     [___________________] *│
│ Translation: [___________________] *│
│ Context:     [___________________]  │
│ [Cancel] [Save]                     │
└─────────────────────────────────────┘
```

#### If "Exploration" selected:
```
┌─────────────────────────────────────┐
│ Add Exploration                     │
├─────────────────────────────────────┤
│ Item:         [__________________] *│
│ Instructions: [__________________] *│
│ [Cancel] [Save]                     │
└─────────────────────────────────────┘
```

**Step 3: Validation & Save**
- Required fields marked with *
- Save button disabled until all required fields filled
- On save:
  - Create new LearningItem with type and fields
  - Set initial score to 0 (needs practice)
  - Set dialect to current active branch dialect
  - Dispatch AddLearningItems action
  - Clear form and reset to type selection

### Edge Cases

**1. No active branch dialect?**
- If active branch has `dialect: None`
- Disable "Add Learning Item" section
- Show message: "Send a message to start practicing before adding items"

**2. Form validation**
- Empty required fields → Show inline error
- All fields trimmed before save
- Empty strings after trim → Invalid

**3. Cancel behavior**
- Clear all form fields
- Reset to type selection dropdown
- No confirmation needed (no data loss since not saved)

## Data Flow

```
User fills form → Click Save
  ↓
Validate fields
  ↓
Create LearningItem {
  item: LearningItemType (Mistake/Explained/etc),
  score: 0,
  dialect: active_branch.dialect.unwrap()
}
  ↓
Dispatch UserStateAction::AddLearningItems
  ↓
Add to state.learning_items
  ↓
Clear form, show success (item appears in list)
```

## Implementation Files

### Frontend
- `frontend/src/components/learning_panel.rs`
  - Add "Add Learning Item" section to render_expanded_view
  - Add form component with type selection
  - Add conditional field rendering based on type
  - Add validation logic
  - Dispatch AddLearningItems on save

### Shared
- `shared/src/models/agent.rs`
  - Already has all types defined
  - MistakeCategory already has Display trait
  - No changes needed

### State Management
- `frontend/src/app/app_state.rs`
  - AddLearningItems action already exists
  - Just need to dispatch with manually created items

## UI/UX Considerations

**Visual Design:**
- Use existing design tokens (yellow-tint for forms)
- Match existing button styles
- Form should feel integrated, not bolted on

**Accessibility:**
- Required field indicators (*)
- Proper labels for screen readers
- Keyboard navigation support
- Error messages for validation

**User Feedback:**
- Visual confirmation when item added (appears in list immediately)
- Clear button states (enabled/disabled)
- Form resets after successful save

## Success Criteria

- [ ] User can select learning item type from dropdown
- [ ] Correct fields appear based on selected type
- [ ] Required fields are marked and validated
- [ ] Optional fields are clearly optional
- [ ] Save button disabled until form valid
- [ ] Item saved with score: 0 and current dialect
- [ ] Item appears immediately in learning panel
- [ ] Form clears after successful save
- [ ] Cancel button resets form
- [ ] Disabled when branch has no dialect
- [ ] All 315+ tests pass
- [ ] Clippy clean

## Open Questions

1. **Default score for manual items?**
   - Suggestion: 0 (needs practice)
   - Or: Ask user to set initial score?
   - **Decision:** 0 is simple and consistent

2. **Allow editing existing items?**
   - Out of scope for now
   - Can add later if needed

3. **Bulk add multiple items?**
   - Out of scope
   - One at a time is sufficient

4. **Import/export learning items?**
   - Out of scope
   - Future enhancement

## Implementation Phases

**Phase 1: Add form structure to learning panel**
- Add "Add Learning Item" section to expanded view
- Add type selection dropdown
- Wire up state for selected type
- Show/hide section based on branch dialect

**Phase 2: Implement conditional field rendering**
- Add form fields for each type
- Implement required/optional field logic
- Add validation for required fields
- Enable/disable save button based on validation

**Phase 3: Wire up save functionality**
- Create LearningItem from form data
- Dispatch AddLearningItems action
- Clear form on success
- Add visual feedback

**Phase 4: Polish and testing**
- Add cancel button
- Improve validation error messages
- Test all item types
- Ensure clippy clean
- Manual UI testing

## Status

- [ ] Phase 1: Form structure
- [ ] Phase 2: Field rendering
- [ ] Phase 3: Save functionality
- [ ] Phase 4: Polish

**Current Phase:** Not started - awaiting user approval
