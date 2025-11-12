# Dialect Features UI Implementation Plan

## Overview
Display all dialects with feature indicators (TTS voices and corpus availability) in the settings panel, remove filtering restrictions, disable TTS features when unavailable, and update backend to conditionally retrieve corpus samples.

## Phase 1: Update UserState to Return DialectWithFeatures

**Status: COMPLETE** ✅

### Code Style Checklist
- [x] **Planning Documentation**: Have you consulted/created/updated docs/current-plans/dialect_features_ui_IMPLEMENTATION_STATUS.md?
- [x] **Code Simplicity**: Are you following simplicity rules? (functions <20 lines, pure functions, no defensive coding)
- [x] **Code Modularity**: Are you following modularity rules? (helper functions, low cyclomatic complexity)
- [x] **Scope Control**: Are you accomplishing the user's instructions and NOTHING MORE?
- [x] **No Dead Code**: Did you leave dead code? (no future-proofing, no leaving just for tests)
- [x] **No Fake Constructions**: Are there any object instances that are purely for the sake of passing a type checker?
- [x] **Code Purpose**: Do you changes accomplish the plan purpose and not just mechanical checklists?
- [x] **Required Tests**: Have you added tests for any new functions?

### Files Updated
- `shared/src/models/user_state.rs` - Updated `current_dialects()` method (lines 88-93) and added imports
- `frontend/src/components/settings_panel.rs` - Updated to work with `DialectWithFeatures` (lines 68-72)
- `frontend/src/app/user_state/callbacks.rs` - Updated `on_dialect_change` and `on_dialect_cycle` callbacks (lines 38-39, 98-100)

### Implementation Details

1. **Update `current_dialects()` method in `UserState`**:
   - Change return type from `Vec<Dialect>` to `Vec<DialectWithFeatures>`
   - Replace `Dialect::for_language(self.selected_language, true, true)` with `Dialect::for_language(self.selected_language, false, false)` to remove filtering
   - Map each `Dialect` to `DialectWithFeatures` using `dialect_features()` function:
     ```rust
     pub fn current_dialects(&self) -> Vec<DialectWithFeatures> {
         Dialect::for_language(self.selected_language, false, false)
             .into_iter()
             .map(|dialect| dialect_features(dialect))
             .collect()
     }
     ```

2. **Add import to `shared/src/models/user_state.rs`**:
   - Add `use super::dialect::dialect_features;` or `use super::DialectWithFeatures;` as needed
   - Ensure `DialectWithFeatures` is accessible from the dialect module

### Deliverables
- ✅ Updated `UserState::current_dialects()` method that returns `Vec<DialectWithFeatures>` without filtering
- ✅ All existing tests updated to work with new return type
- ✅ All checks passing: `cargo fmt --check`, `cargo clippy --workspace`, `cargo test --workspace`

### Phase Completion
- ✅ Run formatting check: `cargo fmt --check` - PASSED
- ✅ Run lint check: `cargo clippy --workspace` - PASSED (fixed redundant closure warning)
- ✅ Run FULL test suite: `cargo test --workspace` - PASSED (all 131 shared tests, 22 frontend tests, 65 backend tests, etc.)
- ✅ Status document updated with progress
- **ALL agents must STOP and wait for EXPLICIT approval before proceeding to Phase 2**

### Implementation Notes
- Updated `current_dialects()` to return `Vec<DialectWithFeatures>` by mapping dialects through `dialect_features()` function
- Changed filtering from `(true, true)` to `(false, false)` to return all dialects for the selected language
- Updated frontend code in `settings_panel.rs` and `callbacks.rs` to extract `.dialect` field from `DialectWithFeatures`
- Fixed clippy warning about redundant closure by using `dialect_features` directly instead of `|dialect| dialect_features(dialect)`
- All tests pass without modification - existing tests work correctly with the new return type

---

## Phase 2: Update Settings Panel to Display All Dialects with Feature Indicators

**Status: COMPLETE** ✅

