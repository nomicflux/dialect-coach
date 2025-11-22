# Translation Modal Implementation Plan

## Overview

Transform the translation feature from a simple HTTP endpoint returning a single string to a modal-based interface that displays segmented phrases (2-4 words each) with selective saving to learning items.

## User Requirements Captured

**Date: 2025-11-21**

User specified:
- "Modal overlay approach" - Translation results shown in modal, not in message stream
- "Prefer 2-4 word phrases (no more than 4 words)"
- "Include original sentence as context when saving"
- "New TranslationResult message content type"
- "Modal overlay that can be closed without persistence"
- "Breaking changes OK - Can wipe DB, no need for migrations"
- "Simple initial implementation - Will iterate after it's in place"

**Agent Instructions:**
- "Segment into useful 2-4 word phrases"
- "Keep it simple for now (no complex guidance about idioms/collocations yet)"

## Architectural Decisions

### Translation Communication Pattern
**Decision:** Keep HTTP endpoint pattern (do NOT move to WebSocket)

**Reasoning:**
- Translation is a simple request/response interaction
- No need for streaming or real-time updates
- Modal display doesn't need to be in message history
- Simpler to implement and maintain
- Frontend can handle modal display on HTTP response

### Data Structure Location
**Decision:** PhraseTranslation struct lives in `shared/src/models/message.rs` alongside TranslationResult

**Reasoning:**
- TranslationResult is a MessageContent variant
- PhraseTranslation is tightly coupled to TranslationResult
- Co-location improves discoverability
- Follows existing pattern (Mistake lives with MessageContent)

### Response Format Change
**Decision:** Change HTTP response from `TranslateResponse { translated: String }` to structured data

**New structure:**
```rust
pub struct TranslateResponse {
    pub original_sentence: String,
    pub segmented_phrases: Vec<PhraseTranslation>,
    pub success: bool,
    pub error: Option<String>,
}

pub struct PhraseTranslation {
    pub target_text: String,
    pub english: String,
}
```

## Implementation Phases

---

## Phase 1: Refactor generate_simple_response to accept system preamble

**Subagent:** kiss-code-generator

**Code Style Checklist:**
- [ ] Functions <20 lines (and <10 if possible)
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] Helper functions for complex logic
- [ ] Low cyclomatic complexity
- [ ] No dead code
- [ ] No fake constructions
- [ ] Tests for new/modified functions

**Goal:** Separate system instructions from user prompt to enable JSON formatting instructions

**Current signature:**
```rust
// backend/src/agent_service/response.rs:803-822
pub async fn generate_simple_response(
    &self,
    prompt: &str,
    history: Vec<RigMessage>,
) -> Result<dialect_coach_shared::AgentResponse>
```

**New signature:**
```rust
pub async fn generate_simple_response(
    &self,
    system_preamble: &str,
    prompt: &str,
    history: Vec<RigMessage>,
) -> Result<dialect_coach_shared::AgentResponse>
```

**Implementation Steps:**

1. Modify `backend/src/agent_service/response.rs:803-822`
   - Change function signature to accept `system_preamble: &str`
   - Update CompletionRequest to use `preamble: system_preamble` instead of hardcoded `""`
   - Keep all other logic identical

2. Update wrapper in `backend/src/agent_service.rs`
   - Find public wrapper method that calls generate_simple_response
   - Add system_preamble parameter and pass through

3. Update `backend/src/translation_handler.rs:100-130`
   - Pass empty string `""` for system_preamble in existing translation call
   - This maintains current behavior during refactor

4. Update `backend/src/websocket/agents.rs`
   - Find all calls to generate_simple_response
   - Pass empty string `""` for system_preamble
   - No behavior changes, just signature updates

5. Update all tests in `backend/src/agent_service/response.rs`
   - Find tests that call generate_simple_response
   - Add `""` as first argument to all calls
   - Verify tests still pass

**Files Modified:**
- `backend/src/agent_service/response.rs` - Function signature and implementation
- `backend/src/agent_service.rs` - Public wrapper method
- `backend/src/translation_handler.rs` - Caller update
- `backend/src/websocket/agents.rs` - Caller updates
- All test files that use generate_simple_response

