# Keyboard Shortcuts Implementation Status

## Overview
Add configurable keyboard shortcuts to the Yew WASM frontend for common actions.

## Requirements
1. Send messages (Shift + Enter) - in chat input
2. Toggle branch/learning goal pane (Ctrl + Left Arrow)
3. Toggle learning items pane (Ctrl + Right Arrow)
4. Toggle usage stats (Ctrl + u)
5. Toggle practice setting (Ctrl + p)
6. Toggle auto-speak (Ctrl + Shift + S)
7. Replay last message (Ctrl + s)
8. Switch dialect (Ctrl + d)
9. Switch teaching mode (Ctrl + t)
10. Switch register/formality (Ctrl + r)
11. Move focus to learning goal entry (Ctrl + g)
12. Move focus to chat box entry (Ctrl + c)

## Agreements Made
- Shortcuts must be configurable (data structure ready for future customization)
- UI for customization is not needed yet
- Shortcuts must not conflict with browser defaults

## Explicitly Rejected
- Custom UI for shortcut configuration (future work)
- Complex modifier combinations beyond current requirements

---

## Phase 1: Shortcut Configuration Data Structure
**Status**: COMPLETED
**Subagent**: kiss-code-generator

### Code Style Checklist
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No dead code
- [ ] No TODOs

### Deliverables
- Data structure to hold shortcut mappings
- Type-safe representation of keyboard shortcuts
- Default configuration values

### Files to Create
- `frontend/src/keyboard_shortcuts.rs` - Shortcut configuration module

### Implementation Details

```rust
// frontend/src/keyboard_shortcuts.rs

use std::collections::HashMap;

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum ShortcutAction {
    ToggleSidebar,
    ToggleLearningPanel,
    ToggleUsageFooter,
    TogglePracticeSettings,
    ToggleAutoSpeak,
    ReplayLastMessage,
    CycleDialect,
    CycleTeachingMode,
    CycleFormality,
    FocusGoalInput,
    FocusChatInput,
}

#[derive(Clone, PartialEq)]
pub struct KeyBinding {
    pub key: String,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub meta: bool,
}

impl KeyBinding {
    pub fn new(key: &str, ctrl: bool, shift: bool) -> Self {
        Self {
            key: key.to_string(),
            ctrl,
            shift,
            alt: false,
            meta: false,
        }
    }
}

pub fn default_shortcuts() -> HashMap<ShortcutAction, KeyBinding> {
    let mut map = HashMap::new();
    map.insert(ShortcutAction::ToggleSidebar, KeyBinding::new("ArrowLeft", true, false));
    map.insert(ShortcutAction::ToggleLearningPanel, KeyBinding::new("ArrowRight", true, false));
    map.insert(ShortcutAction::ToggleUsageFooter, KeyBinding::new("u", true, false));
    map.insert(ShortcutAction::TogglePracticeSettings, KeyBinding::new("p", true, false));
    map.insert(ShortcutAction::ToggleAutoSpeak, KeyBinding::new("s", true, true)); // Ctrl+Shift+S
    map.insert(ShortcutAction::ReplayLastMessage, KeyBinding::new("s", true, false));
    map.insert(ShortcutAction::CycleDialect, KeyBinding::new("d", true, false));
    map.insert(ShortcutAction::CycleTeachingMode, KeyBinding::new("t", true, false));
    map.insert(ShortcutAction::CycleFormality, KeyBinding::new("r", true, false));
    map.insert(ShortcutAction::FocusGoalInput, KeyBinding::new("g", true, false));
    map.insert(ShortcutAction::FocusChatInput, KeyBinding::new("c", true, false));
    map
}

pub fn matches_binding(event: &web_sys::KeyboardEvent, binding: &KeyBinding) -> bool {
    let key_matches = event.key() == binding.key
        || event.key().to_lowercase() == binding.key.to_lowercase();
    let ctrl_matches = event.ctrl_key() == binding.ctrl || event.meta_key() == binding.ctrl;
    let shift_matches = event.shift_key() == binding.shift;
    let alt_matches = event.alt_key() == binding.alt;

    key_matches && ctrl_matches && shift_matches && alt_matches
}
```