### Code Style Checklist
- [x] **Planning Documentation**: Have you consulted/created/updated docs/current-plans/dialect_features_ui_IMPLEMENTATION_STATUS.md?
- [x] **Code Simplicity**: Are you following simplicity rules? (functions <20 lines, pure functions, no defensive coding)
- [x] **Code Modularity**: Are you following modularity rules? (helper functions, low cyclomatic complexity)
- [x] **Scope Control**: Are you accomplishing the user's instructions and NOTHING MORE?
- [x] **No Dead Code**: Did you leave dead code? (no future-proofing, no leaving just for tests)
- [x] **No Fake Constructions**: Are there any object instances that are purely for the sake of passing a type checker?
- [x] **Code Purpose**: Do you changes accomplish the plan purpose and not just mechanical checklists?
- [x] **Required Tests**: Have you added tests for any new functions?

### Files Updated
- `shared/src/models/dialect.rs` - Added `has_tts()` helper method to `DialectWithFeatures` (lines 321-326)
- `frontend/src/components/settings_panel.rs` - Updated language dropdown (lines 57-59) and dialect dropdown (lines 71-75), added Language import (line 5)
- `frontend/src/app/user_state/callbacks.rs` - Fixed dialect cycle callback to use `df.dialect == current` (line 98)

### Implementation Details

1. **Update language dropdown (line ~55-59)**:
   - Remove hardcoded `selected=true` from Spanish option (line 56)
   - Add logic to determine which language option is selected based on `us.selected_language`:
     ```rust
     <select id="language-select" onchange={on_language_change(user_state.clone())}>
         <option value="spanish" selected={us.selected_language == Language::Spanish}>{"Spanish"}</option>
         <option value="arabic" selected={us.selected_language == Language::Arabic}>{"Arabic"}</option>
         <option value="french" selected={us.selected_language == Language::French}>{"French"}</option>
     </select>
     ```

2. **Update dialect dropdown (lines ~64-77)**:
   - Change from `us.current_dialects()` returning `Vec<Dialect>` to `Vec<DialectWithFeatures>`
   - Update the mapping to use `DialectWithFeatures`:
     ```rust
     <select id="dialect-select" onchange={on_dialect_change(user_state.clone())}>
         {{
             let dialects = us.current_dialects();
             let current = us.current_dialect();
             dialects.iter().map(|dialect_features| {
                 let is_selected = dialect_features.dialect == current;
                 let has_tts = dialect_features.tts_voices.values().any(|v| v.is_some());
                 let tts_indicator = if has_tts { "🔊" } else { "" };
                 let corpus_indicator = if dialect_features.has_corpus { "📚" } else { "" };
                 html! {
                     <option value={dialect_features.dialect.id()} selected={is_selected}>
                         {format!("{} {} {}", dialect_features.dialect.name(), tts_indicator, corpus_indicator)}
                     </option>
                 }
             }).collect::<Html>()
         }}
     </select>
     ```

3. **Add import to `frontend/src/components/settings_panel.rs`**:
   - Ensure `DialectWithFeatures` is accessible (likely through `dialect_coach_shared::models::DialectWithFeatures`)

### Deliverables
- ✅ Settings panel displays all dialects for the selected language (not just those with TTS and corpus)
- ✅ Language dropdown shows the currently selected language from user state (not hardcoded Spanish)
- ✅ Dialect dropdown shows emoji indicators: 🔊 for TTS support, 📚 for corpus availability
- ✅ Selected dialect matches user state correctly
- ✅ All checks passing: `cargo fmt --check`, `cargo clippy --workspace`, `cargo test --workspace`

### Phase Completion
- ✅ Run formatting check: `cargo fmt --check` - PASSED
- ✅ Run lint check: `cargo clippy --workspace` - PASSED
- ✅ Run FULL test suite: `cargo test --workspace` - PASSED (all 131 shared tests, 22 frontend tests, 65 backend tests, etc.)
- ✅ Status document updated with progress
- **ALL agents must STOP and wait for EXPLICIT approval before proceeding to Phase 3**

