# Fix UpdateLevel Architecture Violation

## Problem Statement

The `UpdateLevel` action carries state (dialect) instead of just user input (level). This violates the reducer architecture principle:

> **Actions have inputs. Reducer handles state. Period.**

### Current (Broken) Flow

```rust
// callbacks.rs - reads state and passes it in action
pub fn on_language_level_change(
    user: Rc<UserState>,  // captured state
    dispatch: Callback<UserDomainAction>,
) -> Callback<Event> {
    Callback::from(move |e: Event| {
        let level = LanguageLevel::from_id(&select.value())...;
        let dialect = user.selected_dialect;  // READS STATE
        dispatch.emit(UpdateLevel(dialect, level));  // PASSES STATE IN ACTION
    })
}

// actions.rs
UpdateLevel(Dialect, LanguageLevel)  // carries state

// reducer.rs
UpdateLevel(dialect, level) => {
    next.set_level_for_dialect(dialect, level);  // uses action's dialect
}
```

### Why This Causes Bugs

When the user captures `Rc<UserState>` at callback creation time, the `selected_dialect` value can become stale. If the user:
1. Is on Spanish, changes level
2. Switches to Japanese
3. The callback still has old `user.selected_dialect = Spanish`
4. Changing Japanese level updates Spanish instead

### Correct Flow

```rust
// callbacks.rs - only dispatches user input
dispatch.emit(UpdateLevel(level));

// actions.rs
UpdateLevel(LanguageLevel)  // only input

// reducer.rs
UpdateLevel(level) => {
    next.set_level_for_dialect(next.selected_dialect, level);  // state from reducer
}
```

---

## Phase 1: Fix UpdateLevel Action

**Subagent**: kiss-code-generator

### Code Style Checklist
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No dead code
- [ ] No defensive coding
- [ ] Scope control - only fix UpdateLevel

### Files to Modify

1. **`frontend/src/app/app_state/user/actions.rs`**
   - Line 58: Change `UpdateLevel(Dialect, LanguageLevel)` to `UpdateLevel(LanguageLevel)`

2. **`frontend/src/app/app_state/user/reducer.rs`**
   - Lines 250-252: Change from using action parameter to using `next.selected_dialect`
   ```rust
   // Before
   UpdateLevel(dialect, level) => {
       next.set_level_for_dialect(dialect, level);
   }

   // After
   UpdateLevel(level) => {
       next.set_level_for_dialect(next.selected_dialect, level);
   }
   ```

3. **`frontend/src/app/user_state/callbacks.rs`**
   - Lines 106-120: Remove `user` parameter, only pass level
   ```rust
   // Before
   pub fn on_language_level_change(
       user: Rc<UserState>,
       dispatch: Callback<UserDomainAction>,
   ) -> Callback<Event> {
       Callback::from(move |e: Event| {
           let level = LanguageLevel::from_id(&select.value())...;
           let dialect = user.selected_dialect;
           dispatch.emit(UpdateLevel(dialect, level));
       })
   }

   // After
   pub fn on_language_level_change(
       dispatch: Callback<UserDomainAction>,
   ) -> Callback<Event> {
       Callback::from(move |e: Event| {
           if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>()
               && let Some(level) = LanguageLevel::from_id(&select.value())
           {
               dispatch.emit(UserDomainAction::Settings(SettingsAction::UpdateLevel(level)));
           }
       })
   }
   ```

4. **`frontend/src/app/app_state/user/tests.rs`**
   - Lines 827-842: Update test to use new signature
   ```rust
   // Before
   let action = UserStateAction::Settings(SettingsAction::UpdateLevel(
       Dialect::SpanishMexican,
       LanguageLevel::Cefr(CefrLevel::C1),
   ));

   // After
   state.selected_dialect = Dialect::SpanishMexican;  // Set state first
   let action = UserStateAction::Settings(SettingsAction::UpdateLevel(
       LanguageLevel::Cefr(CefrLevel::C1),
   ));
   ```

5. **`frontend/src/components/utility_sidebar/settings.rs`**
   - Line 150: Remove `user` argument from callback call
   ```rust
   // Before
   onchange={on_language_level_change(user.clone(), dispatch.clone())}

   // After
   onchange={on_language_level_change(dispatch.clone())}
   ```

### Deliverables
- `UpdateLevel` action only contains `LanguageLevel`
- Reducer uses `next.selected_dialect`
- Callback only dispatches level
- Test passes with new signature
- `cargo check` passes
- `cargo test` passes

### Phase Completion
- [ ] Run `cargo check`
- [ ] Run `cargo test`
- [ ] Run `cargo clippy` and fix all errors
- [ ] Update status document
- [ ] Git commit: `git commit -m "Phase 1 (fix UpdateLevel architecture) complete"`

---

## Status

- [x] Phase 1: Fix UpdateLevel Action
  - All 5 files modified successfully
  - `cargo check` passes with no errors
  - `cargo test` passes (306 passed; 0 failed; 3 ignored)
  - `cargo clippy` clean with no warnings/errors
  - Changes committed
