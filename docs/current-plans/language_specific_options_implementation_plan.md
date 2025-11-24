# Language-Specific Options - Implementation Plan

## Overview

This plan implements language-specific display and formatting options (Arabic script options and Japanese script options) that are persisted with user settings and sent to all agents. The implementation follows the approved specification in `language_specific_options.md`.

**Key Decisions:**
- Japanese Ruby Text: Agent-generated `<ruby><rt>` tags (not frontend WASM library)
- Arabic Fonts: Naskh uses system fonts, Ruq'a uses existing "Aref Ruqaa", Latin uses default
- Romanization: Agents use their own knowledge (not standardized)
- State Migration: None needed (DB will be deleted)

## Phase 1: Shared Models and Language Family Classification ✅ COMPLETE

**Subagent:** kiss-code-generator

**Code Style Checklist:**
- [x] Functions <20 lines, <10 if possible
- [x] Helper functions instead of nested logic
- [x] Pure functions where possible
- [x] Tests for all new functions
- [x] No TODOs or future-proofing
- [x] No defensive coding

**Deliverables:**
- New file: `shared/src/models/language_options.rs` with enums and structs ✅
- Modified: `shared/src/models/language.rs` to add `language_family()` method to `Language` enum ✅
- Modified: `shared/src/models/mod.rs` to export new module ✅

**Completion Status:** Phase 1 complete (commit a1915e6f)
- 14 new tests added (13 in language_options.rs, 1 in language.rs)
- All 151 workspace tests passing
- Zero clippy warnings/errors
- All functions under 20 lines
- Pure functions used throughout

**Files to Update:**
- CREATE: `/Users/demouser/Code/dialect-coach/shared/src/models/language_options.rs`
- MODIFY: `/Users/demouser/Code/dialect-coach/shared/src/models/language.rs`
- MODIFY: `/Users/demouser/Code/dialect-coach/shared/src/models/mod.rs`

**Implementation Steps:**

1. Create `shared/src/models/language_options.rs`:
   - Define `ArabicScript` enum (Naskh, Ruqa, Latin) with Default = Naskh
   - Define `JapaneseScript` enum (Romaji, OnlyKana, KanjiWithRuby, Kanji) with Default = KanjiWithRuby
   - Implement `Display` trait for both enums (for UI labels)
   - Define `LanguageOptions` struct with `arabic_script: Option<ArabicScript>` and `japanese_script: Option<JapaneseScript>`
   - Define `LanguageOption` enum (Arabic(ArabicScript), Japanese(JapaneseScript))
   - Implement `LanguageOptions::for_language()` method to get relevant option for a Language
   - Add Serialize/Deserialize derives for all types
   - Write unit tests for each type and method

2. Modify `shared/src/models/language.rs`:
   - Add `language_family()` method to `Language` enum returning `Language` (since Language already represents families)
   - The method simply returns `self.clone()` or `*self` since Language IS the family
   - Add test for `language_family()` method

3. Modify `shared/src/models/mod.rs`:
   - Add `pub mod language_options;`
   - Add `pub use language_options::*;`

**Verification:**
- Run `cargo test` in shared/ directory - 100% pass required
- Run `cargo clippy` - zero warnings/errors
- Verify all tests pass for new language_options module

**Phase Completion:**
- Update this document with completion status
- Add changes via git
- Commit: `git commit -m "Phase 1 (shared language models) complete"`

---

## Phase 2: UserState and UserMessageWithContext Integration

**Subagent:** kiss-code-generator

**Code Style Checklist:**
- [ ] Functions <20 lines, <10 if possible
- [ ] Helper functions instead of nested logic
- [ ] Pure functions where possible
- [ ] Tests for all new functions
- [ ] No TODOs or future-proofing
- [ ] No defensive coding

**Deliverables:**
- Modified: `shared/src/models/user_state.rs` to add `language_options` field
- Modified: `shared/src/models/message.rs` to add `language_option` field to `UserMessageWithContext`
- All existing tests updated to compile with new fields

**Files to Update:**
- MODIFY: `/Users/demouser/Code/dialect-coach/shared/src/models/user_state.rs`
- MODIFY: `/Users/demouser/Code/dialect-coach/shared/src/models/message.rs`

