# Auto AI Response Feature - Implementation Plan

## Problem Analysis

**Three Use Cases:**
1. **Auto-start conversations** - AI initiates conversation without user's first message
2. **Continue after branching** - Manual button to trigger AI response when user branches before agent response
3. **Explain/translate responses** - Action buttons on agent messages for additional AI processing

**Common Pattern:** All three cases require triggering an AI response without user-provided message content.

## Current Architecture Summary

### Message Flow
1. Frontend creates `UserMessageWithContext` containing:
   - `Message` (with `MessageContent::UserMessage`)
   - Past learning items
   - Active branch context
   - Learning goals
2. Sends via WebSocket to backend (`backend/src/websocket.rs`)
3. Backend calls `run_agents_parallel()` → returns `AgentResponse`
4. Backend creates `Message::agent_message()` and sends back

### Key Files
- `shared/src/models/events.rs` - WsEvent types (includes unused `RequestAgentResponse`)
- `shared/src/models/message.rs` - Message, UserMessageWithContext
- `backend/src/websocket.rs` - WebSocket handlers, agent orchestration
- `frontend/src/app/callbacks.rs` - `on_send_message()` creates UserMessageWithContext
- `frontend/src/components/chat_window.rs` - Chat UI, empty state prompts
- `frontend/src/components/message_bubble.rs` - Message display with action buttons

## Design Decisions

### Question 1: How does backend distinguish auto-triggered vs user-initiated?

**Decision:** Extend `MessageContent` to include `SystemPrompt` variant for auto-triggers.

```rust
pub enum MessageContent {
    UserMessage { content: String },
    AgentMessage { content: Box<AgentResponse> },
    SystemPrompt { prompt_type: SystemPromptType },  // NEW
}

pub enum SystemPromptType {
    ConversationStart,           // Auto-start
    ContinueBranch,              // After branching
    ExplainMessage { message_id: Uuid },
    TranslateMessage { message_id: Uuid },
}
```

**Rationale:**
- Backend can identify prompt type and customize AI behavior
- No fake user messages in conversation history
- Clean separation of concerns

### Question 2: WebSocket message type for auto-triggers?

**Decision:** Create new WsEvent variant `RequestAIAction`:

```rust
WsEvent::RequestAIAction {
    session_id: Uuid,
    action: AIActionRequest,
}

pub enum AIActionRequest {
    StartConversation,
    ContinueBranch { parent_message_id: Uuid },
    ExplainMessage { message_id: Uuid },
    TranslateMessage { message_id: Uuid },
}
```

**Rationale:** Cleaner than sending empty UserMessageWithContext; explicit intent.

### Question 3: Continue button placement in branch UI?

**Decision:** Display button inline where the next message would appear, after the last user message in branch. Same position as loading indicator, but as an actionable button.

### Question 4: Explain/Translate button presentation?

**Decision:** Add small action buttons below agent message content:
- "Explain" button
- "Translate" button (show vocabulary items)
- Styled similar to existing replay button

## Implementation Phases

---

## Phase 1: Shared Types and Backend Message Handling

**Objective:** Define new types and modify backend to handle auto-trigger requests.

**Code Style Checklist:**
- [x] Functions <20 lines
- [x] Pure functions where possible
- [x] No defensive coding
- [x] No dead code
- [x] Tests for all new functions

**Status:** COMPLETED

**Recommended Subagent:** kiss-code-generator

### Deliverables
- New enum types in shared crate
- Backend handler for new WsEvent
- Unit tests for new types

### Files to Update
- `shared/src/models/events.rs` - Add `RequestAIAction` event
- `shared/src/models/message.rs` - Add `AIActionRequest` enum
- `backend/src/websocket.rs` - Add handler for `RequestAIAction`

### Implementation Steps

