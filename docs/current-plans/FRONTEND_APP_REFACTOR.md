# App.rs Refactor Implementation Plan

**Status**: Ready to Execute  
**Date**: 2025-01-XX  
**Goal**: Refactor `frontend/src/app.rs` by moving callbacks, extracting WebSocket logic into hooks, and splitting HTML into components. Functionality must remain unchanged.

---

## Current State

**File Structure**:
- `frontend/src/app.rs` exists and declares `mod app_state;`
- `frontend/src/app/app_state.rs` contains all state definitions (AppState, UIState, OptionalUserState, reducers)
- All callback functions are in `app.rs` (25+ functions)
- All WebSocket initialization logic is in `app()` function (3 separate `use_effect_with` blocks)
- All HTML rendering is in `app.rs` (1000+ lines)

**No existing modules**:
- No `app/mod.rs` exists
- No callback modules exist
- No websocket hook modules exist
- No component modules exist for extracted HTML sections

---

## Refactoring Goals

1. **Move callbacks to organized modules**:
   - `app/user_state/callbacks.rs` - Callbacks dispatching `UserStateAction`
   - `app/app_state/callbacks.rs` - Callbacks dispatching `AppStateAction`
   - `app/callbacks.rs` - Callbacks affecting both states or neither

2. **Extract WebSocket logic into hooks**:
   - `app/websocket_hooks.rs` - Custom hooks for chat, user_state, and user WebSockets

3. **Split HTML into components**:
   - `components/header.rs` - Header with user auth and connection status
   - `components/settings_panel.rs` - Practice settings panel
   - `components/welcome_screen.rs` - Welcome message for unauthenticated users
   - `components/main_content.rs` - Main chat interface and learning panels

4. **Create helper functions module**:
   - `app/helpers.rs` - Pure helper functions like `extract_learning_items`, `render_message_undo_notification`

---

## Implementation Phases

### Phase 1: Create Module Structure

**Code Style Guidelines**:
- Review `.claude/CLAUDE.md` for code style guidelines
- Functions must be <20 lines (prefer <10 lines)
- Code must be simple and modular
- No inline styles - use CSS classes where applicable
- Use `cargo fmt` to format code
- Use `cargo clippy` to check for linting issues

**Files to Create**:
- `frontend/src/app/mod.rs`
- `frontend/src/app/helpers.rs`
- `frontend/src/app/callbacks.rs`
- `frontend/src/app/app_state/callbacks.rs`
- `frontend/src/app/user_state/mod.rs`
- `frontend/src/app/user_state/callbacks.rs`
- `frontend/src/app/websocket_hooks.rs`

**Files to Modify**:
- `frontend/src/app/app_state.rs` - Add `pub mod callbacks;` declaration at the top

**Tasks**:
1. Create `frontend/src/app/mod.rs` with module declarations
2. Add `pub mod callbacks;` to `frontend/src/app/app_state.rs`
3. Create all other empty module files

**Completion Criteria**:
- [ ] All files created
- [ ] Module declarations compile (`cargo check`)
- [ ] No functionality changed

**Documentation Update**:
- Update this document with completed work for Phase 1

---

### Phase 2: Move Helper Functions

**Code Style Guidelines**:
- Review `.claude/CLAUDE.md` for code style guidelines
- Functions must be <20 lines (prefer <10 lines)
- Code must be simple and modular
- Prefer pure functions when possible
- No inline styles - use CSS classes where applicable
- Use `cargo fmt` to format code
- Use `cargo clippy` to check for linting issues

**Files to Modify**:
- `frontend/src/app.rs` - Remove helper functions
- `frontend/src/app/helpers.rs` - Add helper functions

**Functions to Move**:
1. `extract_learning_items` (lines 19-46)
2. `render_message_undo_notification` (lines 48-59)

**Tasks**:
1. Move functions to `app/helpers.rs` with proper imports
2. Update `app.rs` to import from `crate::app::helpers`
3. Verify compilation

**Completion Criteria**:
- [ ] Functions moved and compile
- [ ] Imports updated in `app.rs`
- [ ] Functionality unchanged

**Documentation Update**:
- Update this document with completed work for Phase 2

---

### Phase 3: Move UserState Callbacks

**Code Style Guidelines**:
- Review `.claude/CLAUDE.md` for code style guidelines
- Functions must be <20 lines (prefer <10 lines)
- Code must be simple and modular
- Break complex callbacks into smaller helper functions if needed
- No inline styles - use CSS classes where applicable
- Use `cargo fmt` to format code
- Use `cargo clippy` to check for linting issues