**Implementation Steps:**

1. Modify `shared/src/models/user_state.rs`:
   - Add `pub language_options: LanguageOptions` field to `UserState` struct
   - Update `UserState::new()` to initialize with `language_options: LanguageOptions::default()`
   - Add helper method `current_language_option(&self) -> Option<LanguageOption>` that calls `self.language_options.for_language(self.selected_language)`
   - Update all test construction code to include `language_options: LanguageOptions::default()`
   - Add test for `current_language_option()` method

2. Modify `shared/src/models/message.rs`:
   - Add `pub language_option: Option<LanguageOption>` field to `UserMessageWithContext` struct
   - Update `UserMessageWithContext::new()` to accept `language_option: Option<LanguageOption>` parameter
   - Update all test construction code to include language_option parameter (use `None`)

**Verification:**
- Run `cargo test` in shared/ directory - 100% pass required
- Run `cargo clippy` - zero warnings/errors
- Verify UserState serialization/deserialization tests pass

**Phase Completion:**
- Update this document with completion status
- Add changes via git
- Commit: `git commit -m "Phase 2 (UserState integration) complete"`

---

## Phase 3: Backend WebSocket Context Building

**Subagent:** kiss-code-generator

**Code Style Checklist:**
- [ ] Functions <20 lines, <10 if possible
- [ ] Helper functions instead of nested logic
- [ ] Pure functions where possible
- [ ] Tests for all new functions
- [ ] No TODOs or future-proofing
- [ ] No defensive coding

**Deliverables:**
- Modified: `backend/src/websocket/agents.rs` to pass `language_option` when building `UserMessageWithContext`
- WebSocket agent message handler now extracts and passes language option to agents

**Files to Update:**
- MODIFY: `/Users/demouser/Code/dialect-coach/backend/src/websocket/agents.rs`

**Implementation Steps:**

1. Find where `UserMessageWithContext` is constructed in `agents.rs`
2. Extract current language option from `UserState`:
   - Call `user_state.current_language_option()` to get `Option<LanguageOption>`
3. Pass `language_option` to `UserMessageWithContext::new()` call
4. Verify existing tests still compile and pass

**Verification:**
- Run `cargo test` in backend/ directory - 100% pass required
- Run `cargo clippy` - zero warnings/errors
- Run backend with `cargo run` and verify it compiles

**Phase Completion:**
- Update this document with completion status
- Add changes via git
- Commit: `git commit -m "Phase 3 (backend context building) complete"`

---

## Phase 4: Agent System Instructions

**Subagent:** modular-builder

**Code Style Checklist:**
- [ ] Functions <20 lines, <10 if possible
- [ ] Helper functions instead of nested logic
- [ ] Pure functions where possible
- [ ] Tests for all new functions
- [ ] No TODOs or future-proofing
- [ ] No defensive coding

**Deliverables:**
- Modified: All 5 agent files to include conditional system instructions based on `language_option`
- Helper function to generate language-specific instructions
- Agents format responses according to script preference

**Files to Update:**
- MODIFY: `/Users/demouser/Code/dialect-coach/backend/src/agent_service/response.rs`
- MODIFY: `/Users/demouser/Code/dialect-coach/backend/src/agent_service/learning.rs`
- MODIFY: `/Users/demouser/Code/dialect-coach/backend/src/agent_service/analysis.rs`
- CREATE: `/Users/demouser/Code/dialect-coach/backend/src/agent_service/language_instructions.rs` (helper)

**Implementation Steps:**