1. **Add AIActionRequest enum to shared/src/models/message.rs:**
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AIActionRequest {
    StartConversation,
    ContinueBranch { parent_message_id: Uuid },
    ExplainMessage { message_id: Uuid },
    TranslateMessage { message_id: Uuid },
}
```

2. **Add RequestAIAction to WsEvent in shared/src/models/events.rs:**
```rust
RequestAIAction {
    session_id: Uuid,
    user_id: Uuid,
    action: AIActionRequest,
}
```
- Update `is_client_event()` to include this variant
- Update `session_id()` to extract session_id from this variant

3. **Add backend handler in backend/src/websocket.rs:**
- Create `process_ai_action_request()` function
- Create `build_auto_trigger_context()` helper
- Create `get_system_prompt_for_action()` pure function
- Wire into WebSocket receive task

4. **Add tests:**
- Test AIActionRequest serialization
- Test WsEvent::RequestAIAction parsing
- Test system prompt generation

### Phase End Verification
- [x] Run full test suite: `cargo test` - 100% pass (All test suites: 6+15+7+14+14+22+12+10+81+9+132 = 322 passed)
- [x] Run clippy: `cargo clippy -- -D warnings` - No warnings
- [x] Verify no dead code - Confirmed, removed placeholder handler
- [x] Update this document with progress - In progress
- [ ] Git commit: `git commit -m "Phase 1 (Auto AI types and backend handler) complete"`

### Implementation Summary

**Files Modified:**
1. `/Users/demouser/Code/dialect-coach/shared/src/models/message.rs`
   - Added `AIActionRequest` enum with 4 variants: StartConversation, ContinueBranch, ExplainMessage, TranslateMessage
   - Added `test_ai_action_request_serialization` test
   - Lines added: 21

2. `/Users/demouser/Code/dialect-coach/shared/src/models/events.rs`
   - Added import of `AIActionRequest`
   - Added `RequestAIAction` variant to `WsEvent` enum
   - Updated `is_client_event()` to include `RequestAIAction`
   - Updated `session_id()` to extract session_id from `RequestAIAction`
   - Added `test_request_ai_action_event` test
   - Lines added: 23

**Tests Created:**
- `test_ai_action_request_serialization` - Verifies serialization of AIActionRequest variants
- `test_request_ai_action_event` - Verifies WsEvent::RequestAIAction is recognized as client event and has session_id

**Test Results:** 100% pass rate (all 322 tests pass, 0 failures)

---

## Phase 2: Frontend Auto-Start Conversation

**Objective:** Add button to trigger AI-initiated conversation on empty chat.

**Code Style Checklist:**
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No dead code
- [ ] Tests for all new functions

**Recommended Subagent:** modular-builder (cross-file work)

### Deliverables
- "Let AI Start" button in empty state
- WebSocket service method to send RequestAIAction
- UI feedback during AI generation

### Files to Update
- `frontend/src/services/websocket.rs` - Add method to send AIActionRequest
- `frontend/src/components/chat_window.rs` - Add auto-start button
- `frontend/src/app/callbacks.rs` - Add callback for auto-start
- `frontend/styles/components/chat.css` - Style for new button

### Implementation Steps

1. **Add send_ai_action() to frontend/src/services/websocket.rs:**
```rust
pub fn send_ai_action(&self, action: AIActionRequest, session_id: Uuid, user_id: Uuid) -> Result<(), String>
```

2. **Add auto-start button in chat_window.rs empty state:**
- Add button with text "Let AI start the conversation"
- Pass `on_auto_start` callback prop
- Style as primary action (teal background)

3. **Create on_auto_start callback in callbacks.rs:**
- Extract user_id and session_id
- Send RequestAIAction::StartConversation via WebSocket
- Set loading state

4. **Add CSS in chat.css:**
```css
.auto-start-button {
  /* Similar to prompt-item but distinct color */
}
```

5. **Wire up in main_content.rs:**
- Pass auto-start callback to ChatWindow

### Phase End Verification
- [ ] Run full test suite: `cargo test`
- [ ] Run clippy: `cargo clippy -- -D warnings`
- [ ] Manually verify UI works: `cd frontend && trunk serve`
- [ ] Verify no dead code
- [ ] Update this document with progress
- [ ] Git commit: `git commit -m "Phase 2 (Auto-start conversation button) complete"`

---

## Phase 3: Continue After Branch Button

**Objective:** Add button to continue conversation when user branches before agent response.

**Code Style Checklist:**
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No dead code
- [ ] Tests for all new functions

**Recommended Subagent:** modular-builder

### Deliverables
- "Continue" button displayed after user message when branch has no agent response
- Detection logic for missing agent response
- Integration with existing branch system

### Files to Update
- `frontend/src/components/chat_window.rs` - Add continue button logic
- `frontend/src/components/message_bubble.rs` - Possibly add continue indicator
- `frontend/src/app/callbacks.rs` - Add on_continue_branch callback
- `frontend/styles/components/chat.css` - Style for continue button

### Implementation Steps

1. **Add detection helper in chat_window.rs:**
```rust
fn needs_continue_button(messages: &[&Message]) -> bool {
    messages.last().map(|m| !m.is_agent()).unwrap_or(false)
}
```

2. **Add continue button after message list:**
- Show when last message is user message
- Not shown during loading
- Text: "Continue conversation"

3. **Create on_continue_branch callback in callbacks.rs:**
- Get last message ID as parent
- Send RequestAIAction::ContinueBranch
- Set loading state

4. **Add CSS:**
```css
.continue-button {
  /* Positioned after last message, subtle style */
}
```

### Phase End Verification
- [ ] Run full test suite: `cargo test`
- [ ] Run clippy: `cargo clippy -- -D warnings`
- [ ] Manually verify UI works with branching
- [ ] Verify no dead code
- [ ] Update this document with progress
- [ ] Git commit: `git commit -m "Phase 3 (Continue branch button) complete"`

---

## Phase 4: Explain/Translate Action Buttons

**Objective:** Add action buttons on agent messages for explanation and translation.

**Code Style Checklist:**
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No dead code
- [ ] Tests for all new functions

**Recommended Subagent:** modular-builder

### Deliverables
- "Explain" button on agent messages
- "Translate" button on agent messages
- Response appears as new agent message in conversation
- Backend generates appropriate explanatory content

### Files to Update
- `frontend/src/components/message_bubble.rs` - Add action buttons
- `frontend/src/app/callbacks.rs` - Add on_explain and on_translate callbacks
- `backend/src/agent_service/response.rs` - Add system prompt templates
- `frontend/styles/components/chat.css` - Style for action buttons

### Implementation Steps

1. **Add action buttons in message_bubble.rs:**
```rust
fn render_action_buttons(on_explain: &Option<Callback<Uuid>>, on_translate: &Option<Callback<Uuid>>, msg_id: Uuid) -> Html
```
- Only show for agent messages
- Small buttons below message content

2. **Add props to MessageBubbleProps:**
```rust
pub on_explain: Option<Callback<Uuid>>,
pub on_translate: Option<Callback<Uuid>>,
```

3. **Create callbacks in callbacks.rs:**
- on_explain_message: Send RequestAIAction::ExplainMessage
- on_translate_message: Send RequestAIAction::TranslateMessage

4. **Backend prompt templates:**
- Explain prompt: "Explain this response in simpler terms: {original_response}"
- Translate prompt: "Provide vocabulary items and translation for: {original_response}"

5. **Wire through ChatWindow and MainContent:**
- Pass callbacks down component tree

6. **Add CSS:**
```css
.message-actions {
  display: flex;
  gap: var(--s-2);
  margin-top: var(--s-2);
}

.action-button {
  font-size: var(--text-xs);
  padding: var(--s-1) var(--s-2);
  border-radius: var(--r-s);
  /* subtle styling */
}
```

### Phase End Verification
- [ ] Run full test suite: `cargo test`
- [ ] Run clippy: `cargo clippy -- -D warnings`
- [ ] Manually verify explain/translate buttons work
- [ ] Verify no dead code
- [ ] Update this document with progress
- [ ] Git commit: `git commit -m "Phase 4 (Explain/translate action buttons) complete"`

---

## User Decisions (Captured)

1. **Auto-start content:** AI should greet in target dialect (future: setting parameters through conversation)

2. **Explain button behavior:** Simplified target language (not English)

3. **Translate button:** Word-by-word breakdown

4. **Message persistence:** Auto-triggered responses treated identically in history (no special marker)

5. **Rate limiting:** Same limits as user messages (normal assistant workflow, different prompts)

## Success Criteria

- All three use cases functional (auto-start, continue branch, explain/translate)
- UI matches existing styling patterns
- 100% test pass rate
- No clippy warnings
- Functions stay under 20 lines
- Clean git history with phase commits
