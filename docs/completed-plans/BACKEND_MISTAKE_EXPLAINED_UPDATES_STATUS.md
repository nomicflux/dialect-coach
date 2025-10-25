# Backend: Agent-Driven Mistake and Explanation Updates - Implementation Status

## Overview
Implementation of dynamic learning item score updates through agent analysis.

**Start Date**: 2025-10-24

---

## Phase 1: Data Structure Changes (shared crate)

### Status: ✅ Complete

### Tasks:
- [x] 1.1: Add IDs to Mistake and Explained - Added `MistakeId` and `ExplainedId` type aliases, deterministic UUID generation via `new()` constructors
- [x] 1.2: Create LearningItemScore struct - Added with score: i8 field, 2 tests pass
- [x] 1.3: Create AgentAnalysis struct - Added with HashMap fields, 3 tests pass
- [x] 1.4: Update AgentResponse - Added analysis: Option<AgentAnalysis>, 2 new tests pass, backward compatibility verified

### Files Modified:
- `shared/src/models/agent.rs`

### Tests: 47 total tests pass (7 new tests added)

---

## Phase 2: WebSocket Message Protocol (shared crate)

### Status: ✅ Complete

### Tasks:
- [x] 2.1: Create UserMessageWithContext struct - Added with message, past_mistakes, past_explained fields, 3 tests pass

### Files Modified:
- `shared/src/models/message.rs`

### Tests: 50 total tests pass (3 new tests added)

---

## Phase 3: Analysis Agent (backend crate)

### Status: ✅ Complete (Corrected Approach)

**Mistake Made**: Initially coupled analysis orchestration into agent service, breaking modularity by modifying `generate_response()` signature and adding parallel execution logic at wrong layer.

**Correction**: Agent service provides standalone methods only. Orchestration belongs in WebSocket layer (Phase 4).

### Tasks:
- [x] 3.1: Add analysis prompt helpers - `analysis_agent_prompt()`, `format_mistakes_for_analysis()`, `format_explained_for_analysis()`
- [x] 3.2: Add standalone `generate_analysis()` method
- [x] 3.3: REVERTED - Removed parallel orchestration helpers, restored original `generate_response()` signature

### Files Modified:
- `backend/src/agent_service.rs`

### Key Methods Added:
- `pub async fn generate_analysis(dialect, conversation_history, mistakes, explained) -> Result<AgentAnalysis>`
- Pure helper functions for prompt formatting

### Tests: All tests pass (50 shared tests, build compiles cleanly)

### Key Learning:
Orchestration logic belongs at the WebSocket layer, not in the agent service. Agent service should provide independent methods that can be composed by callers. This maintains modularity and keeps the system compilable at each step.

---

## Phase 4: WebSocket Handler Updates (backend crate)

### Status: ✅ Complete

### Tasks:
- [x] 4.1: Update message parsing to expect UserMessageWithContext instead of Message
- [x] 4.2: Add parallel agent orchestration helper - `run_agents_parallel()` uses `tokio::join!` to run both agents concurrently
- [x] 4.3: Verify WebSocket changes compile and work

### Files Modified:
- `backend/src/websocket.rs`

### Key Changes:

**Added `run_agents_parallel()` helper**:
- Checks if learning items are present (non-empty arrays)
- If no learning items: calls only `generate_response()`
- If learning items present: runs both agents in parallel with `tokio::join!`
- Combines results by populating `analysis` field on `AgentResponse`
- On analysis failure: returns error to fail entire request (retry logic happens at agent service level)

**Updated `call_agent_and_respond()`**:
- Changed parameter from `&Message` to `&UserMessageWithContext`
- Delegates to `run_agents_parallel()` for orchestration
- Extracts `message` field when passing to success/error handlers

**Updated `process_user_message()`**:
- Changed parameter from `Message` to `UserMessageWithContext`
- Extracts `message` field for session ID and user text

**Updated `create_recv_task()`**:
- Changed parsing from `Message` to `UserMessageWithContext`
- Enhanced logging to show count of mistakes and explained items received

### Tests: All tests pass (50 shared tests + 19 other tests)

### Build Status: ✅ Compiles cleanly with no errors

---

## Phase 5: Frontend - ID Tracking (frontend crate)

### Status: ✅ Complete

### Tasks:
- [x] 5.1: ID tracking already complete - Mistake and Explained have deterministic IDs from Phase 1
- [x] 5.2: Send learning items to backend via UserMessageWithContext

### Files Modified:
- `frontend/src/services/websocket.rs`
- `frontend/src/app.rs`

### Key Changes:

**Updated WebSocket Service (`websocket.rs`)**:
- Changed import to include `UserMessageWithContext`
- Updated `send_message()` to accept `&UserMessageWithContext` instead of `&Message`
- Enhanced logging to show count of mistakes and explained items being sent

**Updated App Layer (`app.rs`)**:
- Added `extract_learning_items()` helper function to convert `Vec<LearningItem>` into separate `Vec<Mistake>` and `Vec<Explained>`
- Updated `on_send_message()` callback to:
  - Accept both `app_state` and `ui_state` parameters
  - Extract learning items from UI state
  - Build `UserMessageWithContext` with message and learning items
  - Send through WebSocket
- Updated call sites to pass `ui_state` parameter

### Tests: All tests pass (50 shared + 19 other)

### Build Status: ✅ Compiles cleanly with no errors

### Design Notes:
IDs were already tracked since Phase 1 added deterministic UUIDs to `Mistake` and `Explained` structs. The `LearningItem` wrapper in frontend inherits these IDs through the enum variants.