1. Create `backend/src/agent_service/language_instructions.rs`:
   - Function `build_language_instruction(language_option: &Option<LanguageOption>) -> String`
   - Match on language_option:
     - `Some(LanguageOption::Arabic(ArabicScript::Latin))` → "You must respond using romanized Arabic script. Use the appropriate romanization system for the user's dialect."
     - `Some(LanguageOption::Arabic(ArabicScript::Naskh))` → "You must respond using Arabic script in Naskh style."
     - `Some(LanguageOption::Arabic(ArabicScript::Ruqa))` → "You must respond using Arabic script in Ruq'a style."
     - `Some(LanguageOption::Japanese(JapaneseScript::Romaji))` → "You must respond using romanized Japanese (romaji)."
     - `Some(LanguageOption::Japanese(JapaneseScript::OnlyKana))` → "You must respond using only hiragana and katakana. Do not use kanji."
     - `Some(LanguageOption::Japanese(JapaneseScript::KanjiWithRuby))` → "You must respond using kanji with furigana. Format kanji with readings as: <ruby>漢字<rt>かんじ</rt></ruby>"
     - `Some(LanguageOption::Japanese(JapaneseScript::Kanji))` → "You must respond using standard kanji without annotations."
     - `None` → "" (empty string)
   - Write unit tests for each case

2. Modify `backend/src/agent_service/response.rs`:
   - Import `language_instructions::build_language_instruction`
   - In system prompt building, append result of `build_language_instruction(&context.language_option)`
   - Verify agent receives and uses language option

3. Modify `backend/src/agent_service/learning.rs`:
   - Import `language_instructions::build_language_instruction`
   - In system prompt building, append result of `build_language_instruction(&context.language_option)`

4. Modify `backend/src/agent_service/analysis.rs`:
   - Import `language_instructions::build_language_instruction`
   - In system prompt building, append result of `build_language_instruction(&context.language_option)`

5. Check for simple agents (translation, explanation):
   - Search codebase for translation/explanation agent files
   - Apply same pattern if they exist
   - If they use shared helper for system prompts, modify the helper instead

6. Update `backend/src/agent_service.rs` (or mod.rs) to export the new module

**Verification:**
- Run `cargo test` in backend/ directory - 100% pass required
- Run `cargo clippy` - zero warnings/errors
- Manual test: Send message with Arabic Latin option, verify romanized response

**Phase Completion:**
- Update this document with completion status
- Add changes via git
- Commit: `git commit -m "Phase 4 (agent language instructions) complete"`

---

## Phase 5: Frontend UserState Actions

**Subagent:** kiss-code-generator

**Code Style Checklist:**
- [ ] Functions <20 lines, <10 if possible
- [ ] Helper functions instead of nested logic
- [ ] Pure functions where possible
- [ ] Tests for all new functions
- [ ] No TODOs or future-proofing
- [ ] No defensive coding

**Deliverables:**
- Modified: `frontend/src/app/user_state/mod.rs` to add new action variants
- Modified: `frontend/src/app/user_state/callbacks.rs` to add callback functions
- UserState reducer handles language option updates

**Files to Update:**
- MODIFY: `/Users/demouser/Code/dialect-coach/frontend/src/app/user_state/mod.rs`
- MODIFY: `/Users/demouser/Code/dialect-coach/frontend/src/app/user_state/callbacks.rs`

**Implementation Steps:**

1. Modify `frontend/src/app/user_state/mod.rs`:
   - Add `SetArabicScript(ArabicScript)` to `UserStateAction` enum
   - Add `SetJapaneseScript(JapaneseScript)` to `UserStateAction` enum
   - In reducer, handle `SetArabicScript`:
     - Set `state.language_options.arabic_script = Some(action.0)`
   - In reducer, handle `SetJapaneseScript`:
     - Set `state.language_options.japanese_script = Some(action.0)`

2. Modify `frontend/src/app/user_state/callbacks.rs`:
   - Add `on_arabic_script_change(user_state: UseReducerHandle<OptionalUserState>) -> Callback<Event>`
   - Parse select value to `ArabicScript` enum
   - Dispatch `UserStateAction::SetArabicScript(parsed_value)`
   - Add `on_japanese_script_change(user_state: UseReducerHandle<OptionalUserState>) -> Callback<Event>`
   - Parse select value to `JapaneseScript` enum
   - Dispatch `UserStateAction::SetJapaneseScript(parsed_value)`

**Verification:**
- Run `trunk build` in frontend/ directory - must succeed
- Run `cargo clippy` in frontend/ - zero warnings/errors
- Verify frontend compiles without errors

**Phase Completion:**
- Update this document with completion status
- Add changes via git
- Commit: `git commit -m "Phase 5 (frontend actions) complete"`

---