**Files to Modify**:
- `frontend/src/app.rs` - Remove callback functions
- `frontend/src/app/user_state/callbacks.rs` - Add callback functions

**Functions to Move** (dispatch `UserStateAction`):
1. `on_language_change` (lines ~176)
2. `on_dialect_change` (lines ~188)
3. `on_formality_change` (lines ~217)
4. `on_teaching_mode_change` (lines ~237)
5. `on_dialect_cycle` (lines ~291)
6. `on_formality_cycle` (lines ~300)
7. `on_teaching_mode_cycle` (lines ~320)
8. `on_delete_message_callback` (lines ~497)
9. `on_undo_message_callback` (lines ~513)
10. `on_create_branch` (lines ~529)
11. `on_switch_branch` (lines ~537)
12. `on_delete_branch` (lines ~545)
13. `on_add_goal` (lines ~556)
14. `on_delete_goal` (lines ~564)
15. `on_delete_learning_item_callback` - needs to be created (similar to `on_delete_message_callback`)

**Tasks**:
1. Move all functions to `app/user_state/callbacks.rs`
2. Add proper imports (UseReducerHandle, Callback, etc.)
3. Update `app.rs` to import from `crate::app::user_state::callbacks`
4. Create `on_delete_learning_item_callback` if missing
5. Verify all callbacks still work

**Completion Criteria**:
- [ ] All functions moved and compile
- [ ] Imports updated in `app.rs`
- [ ] All callback usages updated
- [ ] Functionality unchanged

**Documentation Update**:
- Update this document with completed work for Phase 3

---

### Phase 4: Move AppState Callbacks

**Code Style Guidelines**:
- Review `.claude/CLAUDE.md` for code style guidelines
- Functions must be <20 lines (prefer <10 lines)
- Code must be simple and modular
- Break complex callbacks into smaller helper functions if needed
- No inline styles - use CSS classes where applicable
- Use `cargo fmt` to format code
- Use `cargo clippy` to check for linting issues

**Files to Modify**:
- `frontend/src/app.rs` - Remove callback functions
- `frontend/src/app/app_state/callbacks.rs` - Add callback functions

**Functions to Move** (dispatch `AppStateAction`):
1. `on_replay_message` (lines ~257)
2. `on_user_state_ws_open` (lines ~346)
3. `on_user_state_load_response` (lines ~356)
4. `on_user_state_save_response` (lines ~378)

**Tasks**:
1. Move all functions to `app/app_state/callbacks.rs`
2. Add proper imports
3. Update `app.rs` to import from `crate::app::app_state::callbacks`
4. Verify compilation

**Completion Criteria**:
- [ ] All functions moved and compile
- [ ] Imports updated in `app.rs`
- [ ] Functionality unchanged

**Documentation Update**:
- Update this document with completed work for Phase 4

---

### Phase 5: Move Mixed Callbacks

**Code Style Guidelines**:
- Review `.claude/CLAUDE.md` for code style guidelines
- Functions must be <20 lines (prefer <10 lines)
- Code must be simple and modular
- Break complex callbacks into smaller helper functions if needed
- No inline styles - use CSS classes where applicable
- Use `cargo fmt` to format code
- Use `cargo clippy` to check for linting issues

**Files to Modify**:
- `frontend/src/app.rs` - Remove callback functions
- `frontend/src/app/callbacks.rs` - Add callback functions

**Functions to Move** (affect both or neither):
1. `on_send_message` (lines 59-120)
2. `on_prompt_click` (lines 122-180)
3. `on_tts_toggle` (lines ~273)
4. `on_create_user_click` (lines ~398)
5. `on_signin_click` (lines ~421)
6. `on_user_create_response` (lines ~427)
7. `on_user_signin_response` (lines ~456)
8. `on_signout_click` (lines ~481)

**Tasks**:
1. Move all functions to `app/callbacks.rs`
2. Add proper imports
3. Update `app.rs` to import from `crate::app::callbacks`
4. Verify compilation

**Completion Criteria**:
- [ ] All functions moved and compile
- [ ] Imports updated in `app.rs`
- [ ] Functionality unchanged

**Documentation Update**:
- Update this document with completed work for Phase 5

---

### Phase 6: Extract WebSocket Hooks

**Code Style Guidelines**:
- Review `.claude/CLAUDE.md` for code style guidelines
- Functions must be <20 lines (prefer <10 lines)
- Code must be simple and modular
- Break complex hooks into smaller helper functions if needed
- No inline styles - use CSS classes where applicable
- Use `cargo fmt` to format code
- Use `cargo clippy` to check for linting issues

