# Frontend Dirty State - Implementation Plan

## Overview

Add automatic save functionality when learning items are added via a `needs_save` flag on `OptionalUserState` that triggers an immediate save effect. This eliminates manual callback wiring while ensuring all learning item additions trigger saves.

## Architecture Decision

**Modify Existing Code In Place:**
- Change `OptionalUserState` from tuple struct `OptionalUserState(Option<UserState>)` to regular struct with `state` and `needs_save` fields
- Update ALL references from `.0` to `.state` across entire frontend codebase
- Add `MarkSaved` action to clear the flag after successful save
- Add save effect in app.rs that watches `needs_save`

**Files With `.0` Access (from grep):**
- frontend/src/app.rs (2 locations)
- frontend/src/app/callbacks.rs
- frontend/src/components/main_content.rs (2 locations)
- frontend/src/app/app_state.rs (4 locations in reduce method)
- frontend/src/components/learning_panel.rs
- frontend/src/app/user_state/callbacks.rs (multiple locations)
- frontend/src/components/settings_panel.rs
- frontend/src/services/speech.rs
- frontend/src/hooks/use_debounced_save.rs
- frontend/src/app/helpers.rs

---

## Phase 1: Refactor OptionalUserState Structure

**Subagent:** kiss-code-generator (single-file, structural change)

**Code Style Checklist:**
- [ ] Functions <20 lines (helper functions for field access if needed)
- [ ] Pure functions where possible
- [ ] No defensive coding - trust the types
- [ ] No dead code
- [ ] Tests for new structure

**Deliverables:**
- `OptionalUserState` changed from tuple struct to regular struct with `state: Option<UserState>` and `needs_save: bool` fields
- Default implementation provides `state: None, needs_save: false`
- `reduce` method updated to use `.state` instead of `.0`
- All test constructors updated to use new struct syntax

**Files to Update:**
- frontend/src/app/app_state.rs

**Implementation Steps:**

1. Change OptionalUserState struct definition (app_state.rs:733):
   ```rust
   #[derive(Clone, PartialEq)]
   pub struct OptionalUserState {
       pub state: Option<UserState>,
       pub needs_save: bool,
   }
   ```

2. Update reduce method to use `.state` instead of `.0` (app_state.rs:738-753):
   - Change all `self.0` to `self.state`
   - Change all `OptionalUserState(Some(new_state))` to `OptionalUserState { state: Some(new_state), needs_save: false }`
   - Change `OptionalUserState(None)` to `OptionalUserState { state: None, needs_save: false }`

3. Update test constructors (app_state.rs:757-1303):
   - Change all `OptionalUserState(Some(base))` to `OptionalUserState { state: Some(base), needs_save: false }`
   - Change all `OptionalUserState(None)` to `OptionalUserState { state: None, needs_save: false }`

**Phase Completion:**
- Run `cargo test` in frontend/ directory (100% pass required)
- Run `cargo clippy` in frontend/ directory (fix ALL errors including dead code)
- Update this document with completion timestamp
- Commit: `git add -A && git commit -m "Phase 1 (OptionalUserState refactor) complete"`

---

## Phase 2: Update All `.0` References to `.state`

**Subagent:** modular-builder (cross-file refactoring)

**Code Style Checklist:**
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No dead code
- [ ] All references updated

**Deliverables:**
- All `.0` accesses on `user_state` changed to `.state` across entire frontend
- No compilation errors
- All tests passing

**Files to Update:**
- frontend/src/app.rs (line 36, 95)
- frontend/src/app/callbacks.rs
- frontend/src/components/main_content.rs (line 46, 62, 113, 130, 174, 237)
- frontend/src/components/learning_panel.rs
- frontend/src/app/user_state/callbacks.rs (lines 10, 31, 51, 72, 95, 113, 129, 150, 175, 195, 207, 217, 227, 237, 252, 265, 288, 307)
- frontend/src/components/settings_panel.rs
- frontend/src/services/speech.rs
- frontend/src/hooks/use_debounced_save.rs
- frontend/src/app/helpers.rs

**Implementation Steps:**

1. For each file listed above:
   - Find all instances of `user_state.0`
   - Replace with `user_state.state`
   - Verify logic remains correct

2. Special attention to pattern matches:
   - `if let Some(state) = user_state.0.as_ref()` → `if let Some(state) = user_state.state.as_ref()`
   - `match user_state.0.as_ref()` → `match user_state.state.as_ref()`
   - `if user_state.0.is_some()` → `if user_state.state.is_some()`
   - `if user_state.0.is_none()` → `if user_state.state.is_none()`

**Phase Completion:**
- Run `cargo check` to verify no compilation errors
- Run `cargo test` in frontend/ directory (100% pass required)
- Run `cargo clippy` in frontend/ directory (fix ALL errors including dead code)
- Update this document with completion timestamp
- Commit: `git add -A && git commit -m "Phase 2 (update .0 to .state references) complete"`

