# Gender Client Implementation Plan

## Overview

Add user gender support to allow users to specify their gender (Male, Female, NonBinary) for personalized language learning. The agent will use this to provide gender-appropriate language instruction.

## Research Summary

**Verified Code Structures:**
- `UserState` in shared/src/models/user_state.rs (lines 11-25)
- Settings UI in frontend/src/components/settings_panel.rs (lines 90-116)
- Callbacks in frontend/src/app/user_state/callbacks.rs
- `UserMessageWithContext` in shared/src/models/message.rs (lines 147-167)
- `GenerateResponseParams` in backend/src/agent_service/response.rs (lines 372-384)
- `build_system_content()` in backend/src/agent_service/response.rs (lines 280-298)
- Tests are colocated in implementation files (e.g., user_state.rs:263-544)

## Phase 1: Add UserGender Enum and UserState Field

**Subagent:** kiss-code-generator

**Code Style Checklist:**
- [ ] Functions <20 lines (<10 preferred)
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] Helper functions for complex logic
- [ ] Low cyclomatic complexity
- [ ] No dead code
- [ ] No fake constructions
- [ ] Tests for all new functions

**Files to Modify:**
- shared/src/models/user_state.rs

**Implementation Steps:**

1. Add `UserGender` enum after line 9 (before `UserState` struct):
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserGender {
    Male,
    Female,
    NonBinary,
}
```

2. Add `user_gender` field to `UserState` struct at line 21 (after `teaching_mode: TeachingMode,`):
```rust
pub user_gender: UserGender,
```

3. Update `UserState::new()` at line 64 (after `teaching_mode: TeachingMode::Immersive,`):
```rust
user_gender: UserGender::NonBinary,
```

4. Add test for UserGender enum (after existing UserState tests):
```rust
#[test]
fn test_user_gender_serialization() {
    let genders = vec![UserGender::Male, UserGender::Female, UserGender::NonBinary];
    for gender in genders {
        let json = serde_json::to_string(&gender).unwrap();
        let deserialized: UserGender = serde_json::from_str(&json).unwrap();
        assert_eq!(gender, deserialized);
    }
}
```

**Deliverables:**
- UserGender enum defined
- user_gender field added to UserState
- Default value set in UserState::new()
- Test passes
- Automatic persistence via serde (no additional work needed)

**Phase End:**
- [ ] Run: `cargo test` - verify 100% pass rate
- [ ] Run: `cargo clippy` - fix ALL warnings/errors
- [ ] Remove any dead code
- [ ] Update: docs/current-plans/gender_client_implementation.md status
- [ ] Commit: `git commit -m "Phase 1 (UserGender enum and UserState field) complete"`
- [ ] **STOP - Wait for explicit approval**

**Phase 1 Status:** COMPLETE (2025-11-21)
- UserGender enum added with Male, Female, NonBinary variants
- user_gender field added to UserState (after teaching_mode)
- Default value set to NonBinary in UserState::new()
- test_user_gender_serialization added and passing
- All 370+ tests passing (100%)
- Clippy clean (0 warnings)

---

## Phase 2: Add UI Dropdown in Settings Panel

**Subagent:** kiss-code-generator

**Code Style Checklist:**
- [ ] Functions <20 lines (<10 preferred)
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] Helper functions for complex logic
- [ ] Low cyclomatic complexity
- [ ] No dead code
- [ ] No fake constructions
- [ ] Tests for all new functions

**Files to Modify:**
- frontend/src/components/settings_panel.rs
- frontend/src/app/user_state/callbacks.rs

**Implementation Steps:**

1. In frontend/src/app/user_state/callbacks.rs, add callback function (following pattern of on_formality_change):
```rust
pub fn on_user_gender_change(
    user_state: UseReducerHandle<OptionalUserState>,
) -> Callback<Event> {
    Callback::from(move |e: Event| {
        if let Some(select) = e.target_dyn_into::<HtmlSelectElement>() {
            let gender = match select.value().as_str() {
                "male" => UserGender::Male,
                "female" => UserGender::Female,
                "nonbinary" => UserGender::NonBinary,
                _ => UserGender::NonBinary,
            };
            user_state.dispatch(UserStateAction::UpdateUserGender(gender));
        }
    })
}
```

2. In frontend/src/app/user_state/mod.rs, add action to UserStateAction enum:
```rust
UpdateUserGender(UserGender),
```

3. In frontend/src/app/user_state/mod.rs, add reducer case (in reduce function):
```rust
UserStateAction::UpdateUserGender(gender) => {
    if let Some(ref mut state) = inner.0 {
        state.user_gender = gender;
    }
}
```

4. In frontend/src/components/settings_panel.rs, add gender dropdown after teaching-mode-select div (around line 115):
```rust
<div class="panel-field">
    <label for="user-gender-select">{"Your Gender"}</label>
    <select id="user-gender-select" onchange={on_user_gender_change(user_state.clone())}>
        <option value="male">{"Male"}</option>
        <option value="female">{"Female"}</option>
        <option value="nonbinary" selected=true>{"Non-binary"}</option>
    </select>