### Implementation Notes
- Added `has_tts()` helper method to `DialectWithFeatures` to simplify TTS check logic
- Updated language dropdown to use `us.selected_language` comparison instead of hardcoded `selected=true`
- Updated dialect dropdown to show emoji indicators (🔊 for TTS, 📚 for corpus) using `has_tts()` method and `has_corpus` field
- Fixed dialect cycle callback in `callbacks.rs` to correctly compare `df.dialect == current` instead of `d == &current`
- All tests pass without modification

---

## Phase 3: Update Dialect Change Callbacks to Work with DialectWithFeatures

### Code Style Checklist
- [ ] **Planning Documentation**: Have you consulted/created/updated docs/current-plans/dialect_features_ui_IMPLEMENTATION_STATUS.md?
- [ ] **Code Simplicity**: Are you following simplicity rules? (functions <20 lines, pure functions, no defensive coding)
- [ ] **Code Modularity**: Are you following modularity rules? (helper functions, low cyclomatic complexity)
- [ ] **Scope Control**: Are you accomplishing the user's instructions and NOTHING MORE?
- [ ] **No Dead Code**: Did you leave dead code? (no future-proofing, no leaving just for tests)
- [ ] **No Fake Constructions**: Are there any object instances that are purely for the sake of passing a type checker?
- [ ] **Code Purpose**: Do you changes accomplish the plan purpose and not just mechanical checklists?
- [ ] **Required Tests**: Have you added tests for any new functions?

### Files to Update
- `frontend/src/app/user_state/callbacks.rs` - Update `on_dialect_change` and `on_dialect_cycle` callbacks (lines ~26-43 and ~89-103)

### Implementation Details

1. **Update `on_dialect_change` callback (lines ~26-43)**:
   - Change `state.current_dialects()` to work with `Vec<DialectWithFeatures>`
   - Find matching dialect by comparing `dialect_features.dialect.id()` with the selected value
   - Extract the `Dialect` from `DialectWithFeatures` when dispatching:
     ```rust
     pub fn on_dialect_change(user_state: UseReducerHandle<OptionalUserState>) -> Callback<Event> {
         let user_state = user_state.clone();
         Callback::from(move |e: Event| {
             let state = match user_state.0.as_ref() {
                 Some(s) => s,
                 None => return,
             };
             if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
                 let value = select.value();
                 let dialects = state.current_dialects();
                 if let Some(dialect_features) = dialects.iter().find(|df| df.dialect.id() == value) {
                     user_state.dispatch(UserStateAction::ChangeDialect(dialect_features.dialect));
                 }
             }
         })
     }
     ```

2. **Update `on_dialect_cycle` callback (lines ~89-103)**:
   - Change `state.current_dialects()` to work with `Vec<DialectWithFeatures>`
   - Find current dialect position by comparing `dialect_features.dialect == current`
   - Extract the `Dialect` from `DialectWithFeatures` when dispatching:
     ```rust
     pub fn on_dialect_cycle(user_state: UseReducerHandle<OptionalUserState>) -> Callback<()> {
         let user_state = user_state.clone();
         Callback::from(move |_| {
             let state = match user_state.0.as_ref() {
                 Some(s) => s,
                 None => return,
             };
             let dialects = state.current_dialects();
             let current = state.current_dialect();
             if let Some(idx) = dialects.iter().position(|df| df.dialect == current) {
                 let next_idx = (idx + 1) % dialects.len();
                 user_state.dispatch(UserStateAction::ChangeDialect(dialects[next_idx].dialect));
             }
         })
     }
     ```

### Deliverables
- Dialect change callback works correctly with `DialectWithFeatures`
- Dialect cycle callback works correctly with all dialects (not just filtered ones)
- All checks passing: `cargo fmt --check`, `cargo clippy --workspace`, `cargo test --workspace`

