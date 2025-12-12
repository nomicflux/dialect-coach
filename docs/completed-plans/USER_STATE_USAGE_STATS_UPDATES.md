# UserState Usage Stats Updates

This document analyzes ALL production code locations where `usage_stats` is updated on `UserState`.

## Update Functions

### Backend - Usage Tracker Module

**`backend/src/usage_tracker.rs`**

The following functions modify `usage_stats`:

1. **`add_response_usage()`** (Line 30-38)
   - Adds response agent usage events
   - Prunes old events outside the rolling window
   - Modifies: `stats.response_events`

2. **`add_analysis_usage()`** (Line 41-49)
   - Adds analysis agent usage events
   - Prunes old events outside the rolling window
   - Modifies: `stats.analysis_events`

3. **`add_tts_usage()`** (Line 52-59)
   - Adds TTS usage event
   - Prunes old events outside the rolling window
   - Modifies: `stats.tts_events`

## Production Code Update Locations

### 1. Response Agent Usage (WebSocket Handler)

**`backend/src/websocket.rs`**

**Function**: `update_and_save_usage()` (Line 145-179)
- Updates `user_state.usage_stats` with response and/or analysis agent usage
- Saves the updated UserState to persistence
- Called from two locations:

#### Location 1: Line 269 - Response Only (No Analysis)
```rust
let (agent_response, response_usage) = state.agent.generate_response(&params).await?;
update_and_save_usage(state, user_state, response_usage, vec![], now).await;
```
**Context**: Called in `run_agents_parallel()` when there are no learning items (no mistakes, explained, translated, or exploratory items). Only the response agent is called.

#### Location 2: Line 311 - Response + Analysis
```rust
let (agent_response, response_usage) = response_result?;
match analysis_result {
    Ok((analysis, analysis_usage)) => {
        let mut agent_response = agent_response;
        agent_response.analysis = Some(analysis);
        update_and_save_usage(state, user_state, response_usage, analysis_usage, now).await;
        Ok(agent_response)
    }
}
```
**Context**: Called in `run_agents_parallel()` when there are learning items. Both response and analysis agents run in parallel, and usage stats are updated with both sets of usage data.

**Implementation Details**:
- Line 159: Calls `add_response_usage()` to update response events
- Lines 161-166: Calls `add_analysis_usage()` if analysis usage is not empty
- Line 175: Saves the updated UserState to persistence

### 2. TTS Usage (TTS Handler)

**`backend/src/tts_handler.rs`**

**Function**: `track_tts_usage()` (Line 25-33)
- Updates `user_state.usage_stats` with TTS usage
- Saves the updated UserState to persistence

**Called from**: Line 102 in `synthesize_handler()`
```rust
let response = state
    .service
    .synthesize(request)
    .await
    .map_err(TtsErrorResponse::from_tts_error)?;

track_tts_usage(&state, user_state, characters).await;
```
**Context**: Called after successful TTS synthesis. Updates usage stats with the number of characters synthesized.

**Implementation Details**:
- Line 31: Calls `add_tts_usage()` to add TTS usage event
- Line 32: Saves the updated UserState to persistence

## Summary

There are **3 production code locations** where `usage_stats` is updated on `UserState`:

1. **Response agent only** (`backend/src/websocket.rs:269`)
   - When user has no learning items
   - Updates: `response_events`

2. **Response + Analysis agents** (`backend/src/websocket.rs:311`)
   - When user has learning items
   - Updates: `response_events` and `analysis_events`

3. **TTS synthesis** (`backend/src/tts_handler.rs:102`)
   - After successful TTS synthesis
   - Updates: `tts_events`

All updates:
- Add new usage events to the respective event vectors
- Prune old events outside the 24-hour rolling window
- Save the updated UserState to persistence immediately after updating

## Update Flow

1. **Agent calls** → Generate usage data (tokens, timestamps)
2. **Usage tracker functions** → Add events and prune old ones
3. **Persistence save** → Save updated UserState to database

The usage stats are **only** updated **after** the operation completes successfully. On errors, usage stats are **not** updated.

### Error Handling

**Agent Calls:**
- If `generate_response()` fails (line 267), the function returns early with `?` operator - usage stats are **not** updated
- If `generate_analysis()` fails (line 315-318), only the response usage is updated, analysis usage is **not** updated
- If rate limit check fails (line 246-252), the function returns early - usage stats are **not** updated
- If any validation fails (e.g., illegal characters at line 236-238), the function returns early - usage stats are **not** updated

**TTS Synthesis:**
- If `synthesize()` fails (line 96-100), the function returns early with `?` operator - usage stats are **not** updated
- If rate limit check fails (line 94), the function returns early - usage stats are **not** updated

**Result**: Usage stats only track **successful** operations. Failed operations (errors, rate limits, validation failures) do not increment usage stats.

## Frontend Access

**Yes, `usage_stats` is sent to the frontend and displayed to the user.**

### How it's sent:

1. **Backend → Frontend**: When UserState is loaded via WebSocket
   - **Location**: `backend/src/websocket.rs:490`
   - **Message**: `UserStateMessage::LoadResponse(Some(user_state))`
   - The entire `UserState` object (including `usage_stats`) is serialized and sent to the frontend

2. **Frontend receives**: `frontend/src/services/user_state_websocket.rs:126`
   - Deserializes `LoadResponse(Some(user_state))`
   - Calls `on_load.emit(user_state)` with the complete UserState

3. **Frontend stores**: `frontend/src/app/app_state/callbacks.rs:36`
   - Dispatches `ReplaceUserState(state.clone())` which replaces the entire UserState
   - The `usage_stats` field is now available in the frontend's UserState

### How it's displayed:

**`frontend/src/components/usage_footer.rs`**
- **Component**: `UsageFooter` displays usage statistics to the user
- **Props**: Receives `usage_stats: UsageStats` from UserState
- **Displayed data**:
  - Response Agent: number of calls and total tokens
  - Analysis Agent: number of calls and total tokens
  - Text-to-Speech: number of calls and total characters
- **Location in UI**: `frontend/src/components/main_content.rs:182`
  - Rendered as a footer component that can be collapsed/expanded

The usage stats are **live** - they reflect the current state of the UserState, which is updated on the backend after each agent call or TTS synthesis, and then sent back to the frontend when UserState is loaded.