---

## Phase 6: Frontend - Score Updates and Accomplishments (frontend crate)

### Status: ✅ Complete

### Tasks:
- [x] 6.1: Update UIState to handle score updates via AgentAnalysis
- [x] 6.2: Wire score updates through app.rs message handling
- [x] 6.3: Update learning panel to partition accomplishments (score == 100)
- [x] 6.4: Display accomplishments in separate section

### Files Modified:
- `frontend/src/app/app_state.rs`
- `frontend/src/app.rs`
- `frontend/src/components/learning_panel.rs`

### Key Changes:

**UIState Updates (`app_state.rs`)**:
- Added `UIStateAction::UpdateScores(AgentAnalysis)` action
- Implemented `apply_score_updates()` helper function that:
  - Iterates through learning items
  - Matches IDs with AgentAnalysis HashMap
  - Updates score field (max with 0 to prevent negatives)

**App Layer (`app.rs`)**:
- Updated message handling to extract `AgentAnalysis` from agent responses
- Dispatches `UIStateAction::UpdateScores()` when analysis is present
- Added logging for score updates

**Learning Panel (`learning_panel.rs`)**:
- Partitioned items: `score == 100` moves to accomplishments, others stay in learning
- Added "Accomplishments" section header with count badge
- Items in accomplishments displayed with visual distinction

### Tests: All tests pass

### Build Status: ✅ Compiles cleanly with no errors

---

## Phase 7: Tooltips (frontend crate)

### Status: ✅ Complete

### Tasks:
- [x] 7.1: Implement CSS-based hover tooltips
- [x] 7.2: Wire tooltip content from learning items
- [x] 7.3: Fix tooltip text contrast
- [x] 7.4: Fix tooltip z-index to appear above header

### Files Modified:
- `frontend/src/components/learning_panel.rs`
- `frontend/styles/components/learning_panel.css`

### Key Changes:

**Learning Panel Component (`learning_panel.rs`)**:
- Wrapped item content in `.item-with-tooltip` div
- Added `.tooltip-text` span for hover display
- Tooltip content:
  - Mistakes: Shows `mistake_category` as tooltip
  - Explanations: Shows `explanation` field as tooltip

**Tooltip Styles (`learning_panel.css`)**:
- Implemented pure CSS hover tooltips (no HTML title attribute)
- Used hard-coded colors (#1a1a1a background, #ffffff text) for proper contrast
- Added downward-pointing arrow using ::after pseudo-element
- Positioned absolutely above item (bottom: 125%)
- Visibility/opacity transition on hover
- Fixed z-index: Changed learning panel from 100 to 200 to appear above header

### Tests: All tests pass

### Build Status: ✅ Compiles cleanly with no errors

---

## Phase 8: Additional Features and Bug Fixes

### Status: ✅ Complete

### Tasks:
- [x] 8.1: Custom JSON deserialization for flexible Claude responses
- [x] 8.2: Update Corrective mode to ignore punctuation/capitalization
- [x] 8.3: Implement TTS auto-play toggle (default OFF)
- [x] 8.4: Add comprehensive logging for debugging

### Files Modified:
- `shared/src/models/agent.rs`
- `backend/src/agent_service.rs`
- `frontend/src/app/app_state.rs`
- `frontend/src/app.rs`
- `frontend/src/components/speech_controls.rs`
- `frontend/styles/components/composer.css`

### Key Changes:

**Custom Deserialization (`shared/src/models/agent.rs`)**:
- Added custom `Deserialize` for `Mistake`:
  - Accepts JSON with or without `id` field
  - Generates deterministic UUID using `uuid::Uuid::new_v5()` when missing
  - Uses `specific_mistake` content as namespace seed
- Added custom `Deserialize` for `Explained`:
  - Same pattern as Mistake
  - Uses `explanation` content as namespace seed
- Added custom `Deserialize` for `LearningItemScore`:
  - Accepts both integer (`5`) and object (`{"score": 5}`) formats
  - Uses untagged enum helper for flexible parsing

**Prompt Updates (`agent_service.rs`)**:
- Updated Corrective mode prompt at line 103:
  - Added "IGNORE missing punctuation and capitalization - this is casual chat"
  - Focuses on actual language errors (spelling, grammar, dialectal usage)
- Added comprehensive logging:
  - Raw JSON responses at line 492 (main agent)
  - Analysis step logging at lines 213, 230, 236, 238
  - Detailed error logging for JSON parsing failures

**TTS Toggle Feature**:
- Added `tts_enabled: bool` to UIState (default: false)
- Added `UIStateAction::ToggleTTS` action
- Gated auto-play in `app.rs` at line 251: `if msg.participant_id != "user" && (*usc).tts_enabled`
- Added toggle button in `speech_controls.rs` (🔊/🔇 icons)
- Styled with `.tts-enabled` and `.tts-disabled` CSS classes
- Manual TTS replay still works regardless of toggle state

### Tests: All tests pass

### Build Status: ✅ Compiles cleanly with no errors

---

## User Agreements

**Date: 2025-10-24**

User specified:
- "Create separate message type" - Use `UserMessageWithContext` instead of modifying Message struct
- "Retry once, then fail the whole request" - Analysis agent gets one retry, then fail entire request
- "In learning panel" - Accomplishments displayed within existing learning panel
- "Full explanation" - Tooltips show explanation field for Explained items

---

## Issues Encountered

None yet.
