# WebSocket Refactoring - Implementation Status

**Date Created:** 2025-10-24
**Status:** In Progress

## User Requirements (Exact Quote)

**Date: 2025-10-24**

User stated:
> "The handle_socket does not follow ANY of our code style guidelines."
> "Keep it modular - decompose into helper functions of ideally no more than 10 lines, certainly no more than 20"
> "Decompose into separate, testable functionality"
> "Simplify - if code isn't actually needed, remove it."

## Current State (Before)

**File:** `backend/src/websocket.rs`
- `handle_socket`: 244 lines (lines 29-273)
- 6+ levels of nesting
- Business logic in async closures (untestable)
- 24 eprintln + 12 tracing (redundant logging)
- 1 trivial test

## Implementation Plan

### Phase 1: Extract Pure Helper Functions ✅

1. ✅ `trim_history(history: &mut Vec<String>, max_size: usize)` - Lines 19-23
2. ✅ `create_error_message(...) -> Message` - Lines 25-41
3. ✅ `serialize_and_send(msg: &Message, tx: &Sender) -> Result<()>` - Lines 43-54
4. ✅ `update_user_history(state, session_id, user_text) -> Vec<String>` - Lines 56-68
5. ✅ `add_agent_to_history(state, session_id, agent_text)` - Lines 70-75

### Phase 2: Extract Business Logic (In Progress)

6. ⏳ `handle_agent_success(state, parsed_msg, agent_response, tx) -> Result<()>`
   - Add agent to history
   - Create response message
   - Send to client
   - ~15 lines

7. ⏳ `handle_agent_error(parsed_msg, error, tx) -> Result<()>`
   - Create error message
   - Send to client
   - ~8 lines

8. ⏳ `process_user_message(state, parsed_msg, tx) -> Result<()>`
   - Validate dialect
   - Update history
   - Call agent
   - Handle success/error
   - ~20 lines (orchestration)

### Phase 3: Refactor handle_socket

9. ⏳ Extract `create_send_task()` if needed
10. ⏳ Extract `create_recv_task()` if needed
11. ⏳ Simplify `handle_socket` to call helpers

### Phase 4: Cleanup

12. ⏳ Remove all 24 eprintln statements
13. ⏳ Remove `let state = state_clone` line

### Phase 5: Re-analyze

14. ⏳ **Read handle_socket line-by-line again**
15. ⏳ **Identify remaining violations of <20 line guideline**
16. ⏳ **Extract any remaining complex logic into helpers**
17. ⏳ **Verify final line count and nesting levels**

## Progress

### Completed ✅

**1. trim_history()** - Lines 19-23
- TEST: `test_trim_history()` ✅

**2. create_error_message()** - Lines 25-41
- TEST: `test_create_error_message()` ✅

**3. serialize_and_send()** - Lines 43-54
- TEST: `test_serialize_and_send()` ✅

**4. update_user_history()** - Lines 56-68
- TEST: `test_update_user_history()` ✅

**5. add_agent_to_history()** - Lines 70-75
- TEST: `test_add_agent_to_history()` ✅

**6. create_agent_response_message()** - Lines 77-92 (PURE)
- Creates Message from AgentResponse + metadata
- TEST: `test_create_agent_response_message()` ✅

**7. handle_agent_success()** - Lines 94-112
- Orchestrates: add to history, create message, send
- ~18 lines (within guideline)

**8. handle_agent_error()** - Lines 114-129
- Orchestrates: create error message, send
- ~15 lines (within guideline)

### In Progress ⏳

Still TODO: Extract main message processing logic

## Test Results

Current: 6 tests passing
- ✅ test_trim_history
- ✅ test_create_error_message
- ✅ test_serialize_and_send
- ✅ test_update_user_history
- ✅ test_add_agent_to_history
- ✅ test_message_parsing (existing)

## Files Modified

- `backend/src/websocket.rs` - In progress refactoring

## Next Steps

1. Extract handle_agent_success + test
2. Extract handle_agent_error + test
3. Extract process_user_message + test
4. Refactor handle_socket
5. Remove eprintln statements
6. **RE-ANALYZE handle_socket for remaining issues**