## Phase 6: SettingsPanel UI Integration

**Subagent:** kiss-code-generator

**Code Style Checklist:**
- [ ] Functions <20 lines, <10 if possible
- [ ] Helper functions instead of nested logic
- [ ] Pure functions where possible
- [ ] Tests for all new functions (N/A for UI components)
- [ ] No TODOs or future-proofing
- [ ] No defensive coding

**Deliverables:**
- Modified: `frontend/src/components/settings_panel.rs` to show language-specific options
- UI shows appropriate dropdown based on current language
- Dropdowns correctly display current values and dispatch changes

**Files to Update:**
- MODIFY: `/Users/demouser/Code/dialect-coach/frontend/src/components/settings_panel.rs`

**Implementation Steps:**

1. Modify `frontend/src/components/settings_panel.rs`:
   - Import `ArabicScript`, `JapaneseScript`, `Language`
   - Import `on_arabic_script_change`, `on_japanese_script_change` from callbacks
   - After the "Conversation Style" section (around line 127), add new section:
     - Section title: "Display Options"
     - Check `us.selected_language`:
       - If `Language::Arabic`: Show Arabic script dropdown
         - Options: "Naskh", "Ruq'a", "Latin (Romanized)"
         - Values: "naskh", "ruqa", "latin"
         - Selected: `us.language_options.arabic_script.unwrap_or_default()`
         - Onchange: `on_arabic_script_change(user_state.clone())`
       - If `Language::Japanese`: Show Japanese script dropdown
         - Options: "Romaji", "Kana Only", "Kanji with Furigana", "Kanji"
         - Values: "romaji", "only_kana", "kanji_with_ruby", "kanji"
         - Selected: `us.language_options.japanese_script.unwrap_or_default()`
         - Onchange: `on_japanese_script_change(user_state.clone())`
       - Otherwise: Show nothing (empty html! {})

2. Ensure proper Yew HTML structure:
   - Use existing panel CSS classes (`panel-section`, `panel-field`, etc.)
   - Match styling of other sections
   - Add field-help text if helpful

**Verification:**
- Run `trunk serve` and visually inspect settings panel
- Verify Arabic script dropdown appears when Arabic language selected
- Verify Japanese script dropdown appears when Japanese language selected
- Verify no dropdown for Spanish/French/English
- Verify selections persist and update correctly

**Phase Completion:**
- Update this document with completion status
- Add changes via git
- Commit: `git commit -m "Phase 6 (UI integration) complete"`

---

## Phase 7: CSS Styling for Arabic Fonts

**Subagent:** kiss-code-generator

**Code Style Checklist:**
- [ ] Simple, minimal CSS
- [ ] Reuse existing patterns
- [ ] No over-engineering

**Deliverables:**
- Modified: Frontend CSS files to add Arabic font classes
- Message bubbles apply correct font based on language option

**Files to Research:**
- Search for existing CSS font handling
- Find where message content is rendered
- Identify where to add font classes

**Implementation Steps:**

1. Research current CSS structure:
   - Find where Ruq'a font is currently applied
   - Find message bubble CSS files
   - Determine how to conditionally apply font classes

2. Add CSS font classes:
   - `.arabic-naskh { font-family: "Times New Roman", "Noto Naskh Arabic", serif; }`
   - `.arabic-ruqa { font-family: "Aref Ruqaa", cursive; }` (should already exist)
   - `.arabic-latin { /* default font */ }`

3. Modify message bubble component:
   - Read `frontend/src/components/message_bubble.rs`
   - Add logic to apply font class based on message language option
   - Extract language option from message metadata
   - Apply appropriate CSS class to message content div

4. For Japanese ruby text:
   - Add CSS for ruby tags:
     ```css
     ruby { ruby-position: over; }
     rt { font-size: 0.5em; }
     ```
   - Ruby HTML tags will be rendered directly from agent response

**Verification:**
- Run `trunk serve`
- Send Arabic message with Naskh option - verify Naskh font
- Send Arabic message with Ruq'a option - verify Ruq'a font
- Send Japanese message with KanjiWithRuby - verify ruby tags render

**Phase Completion:**
- Update this document with completion status
- Add changes via git
- Commit: `git commit -m "Phase 7 (CSS styling) complete"`

