# CEFR Language Level Feature - COMPLETE

## Summary
✅ Successfully implemented per-dialect CEFR language levels (A1-C2) so the AI agent can tailor response complexity to user proficiency.

## User Agreement
- **Scope**: Per-dialect (user can have B2 in Spanish, A1 in Japanese)
- **Default**: B1 (Intermediate) for new dialects

## Implementation Status: COMPLETE
All 6 phases complete. Feature is fully functional and tested.

## CEFR Levels Reference
| Level | Name | Description |
|-------|------|-------------|
| A1 | Beginner | Basic phrases, very simple sentences |
| A2 | Elementary | Simple sentences, common vocabulary |
| B1 | Intermediate | Standard vocabulary, normal conversation |
| B2 | Upper Intermediate | Complex sentences, idioms naturally |
| C1 | Advanced | Sophisticated vocabulary, abstract topics |
| C2 | Proficient | Native-level complexity and nuance |

---

## Phase 1: Core Data Structures (shared crate)

**Subagent**: `kiss-code-generator`

### Code Style Checklist
- [ ] Functions < 20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No future-proofing

### Deliverables
1. `LanguageLevel` enum (A1-C2) with Default=B1
2. `DialectLevel` struct (dialect + level)
3. `UserState.dialect_levels: Vec<DialectLevel>` field
4. Helper methods: `get_level_for_dialect()`, `set_level_for_dialect()`, `current_language_level()`
5. Unit tests

### Files to Modify
- **MODIFY** `shared/src/models/user_state.rs`:
  - Add `LanguageLevel` enum after `UserGender` (~line 17)
  - Add `DialectLevel` struct
  - Add `dialect_levels` field to `UserState`
  - Add helper methods to `UserState` impl
  - Add tests

### Phase End
- Run `cargo test -p dialect-coach-shared`
- Run `cargo clippy --all`
- Commit: `git commit -m "Phase 1 (CEFR data structures) complete"`

---

## Phase 2: Frontend State Management

**Subagent**: `kiss-code-generator`

### Code Style Checklist
- [ ] Functions < 20 lines
- [ ] Reducer action follows existing patterns

### Deliverables
1. `UserStateAction::UpdateLanguageLevel(Dialect, LanguageLevel)` variant
2. Reducer implementation

### Files to Modify
- **MODIFY** `frontend/src/app/app_state.rs`:
  - Add `UpdateLanguageLevel(Dialect, LanguageLevel)` to `UserStateAction` enum (~line 427)
  - Add reducer case in `apply_user_state_action` (~line 676)

### Phase End
- Run `cargo test --all`
- Run `cargo clippy --all`
- Commit: `git commit -m "Phase 2 (frontend state management) complete"`

---

## Phase 3: Frontend UI - Settings Panel

**Subagent**: `kiss-code-generator`

### Code Style Checklist
- [ ] UI matches existing dropdown pattern
- [ ] Callback follows existing `on_X_change` pattern
- [ ] No new CSS (reuse existing classes)

### Deliverables
1. `on_language_level_change` callback function
2. Dropdown in Settings component (after "Your Gender")

### Files to Modify
- **MODIFY** `frontend/src/app/user_state/callbacks.rs`:
  - Add `on_language_level_change` callback (~after line 110)

- **MODIFY** `frontend/src/components/utility_sidebar/settings.rs`:
  - Add CEFR level dropdown after gender dropdown (~line 141)
  - Include help text: "Your proficiency level in the selected dialect (CEFR scale)"

### Phase End
- Run `trunk serve` and verify UI works
- Run `cargo test --all`
- Run `cargo clippy --all`
- Commit: `git commit -m "Phase 3 (settings UI) complete"`

---

## Phase 4: Message Context Updates

**Subagent**: `kiss-code-generator`

### Code Style Checklist
- [ ] Follow existing builder pattern

### Deliverables
1. Add `language_level` to message context structs
2. Update frontend context builder