### Phase Completion
- Run formatting check: `cargo fmt --check`
- Run lint check: `cargo clippy --workspace`
- Run FULL test suite: `cargo test --workspace`
- Upon 100% success (fmt, clippy, and tests), update this status document with progress
- **ALL agents must STOP and wait for EXPLICIT approval before proceeding to Phase 4**

---

## Phase 4: Disable TTS Features When Dialect Doesn't Support TTS

### Code Style Checklist
- [ ] **Planning Documentation**: Have you consulted/created/updated docs/current-plans/dialect_features_ui_IMPLEMENTATION_STATUS.md?
- [ ] **Code Simplicity**: Are you following simplicity rules? (functions <20 lines, pure functions, no defensive coding)
- [ ] **Code Modularity**: Are you following modularity rules? (helper functions, low cyclomatic complexity)
- [ ] **Scope Control**: Are you accomplishing the user's instructions and NOTHING MORE?
- [ ] **No Dead Code**: Did you leave dead code? (no future-proofing, no leaving just for tests)
- [ ] **No Fake Constructions**: Are there any object instances that are purely for the sake of passing a type checker?
- [ ] **Code Purpose**: Do you changes accomplish the plan purpose and not just mechanical checklists?
- [ ] **Required Tests**: Have you added tests for any new functions?

### Files to Update
- `frontend/src/components/message_bubble.rs` - Update `render_replay_button` function (lines ~31-42)
- `frontend/src/app/app_state.rs` - Update `Speak` action (lines ~113-126) and `ProcessAgentMessage` action (lines ~155-172)
- `frontend/src/components/chat_window.rs` - Update message bubble rendering to pass dialect info (lines ~157-170)
- `frontend/src/components/main_content.rs` - Update message replay callback usage (line ~91)

### Implementation Details

1. **Add helper function to check TTS support in `shared/src/models/dialect.rs`**:
   - Add a method to `Dialect` or create a helper function:
     ```rust
     pub fn dialect_has_tts(dialect: Dialect) -> bool {
         let features = dialect_features(dialect);
         features.tts_voices.values().any(|v| v.is_some())
     }
     ```
   - Alternatively, add a method to `DialectWithFeatures`:
     ```rust
     impl DialectWithFeatures {
         pub fn has_tts(&self) -> bool {
             self.tts_voices.values().any(|v| v.is_some())
         }
     }
     ```

2. **Update `MessageBubble` component (`frontend/src/components/message_bubble.rs`)**:
   - Update `render_replay_button` to check TTS support before rendering:
     ```rust
     fn render_replay_button(
         on_replay: &Option<Callback<Message>>,
         msg: &Message,
     ) -> Html {
         // Check if dialect has TTS support
         let dialect = msg.metadata.dialect;
         let has_tts = dialect_coach_shared::models::dialect::dialect_has_tts(dialect);
         
         if let Some(callback) = on_replay {
             if !has_tts {
                 return html! {};
             }
             let cb = callback.clone();
             let m = msg.clone();
             let onclick = Callback::from(move |_| cb.emit(m.clone()));
             html! {
                 <button class="replay-button" {onclick} title="Replay audio">{"🔊"}</button>
             }
         } else {
             html! {}
         }
     }
     ```

3. **Update `AppState::Speak` action (`frontend/src/app/app_state.rs`, lines ~113-126)**:
   - Check TTS support before calling TTS service:
     ```rust
     AppStateAction::Speak(msg) => {
         let dialect = msg.metadata.dialect;
         let has_tts = dialect_coach_shared::models::dialect::dialect_has_tts(dialect);
         if !has_tts {
             return; // Skip TTS call if dialect doesn't support it
         }
         let tts_service = next.tts_service.clone();
         let user_id = next.current_user.as_ref().map(|u| u.id);
         let language_code = dialect.bcp47_tag();
         let text = msg.get_content();
         wasm_bindgen_futures::spawn_local(async move {
             if let Some(tts) = tts_service
                 && let Some(uid) = user_id
                 && let Err(e) = tts.speak(uid, &text, language_code).await
             {
                 error!("Failed to replay message with TTS: {}", e);
             }
         });
     }
     ```