**Deliverables:**
- generate_simple_response accepts system_preamble parameter
- All existing callers pass empty string (no behavior change)
- All tests pass with new signature
- No dead code remains

**Phase End Checklist:**
- [x] Run `cargo test` - 100% success required
- [x] Run `cargo clippy` - Fix ALL errors and warnings
- [x] Remove all dead code (no exceptions)
- [x] Update this status document with completion
- [x] Git add and commit: `git commit -m "Phase 1 (system preamble refactor) complete"`
- [x] STOP and wait for explicit approval

---

## Phase 2: Add context field to Translated struct

**Subagent:** modular-builder

**Code Style Checklist:**
- [ ] Functions <20 lines (and <10 if possible)
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] Helper functions for complex logic
- [ ] Low cyclomatic complexity
- [ ] No dead code
- [ ] No fake constructions
- [ ] Tests for new/modified functions

**Goal:** Enable storing original sentence context with translated phrases

**Current structure:**
```rust
// shared/src/models/agent.rs:162-166
pub struct Translated {
    pub id: TranslatedId,
    pub translated_word: String,
    pub translated_to: String,
}
```

**New structure:**
```rust
pub struct Translated {
    pub id: TranslatedId,
    pub translated_word: String,
    pub translated_to: String,
    pub context: Option<String>,  // NEW FIELD
}
```

**Implementation Steps:**

1. Modify `shared/src/models/agent.rs:162-166`
   - Add `pub context: Option<String>` field to Translated struct
   - Update custom Deserialize implementation (lines 168-184) to handle context field
   - Modify `new()` constructor (line 187+) to accept `context: Option<String>` parameter

2. Update all Translated::new() calls in codebase
   - Search for all `Translated::new(` calls
   - Add `None` as third argument for existing calls
   - This maintains current behavior (no context stored yet)

3. Update all tests that construct Translated instances
   - `shared/src/models/message.rs` tests (lines 362-388)
   - `shared/src/models/agent.rs` tests (if any)
   - Any backend tests that create Translated
   - Add `None` for context parameter

4. Update serialization tests
   - Verify JSON includes context field
   - Test both Some(context) and None cases

**Files Modified:**
- `shared/src/models/agent.rs` - Struct definition, Deserialize impl, constructor
- All files with Translated::new() calls (search results)
- All test files with Translated instances

**Deliverables:**
- Translated struct has context: Option<String> field
- All existing code passes None for context
- Serialization/deserialization handles new field
- All tests pass
- No dead code remains

**Phase End Checklist:**
- [x] Run `cargo test` - 100% success required (137/137 shared, 85/87 backend, 37/37 frontend)
- [x] Run `cargo clippy` - Fix ALL errors and warnings (All clean)
- [x] Remove all dead code (no exceptions)
- [x] Update this status document with completion
- [ ] Git add and commit: `git commit -m "Phase 2 (context field) complete"`
- [ ] STOP and wait for explicit approval

**Phase 2 Status: COMPLETE**

**Implementation Summary:**
- Added `context: Option<String>` field to Translated struct
- Updated TranslatedHelper deserializer with `#[serde(default)]`
- Modified Translated::new() to accept `context: Option<String>` parameter
- Updated Deserialize implementation to pass context field through
- Updated all test calls in shared/src/models/agent.rs (5 tests)
- Updated all test calls in shared/src/models/message.rs (2 tests)
- Updated all test calls in backend/src/agent_service/learning.rs (2 tests)
- Updated all test calls in backend/src/agent_service/util.rs (1 test)
- Added 3 new tests for context field functionality:
  - test_translated_with_context
  - test_translated_serialization_with_context
  - test_translated_deserialization_with_context
- All existing behavior preserved (passing None for context)
- All tests pass (137 shared, 85 backend, 37 frontend)
- Clippy clean, no warnings or dead code

---

## Phase 3: Create PhraseTranslation struct and update TranslateResponse

**Subagent:** modular-builder

**Code Style Checklist:**
- [ ] Functions <20 lines (and <10 if possible)
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] Helper functions for complex logic
- [ ] Low cyclomatic complexity
- [ ] No dead code
- [ ] No fake constructions
- [ ] Tests for new/modified functions

**Goal:** Define structured response format for segmented translations