### Files to Modify
- **MODIFY** `shared/src/models/message.rs`:
  - Add `language_level: LanguageLevel` to `ConversationContext` (~line 25)
  - Add `language_level: LanguageLevel` to `UserMessageWithContext` (~line 206)
  - Update `UserMessageWithContextBuilder` with field, `new()`, builder method, and `build()`

- **MODIFY** `frontend/src/app/callbacks.rs`:
  - Add `.language_level(state.current_language_level())` to builder chain (~line 53)

### Phase End
- Run `cargo test --all`
- Run `cargo clippy --all`
- Commit: `git commit -m "Phase 4 (message context) complete"`

---

## Phase 5: Backend Agent Integration

**Subagent**: `kiss-code-generator`

### Code Style Checklist
- [ ] Prompt instructions are concise
- [ ] Follows existing pattern for user_gender in prompts
- [ ] Pure function for level instructions

### Deliverables
1. `language_level_instruction(level: LanguageLevel) -> &'static str` helper
2. Updated `GenerateResponseParams` with `language_level` field
3. Updated `build_system_content()` to include level instructions

### Files to Modify
- **MODIFY** `backend/src/agent_service/response.rs`:
  - Add `language_level_instruction()` function (~line 380)
  - Add `language_level: LanguageLevel` to `GenerateResponseParams` (~line 521)
  - Update `build_system_content()` signature and format string
  - Update call site in `generate_response()` (~line 886)
  - Add test for `language_level_instruction`

- **MODIFY** `backend/src/websocket/agents.rs`:
  - Update `build_response_params` to include `language_level` from context

### Prompt Text per Level
```
A1: "LANGUAGE LEVEL A1 (Beginner): Use very basic vocabulary and simple present tense. Short sentences only. Repeat key words. Speak slowly and clearly."
A2: "LANGUAGE LEVEL A2 (Elementary): Use simple sentences and common vocabulary. Basic past and future tenses okay. Keep explanations brief and concrete."
B1: "LANGUAGE LEVEL B1 (Intermediate): Use standard vocabulary and grammar. Can introduce idioms with explanation. Normal conversational pace."
B2: "LANGUAGE LEVEL B2 (Upper Intermediate): Use varied vocabulary including some abstract concepts. Complex sentences okay. Can use idioms naturally."
C1: "LANGUAGE LEVEL C1 (Advanced): Use sophisticated vocabulary and nuanced expressions. Can discuss abstract topics. Full range of tenses and moods."
C2: "LANGUAGE LEVEL C2 (Proficient): Speak as you would to a native speaker. Full complexity, subtlety, and cultural references are appropriate."
```

### Phase End
- Run `cargo test --all`
- Run `cargo clippy --all`
- Commit: `git commit -m "Phase 5 (backend agent integration) complete"`

---

## Phase 6: Final Integration Testing

**Subagent**: `modular-builder`

### Manual Testing Checklist
1. [ ] Open settings, verify dropdown shows current level
2. [ ] Change dialect, verify level changes to that dialect's setting
3. [ ] Set Spanish to C1, Japanese to A1, switch between - verify persistence
4. [ ] Send message at A1 level - verify simple response
5. [ ] Send message at C2 level - verify sophisticated response
6. [ ] Refresh page - verify levels persist

### Phase End
- Run full test suite: `cargo test --all`
- Run `cargo clippy --all`
- Final commit: `git commit -m "Phase 6 (CEFR language levels feature complete)"`

---

## Critical Files Summary

| File | Changes |
|------|---------|
| `shared/src/models/user_state.rs` | LanguageLevel enum, DialectLevel struct, UserState field + helpers |
| `shared/src/models/message.rs` | Add language_level to context structs |
| `frontend/src/app/app_state.rs` | UpdateLanguageLevel action + reducer |
| `frontend/src/app/user_state/callbacks.rs` | on_language_level_change callback |
| `frontend/src/app/callbacks.rs` | Add .language_level() to builder |
| `frontend/src/components/utility_sidebar/settings.rs` | CEFR dropdown UI |
| `backend/src/agent_service/response.rs` | language_level_instruction(), prompt integration |
| `backend/src/websocket/agents.rs` | Pass language_level to response params |