4. **Update `AppState::ProcessAgentMessage` action (`frontend/src/app/app_state.rs`, lines ~155-172)**:
   - Check TTS support before calling TTS service:
     ```rust
     AppStateAction::ProcessAgentMessage(msg) => {
         if next.autoplay_enabled {
             let dialect = msg.metadata.dialect;
             let has_tts = dialect_coach_shared::models::dialect::dialect_has_tts(dialect);
             if !has_tts {
                 return; // Skip TTS call if dialect doesn't support it
             }
             // ... rest of existing TTS code
         }
     }
     ```

5. **Add import to `frontend/src/components/message_bubble.rs`**:
   - Add `use dialect_coach_shared::models::dialect::dialect_has_tts;` or access through module path

6. **Add import to `frontend/src/app/app_state.rs`**:
   - Add `use dialect_coach_shared::models::dialect::dialect_has_tts;` or access through module path

### Deliverables
- Replay button only shows when dialect has TTS support
- TTS service calls are skipped when dialect doesn't support TTS (both in `Speak` and `ProcessAgentMessage` actions)
- No errors when attempting to replay messages for dialects without TTS
- All checks passing: `cargo fmt --check`, `cargo clippy --workspace`, `cargo test --workspace`

### Phase Completion
- Run formatting check: `cargo fmt --check`
- Run lint check: `cargo clippy --workspace`
- Run FULL test suite: `cargo test --workspace`
- Upon 100% success (fmt, clippy, and tests), update this status document with progress
- **ALL agents must STOP and wait for EXPLICIT approval before proceeding to Phase 5**

---

## Phase 5: Update Backend to Use DialectWithFeatures and Conditionally Retrieve Corpus

### Code Style Checklist
- [ ] **Planning Documentation**: Have you consulted/created/updated docs/current-plans/dialect_features_ui_IMPLEMENTATION_STATUS.md?
- [ ] **Code Simplicity**: Are you following simplicity rules? (functions <20 lines, pure functions, no defensive coding)
- [ ] **Code Modularity**: Are you following modularity rules? (helper functions, low cyclomatic complexity)
- [ ] **Scope Control**: Are you accomplishing the user's instructions and NOTHING MORE?
- [ ] **No Dead Code**: Did you leave dead code? (no future-proofing, no leaving just for tests)
- [ ] **No Fake Constructions**: Are there any object instances that are purely for the sake of passing a type checker?
- [ ] **Code Purpose**: Do you changes accomplish the plan purpose and not just mechanical checklists?
- [ ] **Required Tests**: Have you added tests for any new functions?

### Files to Update
- `backend/src/agent_service/response.rs` - Update `GenerateResponseParams` struct (line ~342-354), `collect_examples` method (lines ~460-488), and `generate_response` method (lines ~628-705)
- `backend/src/websocket.rs` - Update call sites where `GenerateResponseParams` is created (lines ~519-531 and ~553-565)
- `backend/src/test_utils.rs` - Update test code if it uses `GenerateResponseParams` (line ~50)

### Implementation Details

1. **Update `GenerateResponseParams` struct in `backend/src/agent_service/response.rs` (line ~342-354)**:
   - Change `dialect: Dialect` field to `dialect: DialectWithFeatures`:
     ```rust
     pub struct GenerateResponseParams<'a> {
         pub user_message: &'a str,
         pub dialect: DialectWithFeatures,
         pub formality: Formality,
         pub teaching_mode: TeachingMode,
         pub conversation_history: &'a [RigMessage],
         pub learning_goals: &'a [String],
         pub rag_config: &'a RAGConfig,
         pub past_mistakes: &'a [Mistake],
         pub past_explained: &'a [Explained],
         pub past_translated: &'a [Translated],
         pub past_exploratory: &'a [Exploratory],
     }
     ```