### Actions
1. Create `frontend/src/keyboard_shortcuts.rs` with above code
2. Add `pub mod keyboard_shortcuts;` to `frontend/src/lib.rs`
3. Write unit tests for `matches_binding` function

### Phase End Verification
- Run `cargo test --package dialect-coach-frontend`
- Run `cargo clippy --package dialect-coach-frontend -- -D warnings`
- No dead code allowed
- Update this document with completion status
- Git commit: "Phase 1 (Shortcut configuration data structure) complete"

---

## Phase 2: Shift+Enter for Send Message in InputBox
**Status**: PENDING
**Subagent**: kiss-code-generator

### Code Style Checklist
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No dead code
- [ ] No TODOs

### Deliverables
- InputBox handles Shift+Enter to submit
- Regular Enter creates newline (default textarea behavior)

### Files to Modify
- `frontend/src/components/input_box.rs` - Add keydown handler

### Implementation Details

Add `onkeydown` handler to textarea in `input_box.rs`:

```rust
let on_keydown = {
    let input_value = input_value.clone();
    let on_send = props.on_send.clone();
    Callback::from(move |e: KeyboardEvent| {
        if e.shift_key() && e.key() == "Enter" {
            e.prevent_default();
            let value = (*input_value).clone();
            if !value.trim().is_empty() {
                on_send.emit(value);
                input_value.set(String::new());
            }
        }
    })
};
```

Add to textarea element:
```rust
onkeydown={on_keydown}
```

### Actions
1. Add `use web_sys::KeyboardEvent;` import
2. Create `on_keydown` callback before `html!` block
3. Add `onkeydown={on_keydown}` to textarea element
4. Update placeholder text to mention Shift+Enter

### Phase End Verification
- Run `cargo test --package dialect-coach-frontend`
- Run `cargo clippy --package dialect-coach-frontend -- -D warnings`
- No dead code allowed
- Update this document with completion status
- Git commit: "Phase 2 (Shift+Enter for send message) complete"

---

## Phase 3: Global Keyboard Shortcut Handler
**Status**: PENDING
**Subagent**: modular-builder

### Code Style Checklist
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No dead code
- [ ] No TODOs

### Deliverables
- Global keyboard event listener in MainContent
- Handle all Ctrl-based shortcuts
- NodeRefs for focus management (goal input, chat input)
- All shortcuts functional

### Files to Modify
- `frontend/src/components/main_content.rs` - Add global shortcut handler
- `frontend/src/components/input_box.rs` - Expose NodeRef for textarea
- `frontend/src/components/learning_goals_panel.rs` - Expose NodeRef for input
- `frontend/src/components/branch_sidebar.rs` - Pass through NodeRef

### Implementation Details

#### main_content.rs Changes

1. Import keyboard_shortcuts module:
```rust
use crate::keyboard_shortcuts::{default_shortcuts, matches_binding, ShortcutAction};
```

2. Create NodeRefs for focus targets:
```rust
let chat_input_ref = use_node_ref();
let goal_input_ref = use_node_ref();
```