**Files to Modify**:
- `frontend/src/app.rs` - Remove `use_effect_with` blocks
- `frontend/src/app/websocket_hooks.rs` - Add hook functions

**WebSocket Blocks to Extract**:
1. Chat WebSocket (lines ~615-711)
2. UserState WebSocket (lines ~713-751)
3. User WebSocket (lines ~753-783)

**Tasks**:
1. Create `use_chat_websocket` hook function
2. Create `use_user_state_websocket` hook function
3. Create `use_user_websocket` hook function
4. Replace `use_effect_with` blocks in `app()` with hook calls
5. Add proper imports to `websocket_hooks.rs`
6. Export hooks from `app/mod.rs`

**Completion Criteria**:
- [ ] All hooks created and compile
- [ ] `app()` uses hooks instead of inline `use_effect_with`
- [ ] Functionality unchanged

**Documentation Update**:
- Update this document with completed work for Phase 6

---

### Phase 7: Extract Header Component

**Code Style Guidelines**:
- Review `.claude/CLAUDE.md` for code style guidelines
- Functions must be <20 lines (prefer <10 lines)
- Code must be simple and modular
- No inline styles - use CSS classes
- Component props struct must be simple and clear
- Use `cargo fmt` to format code
- Use `cargo clippy` to check for linting issues

**Files to Create**:
- `frontend/src/components/header.rs`

**Files to Modify**:
- `frontend/src/app.rs` - Replace header HTML with component
- `frontend/src/components/mod.rs` - Export Header component

**HTML to Extract**:
- `<header class="app-header">` section (lines ~806-893)

**Props Needed**:
- `app_state: UseReducerHandle<AppState>`
- `ui_state: UseReducerHandle<UIState>`
- `user_state: UseReducerHandle<OptionalUserState>`
- Callbacks: `on_create_user_click`, `on_signin_click`, `on_signout_click`

**Tasks**:
1. Create `Header` component with Props struct
2. Move header HTML into component
3. Update `app.rs` to use `<Header />`
4. Export Header from `components/mod.rs`

**Completion Criteria**:
- [ ] Component created and compiles
- [ ] Header renders correctly
- [ ] Functionality unchanged

**Documentation Update**:
- Update this document with completed work for Phase 7

---

### Phase 8: Extract Settings Panel Component

**Code Style Guidelines**:
- Review `.claude/CLAUDE.md` for code style guidelines
- Functions must be <20 lines (prefer <10 lines)
- Code must be simple and modular
- No inline styles - use CSS classes
- Component props struct must be simple and clear
- Use `cargo fmt` to format code
- Use `cargo clippy` to check for linting issues

**Files to Create**:
- `frontend/src/components/settings_panel.rs`

**Files to Modify**:
- `frontend/src/app.rs` - Replace settings panel HTML with component
- `frontend/src/components/mod.rs` - Export SettingsPanel component

**HTML to Extract**:
- `<div class="panel">` section (lines ~1036-1099)

**Props Needed**:
- `user_state: UseReducerHandle<OptionalUserState>`
- `ui_state: UseReducerHandle<UIState>`
- Callbacks: `on_language_change`, `on_dialect_change`, `on_formality_change`, `on_teaching_mode_change`

**Tasks**:
1. Create `SettingsPanel` component with Props struct
2. Move settings panel HTML into component
3. Update `app.rs` to use `<SettingsPanel />`
4. Export SettingsPanel from `components/mod.rs`

**Completion Criteria**:
- [ ] Component created and compiles
- [ ] Settings panel renders correctly
- [ ] Functionality unchanged

**Documentation Update**:
- Update this document with completed work for Phase 8

---

### Phase 9: Extract Welcome Screen Component

**Code Style Guidelines**:
- Review `.claude/CLAUDE.md` for code style guidelines
- Functions must be <20 lines (prefer <10 lines)
- Code must be simple and modular
- No inline styles - use CSS classes
- Component props struct must be simple and clear
- Use `cargo fmt` to format code
- Use `cargo clippy` to check for linting issues

**Files to Create**:
- `frontend/src/components/welcome_screen.rs`

**Files to Modify**:
- `frontend/src/app.rs` - Replace welcome screen HTML with component
- `frontend/src/components/mod.rs` - Export WelcomeScreen component

**HTML to Extract**:
- Welcome message shown when `user_state.0.is_none()` (lines ~1125+)

**Props Needed**:
- None (static content)