---

## Phase 8: End-to-End Testing and Documentation

**Subagent:** General (orchestrator handles this)

**Deliverables:**
- All features tested manually
- Documentation updated
- All tests passing
- Clippy clean

**Testing Checklist:**

1. **Arabic Script Options:**
   - [ ] Select Egyptian Arabic
   - [ ] Change script to Naskh - verify agent responds in Arabic with Naskh font
   - [ ] Change script to Ruq'a - verify agent responds in Arabic with Ruq'a font
   - [ ] Change script to Latin - verify agent responds in romanized Arabic
   - [ ] Verify options persist across page refresh

2. **Japanese Script Options:**
   - [ ] Select Japanese Tokyo
   - [ ] Change script to Romaji - verify agent responds in romaji
   - [ ] Change script to Kana Only - verify agent responds without kanji
   - [ ] Change script to Kanji with Furigana - verify ruby tags render
   - [ ] Change script to Kanji - verify standard kanji response
   - [ ] Verify options persist across page refresh

3. **UI Behavior:**
   - [ ] Switch from Arabic to Spanish - verify display options section disappears
   - [ ] Switch from Spanish to Japanese - verify Japanese options appear
   - [ ] Verify dropdowns show correct current selection

4. **Persistence:**
   - [ ] Set Arabic Ruq'a option
   - [ ] Send message
   - [ ] Refresh page
   - [ ] Verify Ruq'a still selected

5. **Agent Integration:**
   - [ ] Verify Response agent receives language_option
   - [ ] Verify Learning agent receives language_option
   - [ ] Verify Analysis agent receives language_option

**Final Verification:**
- Run `cargo test --workspace` - 100% pass required
- Run `cargo clippy --workspace` - zero warnings/errors
- Run frontend with `trunk serve` - verify no console errors
- Run backend with `cargo run` - verify no startup errors

**Documentation Updates:**
- Mark all success criteria as complete in `language_specific_options.md`
- Update this implementation plan with final status

**Phase Completion:**
- Update status document
- Add all changes via git
- Commit: `git commit -m "Phase 8 (end-to-end testing) complete"`

---

## Success Criteria (from Specification)

- [ ] User can select script option in SettingsPanel
- [ ] Only current language's options are shown
- [ ] Options persist across sessions
- [ ] Arabic Latin option produces romanized agent responses
- [ ] Japanese Romaji option produces romanized agent responses
- [ ] Japanese Only Kana option produces kana-only agent responses
- [ ] Japanese Kanji w/ Ruby displays furigana (agent-generated)
- [ ] Arabic Naskh uses Naskh font family
- [ ] Arabic Ruq'a uses existing Ruq'a font
- [ ] All 5 agents respect language options
- [ ] All tests pass
- [ ] Clippy clean

## Implementation Notes

### Key Architecture Decisions

1. **Language as Family**: The `Language` enum already represents language families, so no separate `LanguageFamily` enum is needed. The spec's `language_family()` method simply returns the Language itself.

2. **Agent-Generated Ruby Tags**: Japanese furigana will be generated by agents as HTML `<ruby><rt>` tags, not parsed/added by frontend.

3. **Minimal CSS Changes**: Leverage existing CSS patterns and only add necessary font-family rules.

4. **No Migration Code**: User confirmed DB will be deleted, so no migration needed. Default values handle initialization.

### Testing Strategy

- **Unit Tests**: All pure functions in shared models
- **Integration Tests**: UserState serialization, agent context building
- **Manual Tests**: UI behavior, agent responses, persistence

### Potential Issues

1. **Ruby tag rendering**: Ensure Yew doesn't escape HTML in agent responses
2. **Font availability**: Naskh system fonts may vary across platforms
3. **Romanization quality**: Agents must produce dialect-appropriate romanization

### Extension Pattern

To add new language options in future:
1. Create new enum (e.g., `ChineseScript`)
2. Add field to `LanguageOptions` struct
3. Add variant to `LanguageOption` enum
4. Add case to `for_language()` method
5. Add UI section in SettingsPanel
6. Add instructions in `language_instructions.rs`