3. Replace single Ctrl+B shortcut with comprehensive handler:
```rust
{
    let ui_state = ui_state.clone();
    let app_state = app_state.clone();
    let user_state = user_state.clone();
    let chat_input_ref = chat_input_ref.clone();
    let goal_input_ref = goal_input_ref.clone();

    use_effect_with((), move |_| {
        let shortcuts = default_shortcuts();

        let listener = EventListener::new(&window(), "keydown", move |e| {
            if let Some(event) = e.dyn_ref::<KeyboardEvent>() {
                // Skip if typing in input/textarea (except for focus shortcuts)
                let target = event.target();
                let is_input = target.as_ref().and_then(|t| t.dyn_ref::<web_sys::HtmlInputElement>()).is_some()
                    || target.as_ref().and_then(|t| t.dyn_ref::<web_sys::HtmlTextAreaElement>()).is_some();

                for (action, binding) in &shortcuts {
                    if matches_binding(event, binding) {
                        // Focus shortcuts should work even in inputs
                        let should_handle = match action {
                            ShortcutAction::FocusGoalInput | ShortcutAction::FocusChatInput => true,
                            _ => !is_input,
                        };

                        if should_handle {
                            event.prevent_default();
                            handle_shortcut_action(
                                action,
                                &ui_state,
                                &app_state,
                                &user_state,
                                &chat_input_ref,
                                &goal_input_ref,
                            );
                            break;
                        }
                    }
                }
            }
        });
        move || drop(listener)
    });
}
```

4. Create handler function (pure logic extraction):
```rust
fn handle_shortcut_action(
    action: &ShortcutAction,
    ui_state: &UseReducerHandle<UIState>,
    app_state: &UseReducerHandle<AppState>,
    user_state: &UseReducerHandle<OptionalUserState>,
    chat_input_ref: &NodeRef,
    goal_input_ref: &NodeRef,
) {
    match action {
        ShortcutAction::ToggleSidebar => {
            ui_state.dispatch(UIStateAction::ToggleSidebar);
        }
        ShortcutAction::ToggleLearningPanel => {
            ui_state.dispatch(UIStateAction::ToggleLearningPanel);
        }
        ShortcutAction::ToggleUsageFooter => {
            ui_state.dispatch(UIStateAction::ToggleUsageFooter);
        }
        ShortcutAction::TogglePracticeSettings => {
            if ui_state.panel_open {
                ui_state.dispatch(UIStateAction::ClosePanel);
            } else {
                ui_state.dispatch(UIStateAction::OpenPanel);
            }
        }
        ShortcutAction::ToggleAutoSpeak => {
            on_tts_toggle(app_state.clone(), user_state.clone()).emit(());
        }
        ShortcutAction::ReplayLastMessage => {
            if let Some(state) = user_state.0.as_ref() {
                if let Some(msg) = state.get_active_branch_messages().last() {
                    on_replay_message(app_state.clone()).emit(msg.clone());
                }
            }
        }
        ShortcutAction::CycleDialect => {
            on_dialect_cycle(user_state.clone()).emit(());
        }
        ShortcutAction::CycleTeachingMode => {
            on_teaching_mode_cycle(user_state.clone()).emit(());
        }
        ShortcutAction::CycleFormality => {
            on_formality_cycle(user_state.clone()).emit(());
        }
        ShortcutAction::FocusGoalInput => {
            if let Some(input) = goal_input_ref.cast::<web_sys::HtmlInputElement>() {
                let _ = input.focus();
            }
        }
        ShortcutAction::FocusChatInput => {
            if let Some(textarea) = chat_input_ref.cast::<web_sys::HtmlTextAreaElement>() {
                let _ = textarea.focus();
            }
        }
    }
}
```

#### input_box.rs Changes

Add prop for external NodeRef:
```rust
#[derive(Properties, PartialEq)]
pub struct InputBoxProps {
    pub on_send: Callback<String>,
    #[prop_or(false)]
    pub disabled: bool,
    #[prop_or_default]
    pub external_value: Option<String>,
    #[prop_or_default]
    pub textarea_ref: Option<NodeRef>,
}
```

Use external ref if provided:
```rust
let textarea_node_ref = props.textarea_ref.clone().unwrap_or_else(use_node_ref);
```

Apply to textarea:
```rust
<textarea
    ref={textarea_node_ref}
    // ... rest of attributes
/>
```

#### learning_goals_panel.rs Changes