---

## Phase 3: Add needs_save Flag Logic and Save Effect

**Subagent:** modular-builder (cross-file, new logic)

**Code Style Checklist:**
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No dead code
- [ ] Tests for new action

**Deliverables:**
- `MarkSaved` action added to `UserStateAction` enum
- `reduce` method sets `needs_save = true` on `AddLearningItems` action
- `reduce` method handles `MarkSaved` action by clearing flag
- Save effect added to app.rs that watches `needs_save` flag
- Effect calls save on flag set, dispatches `MarkSaved` on success, `QueuePendingSave` on error

**Files to Update:**
- frontend/src/app/app_state.rs (enum, reducer logic)
- frontend/src/app.rs (add effect after debounced save)

**Implementation Steps:**

1. Add `MarkSaved` action to UserStateAction enum (app_state.rs:366):
   ```rust
   pub enum UserStateAction {
       MarkSaved,  // Add at top
       AddMessage(Message),
       // ... rest
   }
   ```

2. Update reduce method in OptionalUserState (app_state.rs:738):
   - Add case for `MarkSaved`:
     ```rust
     UserStateAction::MarkSaved => OptionalUserState {
         state: self.state.clone(),
         needs_save: false
     }.into(),
     ```
   - Update the fallback arm that calls `apply_user_state_action`:
     ```rust
     _ => match &self.state {
         Some(state) => {
             let prepared = prepare_state_for_action(state);
             let next_state = apply_user_state_action(&prepared, action);
             let mut needs_save = self.needs_save;

             if matches!(action, UserStateAction::AddLearningItems(..)) {
                 needs_save = true;
             }

             OptionalUserState {
                 state: Some(next_state),
                 needs_save
             }.into()
         }
         None => self,
     }
     ```

3. Add save effect in app.rs (after line 52, before websocket hooks):
   ```rust
   // Immediate save effect for learning-item additions
   {
       let app_state_for_effect = app_state.clone();
       let user_state_for_effect = user_state.clone();
       use_effect_with((user_state.needs_save, user_state.state.clone()), move |(needs_save, state_opt)| {
           if !*needs_save {
               return || ();
           }
           if let Some(state) = state_opt {
               let ws = app_state_for_effect.user_state_ws_service.borrow();
               match ws.save_user_state(state) {
                   Ok(()) => {
                       info!("Immediate save request sent via WebSocket");
                       user_state_for_effect.dispatch(UserStateAction::MarkSaved);
                   }
                   Err(e) => {
                       error!("Immediate save failed: {}", e);
                       app_state_for_effect.dispatch(AppStateAction::QueuePendingSave(state.clone()));
                       // Keep needs_save true for retry
                   }
               }
           }
           || ()
       });
   }
   ```

4. Add test for MarkSaved action in app_state.rs tests:
   ```rust
   #[test]
   fn test_mark_saved_clears_flag() {
       let state = UserState::new(Uuid::new_v4());
       let optional = Rc::new(OptionalUserState {
           state: Some(state),
           needs_save: true
       });
       let updated = OptionalUserState::reduce(optional, UserStateAction::MarkSaved);

       assert!(!updated.needs_save);
       assert!(updated.state.is_some());
   }

   #[test]
   fn test_add_learning_items_sets_needs_save() {
       let state = UserState::new(Uuid::new_v4());
       let optional = Rc::new(OptionalUserState {
           state: Some(state),
           needs_save: false
       });
       let updated = OptionalUserState::reduce(
           optional,
           UserStateAction::AddLearningItems(vec![], vec![], vec![], vec![])
       );

       assert!(updated.needs_save);
       assert!(updated.state.is_some());
   }
   ```

**Phase Completion:**
- Run `cargo test` in frontend/ directory (100% pass required)
- Run `cargo clippy` in frontend/ directory (fix ALL errors including dead code)
- Update this document with completion timestamp
- Commit: `git add -A && git commit -m "Phase 3 (needs_save flag and save effect) complete"`

---

## Success Criteria

1. All learning item additions (via translation modal, manual entry, etc.) trigger immediate save
2. No manual callback wiring required for new learning item sources
3. Save failures queue for retry without clearing needs_save flag
4. All existing tests pass
5. No clippy warnings
6. No dead code

## Testing Strategy

**Unit Tests (60%):**
- Test MarkSaved clears flag
- Test AddLearningItems sets flag
- Test reducer preserves flag state correctly
- Test struct construction

**Integration Tests (30%):**
- Verify save effect triggers on flag change
- Verify MarkSaved dispatched on success
- Verify QueuePendingSave dispatched on error

**Manual Testing (10%):**
- Add learning items via translation modal
- Verify save occurs immediately
- Verify behavior on connection failure

## Notes

- This approach is simpler than per-callback wiring
- Any future learning item addition paths inherit save behavior
- The debounced save can remain for other state changes
- No changes needed to translation modal or learning panel components