</div>
```

5. Import UserGender and callback in settings_panel.rs imports section

**Deliverables:**
- Dropdown renders in settings panel
- Callback updates UserState.user_gender
- UserStateAction reducer handles update
- State persists automatically via existing persistence

**Phase End:**
- [ ] Run: `cd frontend && trunk build` - verify no compile errors
- [ ] Run: `cargo test` - verify 100% pass rate
- [ ] Run: `cargo clippy` - fix ALL warnings/errors
- [ ] Remove any dead code
- [ ] Update: docs/current-plans/gender_client_implementation.md status
- [ ] Commit: `git commit -m "Phase 2 (UI dropdown for user gender) complete"`
- [ ] **STOP - Wait for explicit approval**

**Phase 2 Status:** Not started

---

## Phase 3: Pass User Gender Through Backend Pipeline

**Subagent:** modular-builder

**Code Style Checklist:**
- [ ] Functions <20 lines (<10 preferred)
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] Helper functions for complex logic
- [ ] Low cyclomatic complexity
- [ ] No dead code
- [ ] No fake constructions
- [ ] Tests for all new functions

**Files to Modify:**
- shared/src/models/message.rs
- backend/src/agent_service/response.rs
- backend/src/websocket/agents.rs

**Implementation Steps:**

1. Add user_gender field to `UserMessageWithContext` in shared/src/models/message.rs (after learning_goals field, around line 156):
```rust
pub user_gender: UserGender,
```

2. Update `UserMessageWithContext::new()` signature and initialization in shared/src/models/message.rs (around line 160):
   - Add parameter: `user_gender: UserGender,`
   - Add field initialization: `user_gender,`

3. Update call to `UserMessageWithContext::new()` in backend/src/websocket/agents.rs (find the callsite, likely in handle_user_message):
   - Add argument: `user_state.user_gender`

4. Add user_gender field to `GenerateResponseParams` in backend/src/agent_service/response.rs (after past_exploratory field, around line 383):
```rust
pub user_gender: UserGender,
```

5. Update `GenerateResponseParams` initialization in backend/src/websocket/agents.rs (around line 145):
   - Add field: `user_gender: msg_with_context.user_gender,`

6. Modify `build_system_content()` signature in backend/src/agent_service/response.rs (line 280):
   - Add parameter: `user_gender: UserGender,`

7. Update `build_system_content()` body to include user gender in system prompt (around line 298):
```rust
let user_gender_str = match user_gender {
    UserGender::Male => "male",
    UserGender::Female => "female",
    UserGender::NonBinary => "non-binary",
};

format!(
    "{role_desc}\n\n\
     USER GENDER: The student you're speaking with is {user_gender_str}. Adapt your language instruction to use gender-appropriate forms when teaching grammar and vocabulary.\n\n\
     FORMALITY: {formality_label}\n\n\
     {teaching_rules}\n\n\
     {goals_section}\
     {past_learning}"
)
```

8. Update call to `build_system_content()` in backend/src/agent_service/response.rs (find callsite in generate_response_internal):
   - Add argument: `params.user_gender`

9. Update tests in shared/src/models/message.rs and backend/src/agent_service/response.rs to include user_gender parameter

**Deliverables:**
- user_gender flows from UserState → UserMessageWithContext → GenerateResponseParams → build_system_content()
- Agent system prompt includes user gender information
- All tests pass
- No dead code

**Phase End:**
- [ ] Run: `cargo test` - verify 100% pass rate (ALL tests must pass)
- [ ] Run: `cargo clippy` - fix ALL warnings/errors
- [ ] Remove any dead code
- [ ] Update: docs/current-plans/gender_client_implementation.md status
- [ ] Commit: `git commit -m "Phase 3 (user gender through backend pipeline) complete"`
- [ ] **STOP - Wait for explicit approval**

**Phase 3 Status:** Not started

---

## Success Criteria

- [ ] UserGender enum with Male, Female, NonBinary variants
- [ ] user_gender field in UserState with default NonBinary
- [ ] UI dropdown in settings panel (Conversation Style section)
- [ ] Dropdown updates UserState via callback
- [ ] user_gender persists automatically
- [ ] user_gender passed through UserMessageWithContext
- [ ] user_gender in GenerateResponseParams
- [ ] Agent system prompt includes user gender
- [ ] All tests pass (100% success rate)
- [ ] No clippy warnings
- [ ] No dead code