Add prop for external NodeRef:
```rust
#[derive(Properties, PartialEq)]
pub struct LearningGoalsPanelProps {
    pub goals: Vec<String>,
    pub on_add: Callback<String>,
    pub on_delete: Callback<usize>,
    #[prop_or_default]
    pub input_ref: Option<NodeRef>,
}
```

Use external ref if provided:
```rust
let input_ref = props.input_ref.clone().unwrap_or_else(use_node_ref);
```

#### branch_sidebar.rs Changes

Add prop to pass through NodeRef:
```rust
#[prop_or_default]
pub goal_input_ref: Option<NodeRef>,
```

Pass to LearningGoalsPanel:
```rust
<LearningGoalsPanel
    // ... other props
    input_ref={props.goal_input_ref.clone()}
/>
```

### Actions
1. Add keyboard_shortcuts module import to main_content.rs
2. Add web_sys imports for HtmlInputElement, HtmlTextAreaElement
3. Create NodeRefs in main_content component
4. Replace existing Ctrl+B handler with comprehensive handler
5. Add handle_shortcut_action function
6. Update InputBoxProps to accept optional NodeRef
7. Update LearningGoalsPanelProps to accept optional NodeRef
8. Update BranchSidebarProps to pass through NodeRef
9. Wire up all NodeRefs through component hierarchy
10. Pass NodeRefs from MainContent to child components

### Phase End Verification
- Run `cargo test --package dialect-coach-frontend`
- Run `cargo clippy --package dialect-coach-frontend -- -D warnings`
- No dead code allowed
- Test all shortcuts manually in browser
- Update this document with completion status
- Git commit: "Phase 3 (Global keyboard shortcut handler) complete"

---

## Phase 4: Documentation and Final Verification
**Status**: PENDING
**Subagent**: kiss-code-generator

### Code Style Checklist
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No dead code
- [ ] No TODOs

### Deliverables
- Help text or tooltip showing available shortcuts
- Tests for shortcut matching logic
- All integration tests pass

### Files to Modify
- `frontend/src/keyboard_shortcuts.rs` - Add helper for generating help text
- `frontend/src/components/header.rs` - Optional: show shortcuts reference

### Implementation Details

Add to keyboard_shortcuts.rs:
```rust
pub fn shortcut_help_text() -> Vec<(String, String)> {
    vec![
        ("Ctrl+Left".to_string(), "Toggle sidebar".to_string()),
        ("Ctrl+Right".to_string(), "Toggle learning panel".to_string()),
        ("Ctrl+U".to_string(), "Toggle usage stats".to_string()),
        ("Ctrl+P".to_string(), "Toggle practice settings".to_string()),
        ("Ctrl+Shift+S".to_string(), "Toggle auto-speak".to_string()),
        ("Ctrl+S".to_string(), "Replay last message".to_string()),
        ("Ctrl+D".to_string(), "Cycle dialect".to_string()),
        ("Ctrl+T".to_string(), "Cycle teaching mode".to_string()),
        ("Ctrl+R".to_string(), "Cycle formality".to_string()),
        ("Ctrl+G".to_string(), "Focus goal input".to_string()),
        ("Ctrl+C".to_string(), "Focus chat input".to_string()),
        ("Shift+Enter".to_string(), "Send message (in chat)".to_string()),
    ]
}
```

### Actions
1. Add shortcut_help_text function
2. Add comprehensive unit tests for matches_binding
3. Test edge cases (meta key on Mac, etc.)

### Phase End Verification
- Run full test suite: `cargo test`
- Run `cargo clippy -- -D warnings` for all crates
- No dead code allowed
- Manual testing of all shortcuts
- Update this document with COMPLETED status
- Git commit: "Phase 4 (Documentation and final verification) complete"

---

## Issues Encountered
(To be updated as implementation progresses)

## Success Criteria
- All 12 shortcuts implemented and functional
- Shortcut configuration data structure supports future customization
- No conflicts with browser defaults
- No dead code
- All tests pass
- Clean clippy output