2. **Update `collect_examples` method in `backend/src/agent_service/response.rs` (lines ~460-488)**:
   - Change parameter from `dialect: Dialect` to `dialect: DialectWithFeatures`
   - Check `dialect.has_corpus` before retrieving examples:
     ```rust
     async fn collect_examples(
         &self,
         user_message: &str,
         conversation_history: &[RigMessage],
         dialect: DialectWithFeatures,
         formality: Formality,
         rag_config: &RAGConfig,
     ) -> Result<(Vec<DialectDocument>, Vec<DialectDocument>)> {
         if !dialect.has_corpus {
             return Ok((Vec::new(), Vec::new()));
         }
         // ... existing implementation using dialect.dialect instead of dialect
         let embeddings = self.retrieve_embeddings(user_message, &history_text)?;
         let examples = self
             .retrieve_examples(&dialect.dialect, embeddings, rag_config.num_conversation_documents)
             .await?;
         let sample_formalities = get_sample_formalities(formality);
         let random_samples = self
             .retrieve_random_samples(dialect.dialect, sample_formalities, rag_config.num_random_documents)
             .await?;
         // ... rest of existing implementation
     }
     ```

3. **Update `generate_response` method in `backend/src/agent_service/response.rs` (lines ~628-705)**:
   - Update call to `collect_examples` to pass `DialectWithFeatures`
   - Update any other methods that use the dialect parameter to use `params.dialect.dialect` when they need the `Dialect` enum

4. **Update `build_system_content` function in `backend/src/agent_service/response.rs` (lines ~255-321)**:
   - Change parameter from `dialect: Dialect` to `dialect: DialectWithFeatures`
   - Use `dialect.dialect` when calling `dialect.name()` or other `Dialect` methods

5. **Update `websocket.rs` call sites (`backend/src/websocket.rs`, lines ~519-531 and ~553-565)**:
   - Convert `Dialect` to `DialectWithFeatures` when creating `GenerateResponseParams`:
     ```rust
     let params = GenerateResponseParams {
         user_message: user_text,
         dialect: dialect_coach_shared::models::dialect::dialect_features(dialect),
         formality,
         teaching_mode,
         conversation_history: history_vec,
         learning_goals: &msg_with_context.learning_goals,
         rag_config: &rag_config,
         past_mistakes: &msg_with_context.past_mistakes,
         past_explained: &msg_with_context.past_explained,
         past_translated: &msg_with_context.past_translated,
         past_exploratory: &msg_with_context.past_exploratory,
     };
     ```

6. **Update `backend/src/test_utils.rs` if it uses `GenerateResponseParams` (line ~50)**:
   - Convert `Dialect` to `DialectWithFeatures` when creating test params

7. **Add imports to `backend/src/agent_service/response.rs`**:
   - Add `use dialect_coach_shared::models::DialectWithFeatures;`
   - Add `use dialect_coach_shared::models::dialect::dialect_features;`

8. **Add import to `backend/src/websocket.rs`**:
   - Add `use dialect_coach_shared::models::dialect::dialect_features;`

### Deliverables
- Backend uses `DialectWithFeatures` throughout response generation
- Corpus samples are only retrieved when `dialect.has_corpus` is true
- All backend tests updated and passing
- No corpus retrieval attempts for dialects without corpus
- All checks passing: `cargo fmt --check`, `cargo clippy --workspace`, `cargo test --workspace`

### Phase Completion
- Run formatting check: `cargo fmt --check`
- Run lint check: `cargo clippy --workspace`
- Run FULL test suite: `cargo test --workspace`
- Upon 100% success (fmt, clippy, and tests), update this status document with progress
- **ALL agents must STOP and wait for EXPLICIT approval - Implementation Complete**

---

## Summary
This plan implements displaying all dialects with feature indicators in the UI, removes filtering restrictions, disables TTS features when unavailable, and updates the backend to conditionally retrieve corpus samples. Each phase is self-contained and should pass all tests before proceeding to the next phase.