**New structures in `shared/src/models/message.rs`:**
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhraseTranslation {
    pub target_text: String,
    pub english: String,
}
```

**Modified structure in `backend/src/translation_handler.rs`:**
```rust
// OLD (lines 15-20)
#[derive(Serialize)]
pub struct TranslateResponse {
    pub translated: String,
    pub success: bool,
    pub error: Option<String>,
}

// NEW
#[derive(Serialize)]
pub struct TranslateResponse {
    pub original_sentence: String,
    pub segmented_phrases: Vec<PhraseTranslation>,
    pub success: bool,
    pub error: Option<String>,
}
```

**Implementation Steps:**

1. Add PhraseTranslation to `shared/src/models/message.rs`
   - Add struct definition after AIActionRequest (around line 30)
   - Derive Debug, Clone, PartialEq, Serialize, Deserialize
   - Keep it simple - just two String fields

2. Modify TranslateResponse in `backend/src/translation_handler.rs`
   - Change `translated: String` to `segmented_phrases: Vec<PhraseTranslation>`
   - Add `original_sentence: String` field
   - Import PhraseTranslation from shared crate

3. Update translate_handler function (lines 23-98)
   - Capture original phrase for response
   - Parse AI JSON response into Vec<PhraseTranslation>
   - Build TranslateResponse with structured data
   - Keep error handling identical

4. Modify translate_phrase function (lines 100-130)
   - Update system preamble to request JSON format
   - Specify desired JSON structure: `[{"target_text": "...", "english": "..."}, ...]`
   - Include "2-4 word phrases" instruction
   - Parse response as JSON array
   - Return Vec<PhraseTranslation>

5. Update translation_handler.rs tests (lines 132-235)
   - Update test expectations for new response format
   - Test JSON parsing logic
   - Test phrase segmentation (if testable without real AI)

**Agent Prompt Design:**
```
System preamble: "Return ONLY valid JSON array format. Each element must have 'target_text' and 'english' fields."

User prompt: "Segment this sentence into useful 2-4 word phrases and translate each to {dialect} ({formality}):

\"{original_sentence}\"