**Tasks**:
1. Create `WelcomeScreen` component
2. Move welcome HTML into component
3. Update `app.rs` to use `<WelcomeScreen />`
4. Export WelcomeScreen from `components/mod.rs`

**Completion Criteria**:
- [ ] Component created and compiles
- [ ] Welcome screen renders correctly
- [ ] Functionality unchanged

**Documentation Update**:
- Update this document with completed work for Phase 9

---

### Phase 10: Extract Main Content Component

**Code Style Guidelines**:
- Review `.claude/CLAUDE.md` for code style guidelines
- Functions must be <20 lines (prefer <10 lines)
- Code must be simple and modular
- Break complex render logic into smaller helper functions if needed
- No inline styles - use CSS classes
- Component props struct must be simple and clear
- Use `cargo fmt` to format code
- Use `cargo clippy` to check for linting issues

**Files to Create**:
- `frontend/src/components/main_content.rs`

**Files to Modify**:
- `frontend/src/app.rs` - Replace main content HTML with component
- `frontend/src/components/mod.rs` - Export MainContent component

**HTML to Extract**:
- Main content shown when `user_state.0.is_some()` (lines ~895-1123)
- Includes: BranchSidebar, ChatWindow, SpeechControls, InputBox, LearningPanel, SettingsPanel toggle, LearningPanel toggle

**Props Needed**:
- `app_state: UseReducerHandle<AppState>`
- `ui_state: UseReducerHandle<UIState>`
- `user_state: UseReducerHandle<OptionalUserState>`
- All relevant callbacks

**Tasks**:
1. Create `MainContent` component with Props struct
2. Move main content HTML into component
3. Update `app.rs` to use `<MainContent />`
4. Export MainContent from `components/mod.rs`
5. Ensure all callbacks are passed correctly

**Completion Criteria**:
- [ ] Component created and compiles
- [ ] Main content renders correctly
- [ ] All callbacks work
- [ ] Functionality unchanged

**Documentation Update**:
- Update this document with completed work for Phase 10

---

### Phase 11: Final Cleanup and Verification

**Code Style Guidelines**:
- Review `.claude/CLAUDE.md` for code style guidelines
- Functions must be <20 lines (prefer <10 lines)
- Code must be simple and modular
- No inline styles - use CSS classes
- Use `cargo fmt` to format all code
- Use `cargo clippy` to check for linting issues
- Fix all clippy warnings

**Files to Modify**:
- `frontend/src/app.rs` - Remove unused imports, verify structure
- All module files - Verify exports

**Tasks**:
1. Clean up unused imports in `app.rs`
2. Verify all modules export correctly
3. Run full test suite
4. Manual testing of all functionality
5. Verify `app.rs` is significantly reduced in size

**Completion Criteria**:
- [ ] All tests pass
- [ ] No compilation warnings
- [ ] `app.rs` is <200 lines (down from 1000+)
- [ ] All functionality works identically
- [ ] Code is organized and maintainable

**Documentation Update**:
- Update this document with completed work for Phase 11
- Mark all phases as complete
- Update status to "Completed"

---

## Critical Rules

1. **NO FILE MOVES, RENAMES, OR DELETIONS** - Only create new files and modify existing ones
2. **NO FUNCTIONALITY CHANGES** - This is a pure refactor
3. **NO OPTIMIZATIONS** - Only move code, don't improve it
4. **ALL FUNCTIONS MUST COMPILE** after each phase
5. **VERIFY IMPORTS** at each step to ensure module resolution works

---

## Module Structure After Refactor

frontend/src/
├── app.rs (minimal - just component and hook calls)
├── app/
│ ├── mod.rs
│ ├── helpers.rs
│ ├── callbacks.rs
│ ├── websocket_hooks.rs
│ ├── app_state.rs (existing)
│ ├── app_state/
│ │ └── callbacks.rs
│ └── user_state/
│ ├── mod.rs
│ └── callbacks.rs
└── components/
├── mod.rs (updated)
├── header.rs
├── settings_panel.rs
├── welcome_screen.rs
└── main_content.rs


---

## Verification Checklist

Before marking complete:
- [ ] `cargo check` passes
- [ ] `cargo build` succeeds
- [ ] Application runs in browser
- [ ] All callbacks work correctly
- [ ] All WebSocket connections work
- [ ] All UI components render
- [ ] No regressions in functionality
- [ ] `app.rs` is significantly smaller
- [ ] Code is better organized

---

## Notes

- All callback function signatures must remain identical
- All WebSocket logic must behave identically
- All HTML rendering must produce identical output
- Module paths must be correct for Rust's module system
- `pub mod` declarations must be in correct locations