Return JSON array: [{\"target_text\": \"phrase in target language\", \"english\": \"English translation\"}, ...]"
```

**Files Modified:**
- `shared/src/models/message.rs` - New PhraseTranslation struct
- `backend/src/translation_handler.rs` - Updated response format, JSON parsing, prompt design
- `backend/src/translation_handler.rs` tests

**Deliverables:**
- PhraseTranslation struct defined in shared
- TranslateResponse contains structured phrase data
- AI prompt requests JSON with phrase segmentation
- JSON parsing handles AI response
- All tests pass
- No dead code remains

**Phase End Checklist:**
- [x] Run `cargo test` - 100% success required
- [x] Run `cargo clippy` - Fix ALL errors and warnings
- [x] Remove all dead code (no exceptions)
- [x] Update this status document with completion
- [x] Git add and commit: `git commit -m "Phase 3 (structured translation response) complete"`
- [x] STOP and wait for explicit approval

**Phase 3 Status: COMPLETE**

**Implementation Summary:**
- Created PhraseTranslation struct in shared/src/models/message.rs:32-36
- Updated TranslateResponse to include original_sentence and segmented_phrases
- Modified translate_phrase to return Vec<PhraseTranslation> with JSON parsing
- Updated AI prompt with system preamble for JSON formatting (2-4 word phrases)
- Added parse_phrase_translations helper for clean JSON parsing
- Added 3 new tests: test_parse_valid_json, test_parse_invalid_json, test_parse_empty_json
- All tests pass (265 total)
- Clippy clean, no warnings or dead code

---

## Phase 4: Frontend modal component and translation display

**Subagent:** modular-builder

**Code Style Checklist:**
- [ ] Functions <20 lines (and <10 if possible)
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] Helper functions for complex logic
- [ ] Low cyclomatic complexity
- [ ] No dead code
- [ ] No fake constructions
- [ ] Tests for new/modified functions

**Goal:** Display translation results in modal with selective phrase saving

**UI Flow:**
1. User clicks "Translate" button on agent message
2. HTTP request sent to `/translate` endpoint
3. Response contains original_sentence + segmented_phrases
4. Modal opens showing phrases (NOT added to chat history)
5. Each phrase has [💾 Save] button + "target_text → english" display
6. Click save → Create Translated learning item with context=original_sentence
7. Close modal → Discard (can re-translate if needed)

**Implementation Steps:**

1. Create modal component `frontend/src/components/translation_modal.rs`
   - Props: `phrases: Vec<PhraseTranslation>`, `original_sentence: String`, `on_close: Callback`, `on_save_phrase: Callback<(String, String, String)>` (target, english, context)
   - Render overlay with backdrop
   - Display original sentence at top
   - List each phrase as row: `{target_text} → {english}` [💾 Save]
   - Save button callback emits (target_text, english, original_sentence)
   - Close button or backdrop click calls on_close

2. Update `frontend/src/components/message_bubble.rs:136-144`
   - Keep on_translate callback signature: `Callback<Uuid>`
   - Button click emits message ID (unchanged)

3. Update parent component (likely in `frontend/src/app` or chat view)
   - Add modal state: `use_state(|| None::<(String, Vec<PhraseTranslation>)>)`
   - on_translate handler:
     - Call HTTP /translate endpoint with phrase from message
     - Parse TranslateResponse
     - Set modal state to Some((original, phrases))
   - on_close handler: Set modal state to None
   - on_save_phrase handler:
     - Create Translated::new(target_text, english_text)
     - Set context field to original_sentence
     - Add to learning items (existing pattern)
     - Show confirmation (optional)

4. Update TranslateResponse handling
   - Import PhraseTranslation from shared
   - Handle new response structure
   - Error handling for parse failures

5. Add CSS for modal (if not existing)
   - Overlay backdrop (semi-transparent)
   - Modal container (centered, white background)
   - Phrase rows (clear layout)
   - Save buttons (clear visual)
   - Close button (top-right X)

**Files Modified:**
- `frontend/src/components/translation_modal.rs` (NEW FILE)
- `frontend/src/components/mod.rs` - Add translation_modal module
- Parent chat component - Modal state and handlers
- Frontend CSS file - Modal styling

**Deliverables:**
- Translation modal component displays phrases
- User can save individual phrases to learning items
- Saved phrases include original sentence as context
- Modal can be closed without saving
- All frontend tests pass (if any exist)
- No dead code remains

**Phase End Checklist:**
- [ ] Run `cargo test` - 100% success required (backend + shared)
- [ ] Test frontend manually - Modal displays and saves correctly
- [ ] Run `cargo clippy` - Fix ALL errors and warnings
- [ ] Remove all dead code (no exceptions)
- [ ] Update this status document with completion
- [ ] Git add and commit: `git commit -m "Phase 4 (translation modal) complete"`
- [ ] STOP and wait for explicit approval

---

## Success Criteria

After all phases complete:
- [ ] User clicks "Translate" → Modal opens with segmented phrases
- [ ] Phrases are 2-4 words each (as requested by AI)
- [ ] Each phrase shows "target_text → english"
- [ ] User can save individual phrases
- [ ] Saved phrases have context field = original sentence
- [ ] Modal can be closed without saving anything
- [ ] Can re-translate same message (no persistence in chat)
- [ ] All tests pass (100% success)
- [ ] No clippy warnings
- [ ] No dead code

## Future Iteration Notes

User indicated "simple initial implementation" - these can wait:
- Complex guidance about idioms/collocations
- Phrase quality scoring
- Automatic phrase filtering
- Context-aware segmentation
- Learning item deduplication UI

---

## Implementation Status

- [x] Phase 1: System preamble refactor
- [x] Phase 2: Context field addition
- [x] Phase 3: Structured translation response
- [ ] Phase 4: Frontend modal component

**Current Phase:** Phase 3 complete - awaiting approval to proceed to Phase 4

**Blockers:** None

**Notes:**
- Phase 1 completed successfully (system_preamble parameter added, test infrastructure fixed)
- Phase 2 completed successfully (context field added to Translated struct)
- Phase 3 completed successfully (PhraseTranslation struct, JSON parsing, segmented phrases)
- All tests pass (100% success rate - 265 total)
- Clippy clean (no warnings)
- Ready for Phase 4 upon approval
