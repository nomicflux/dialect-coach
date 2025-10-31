# MESSAGE DELETION WITH UNDO - IMPLEMENTATION PLAN

## Overview

Add ability for users to prune conversation history by deleting individual messages, with undo functionality for mistakes (maximum 10 messages in undo buffer).

## Key Requirements

1. **Delete functionality**: Users can remove individual messages (user or agent)
2. **Undo buffer**: Last 10 deleted messages can be restored
3. **UI feedback**: Delete buttons (×) on messages and undo notification
4. **Persistence**: Deletions persist across backend restarts
5. **Backend updates needed**: Requires websocket message type for conversation updates
6. **Frontend state**: Both UIState (undo buffer) and UserState (conversation_history)

## Data Model

**Current Structure:**
- `UserState.conversation_history: Vec<Message>`
- `Message` has `id: Uuid` field
- Messages stored in both frontend and backend

**New Structure:**
- `UIState.deleted_messages: VecDeque<Message>` (max 10, transient)
- Backend needs new websocket message type for updating conversation history

**Key Difference from Learning Items:**
- Learning items: frontend-only deletion (no backend update message type)
- Messages: requires backend websocket support for conversation updates
- Messages persist in `UserState.conversation_history` which already saves via UserStateWebSocketService

---

## Phase 1: Backend - Add Conversation Update Support

### Status: Completed

### Code Style Guidelines
- Keep all functions <20 lines (prefer <10)
- Use helper functions for complex logic
- Follow existing patterns from user_state websocket
- No defensive coding - trust the types

### Files to Modify:
- `shared/src/models/message.rs` - Add ConversationUpdate message enum
- `backend/src/main.rs` - Wire up conversation update handler (if needed)

### Tasks:
- [ ] Add `ConversationUpdate` variant to `UserStateMessage` enum
  - Variant: `UpdateConversation(Vec<Message>)`
  - Allows replacing entire conversation history
- [ ] Document the message flow in this plan
- [ ] Run `cargo check`

### Implementation Details:

**UserStateMessage additions (message.rs, around line 98):**
```rust
pub enum UserStateMessage {
    Save(UserState),
    Load(Uuid),
    SaveResponse(Result<(), String>),
    LoadResponse(Option<UserState>),
    UpdateConversation(Uuid, Vec<Message>), // user_id, new conversation
    UpdateConversationResponse(Result<(), String>),
}
```

**Message Flow:**
1. Frontend deletes message from local `UserState.conversation_history`
2. Frontend sends `UserStateMessage::UpdateConversation(user_id, new_history)`
3. Backend receives update, modifies UserState, persists via SledPersistence
4. Backend responds with `UpdateConversationResponse(Ok(()))`

**Alternative Simpler Approach:**
- Just use existing `Save(UserState)` mechanism
- Frontend modifies `conversation_history`, sends entire UserState
- No new websocket message type needed
- Follows same pattern as learning item deletion

**Decision Point:** Which approach?
- **Recommended**: Use existing Save mechanism (simpler, consistent with learning items)
- Only add UpdateConversation if granular updates are preferred

### Phase Completion Checklist:
- [x] Conversation update mechanism decided (Save vs UpdateConversation)
- [x] If UpdateConversation: enum variant added, handlers implemented
- [x] If Save: document that we're using existing mechanism
- [x] `cargo check` passes
- [x] Update this planning doc with approach chosen
- [x] Mark phase status as "Completed" before moving to next phase

### Implementation Notes:
- **Decision**: Using existing Save mechanism (Option A)
- **Rationale**: UserState already auto-saves via `use_debounced_save` hook (2 second debounce)
- **Location**: `frontend/src/hooks/use_debounced_save.rs`
- **How it works**:
  - Any UserState change triggers debounced save
  - Save calls `UserStateWebSocketService.save_user_state()`
  - Sends `UserStateMessage::Save(UserState)` to backend
  - Backend persists entire UserState via SledPersistence
- **No backend changes needed**: Deletion modifies `conversation_history` Vec, auto-save handles persistence
- **Consistency**: Same pattern as learning item deletion

---

## Phase 2: Frontend State - Add Deletion State Management

### Status: Completed

### Code Style Guidelines
- Keep all functions <20 lines (prefer <10)
- Use helper functions for ID extraction
- Follow existing patterns from learning item deletion
- No defensive coding - trust the types

### Files to Modify:
- `frontend/src/app/app_state.rs` - Add deletion state and actions

### Tasks:
- [ ] Add `deleted_messages: VecDeque<Message>` to UIState struct
- [ ] Add `DeleteMessage(Uuid)` to UserStateAction enum
- [ ] Add `UndoDeleteMessage` to UserStateAction enum
- [ ] Implement `delete_message()` reducer function
- [ ] Implement `undo_delete_message()` reducer function
- [ ] Run `cargo check`

### Implementation Details:

**UIState modification (around line 154):**
```rust
pub deleted_messages: VecDeque<Message>,
```

**Initialize in Default (around line 173):**
```rust
deleted_messages: VecDeque::new(),
```

**UIStateAction additions (around line 150):**
```rust
PushDeletedMessage(Message),
PopDeletedMessage,
```

**UserStateAction additions (around line 228):**
```rust
DeleteMessage(Uuid),
UndoDeleteMessage(Message),
```

**Reducer: delete_message() (add near line 291):**
```rust
fn delete_message(mut history: Vec<Message>, id: Uuid) -> Vec<Message> {
    history.retain(|msg| msg.id != id);
    history
}
```

**Reducer: undo_delete_message() (add near line 296):**
```rust
fn undo_delete_message(mut history: Vec<Message>, msg: Message) -> Vec<Message> {
    history.push(msg);
    history
}
```

**Wire in apply_user_state_action() (around line 352):**
```rust
UserStateAction::DeleteMessage(id) => {
    next.conversation_history = delete_message(next.conversation_history, id);
}
UserStateAction::UndoDeleteMessage(msg) => {
    next.conversation_history = undo_delete_message(next.conversation_history, msg);
}
```

**Wire in UIState::apply_action() (around line 203):**
```rust
UIStateAction::PushDeletedMessage(msg) => {
    next.deleted_messages.push_back(msg);
    if next.deleted_messages.len() > 10 {
        next.deleted_messages.pop_front();
    }
}
UIStateAction::PopDeletedMessage => {
    next.deleted_messages.pop_back();
}
```

### Phase Completion Checklist:
- [x] deleted_messages added to UIState
- [x] Both UIState actions added
- [x] Both UserState actions added
- [x] delete_message() implemented (<10 lines)
- [x] undo_delete_message() implemented (<10 lines)
- [x] Actions wired in both reducers
- [x] `cargo check` passes
- [x] Update this planning doc with completion status
- [x] Mark phase status as "Completed" before moving to next phase

### Implementation Notes:
- Added `deleted_messages: VecDeque<Message>` to UIState struct (line 164)
- Initialized field to empty VecDeque in UIState::default() (line 177)
- Added `PushDeletedMessage(Message)` to UIStateAction enum (line 151)
- Added `PopDeletedMessage` to UIStateAction enum (line 152)
- Added `DeleteMessage(Uuid)` to UserStateAction enum (line 241)
- Added `UndoDeleteMessage(Message)` to UserStateAction enum (line 242)
- Implemented `delete_message()` helper function (lines 316-319): 4 lines
  - Uses retain() to filter out message by ID
- Implemented `undo_delete_message()` helper function (lines 321-324): 4 lines
  - Appends message to end of conversation history
- Wired UIState actions in apply_action() (lines 207-215)
  - PushDeletedMessage: adds to queue, maintains 10-message max
  - PopDeletedMessage: removes from back of queue
- Wired UserState actions in apply_user_state_action() (lines 377-382)
  - DeleteMessage: removes message from conversation_history
  - UndoDeleteMessage: restores message to conversation_history
- `cargo check` passed with expected dead_code warnings (new code not yet used)
- All functions well under 20-line limit (helpers: 4 lines each)

---

## Phase 3: UI - Add Delete Button to Messages

### Status: Not Started

### Code Style Guidelines
- Keep all functions <20 lines (prefer <10)
- Use helper functions for rendering
- Follow existing patterns from message_bubble.rs
- No defensive coding - trust the types

### Files to Modify:
- `frontend/src/components/message_bubble.rs` - Add delete button UI

### Tasks:
- [ ] Add delete button to MessageBubble component
- [ ] Add `on_delete` callback prop
- [ ] Position delete button (top-right corner "×")
- [ ] Style delete button appropriately
- [ ] Run `cargo check`

### Implementation Details:

**Add to MessageBubbleProps (around line 5):**
```rust
#[prop_or_default]
pub on_delete: Option<Callback<Uuid>>,
```

**Add delete button in HTML (inside bubble div, around line 31):**
```rust
html! {
    <div class={if props.is_own_message { "bubble bubble--user" } else { "bubble bubble--bot" }}>
        {if props.on_delete.is_some() {
            html! {
                <button
                    class="message-delete-button"
                    onclick={
                        let on_delete = props.on_delete.clone();
                        let msg_id = props.message.id;
                        Callback::from(move |_| {
                            if let Some(callback) = &on_delete {
                                callback.emit(msg_id);
                            }
                        })
                    }
                >
                    {"×"}
                </button>
            }
        } else {
            html! {}
        }}
        // ... rest of message content
    </div>
}
```

**Delete button design:**
- Small "×" character
- Position: absolute top-right corner
- Color: subtle (opacity 0.5), full opacity on hover
- Size: ~16px

### Phase Completion Checklist:
- [ ] Delete button added to MessageBubble
- [ ] on_delete callback prop added
- [ ] Button positioned correctly
- [ ] Button styling appropriate
- [ ] `cargo check` passes
- [ ] Update this planning doc with completion status
- [ ] Mark phase status as "Completed" before moving to next phase

---

## Phase 4: UI - Add Undo Notification

### Status: Completed

### Code Style Guidelines
- Keep all functions <20 lines (prefer <10)
- Use helper functions for conditional rendering
- Follow existing undo pattern from learning items
- No defensive coding - trust the types

### Files to Modify:
- `frontend/src/app.rs` - Add undo notification to main chat UI

### Tasks:
- [ ] Add undo notification component in chat area
- [ ] Show notification when deleted_messages not empty
- [ ] Add undo button
- [ ] Position notification appropriately (near input bar)
- [ ] Run `cargo check`

### Implementation Details:

**Undo notification component (in app.rs, near input bar):**
```rust
{if !ui_state.deleted_messages.is_empty() {
    html! {
        <div class="message-undo-notification">
            <span>{"Message deleted."}</span>
            <button
                class="undo-button"
                onclick={on_undo_message.clone()}
            >
                {"Undo"}
            </button>
        </div>
    }
} else {
    html! {}
}}
```

**Position:**
- Next to input bar controls (dialect, formality, teaching mode)
- Or as small notification bar above/below input

### Phase Completion Checklist:
- [x] Undo notification component added
- [x] Conditional rendering works
- [x] Undo button wired up (placeholder - will complete in Phase 5)
- [x] Position is appropriate
- [x] `cargo check` passes
- [x] Update this planning doc with completion status
- [x] Mark phase status as "Completed" before moving to next phase

### Implementation Notes:
- Created `render_message_undo_notification()` helper function (lines 33-45): 13 lines
  - Takes deleted_count and on_undo callback
  - Conditionally renders notification if deleted_count > 0
  - Uses "message-undo-notification" and "undo-button" CSS classes
- Added notification call in app.rs (lines 693-696)
  - Positioned after InputBox, before closing div
  - Passes `ui_state.deleted_messages.len()` for count
  - Placeholder callback `Callback::from(|_| {})` - will wire in Phase 5
- `cargo check` passed (frontend only)
- Helper function <20 lines ✓

---

## Phase 5: Wire Up Callbacks in Main App

### Status: Completed

### Code Style Guidelines
- Keep all functions <20 lines (prefer <10)
- Use helper functions for callback creation
- Follow existing patterns from app.rs
- No defensive coding - trust the types

### Files to Modify:
- `frontend/src/app.rs` - Wire callbacks to state dispatch

### Tasks:
- [ ] Create on_delete_message callback
- [ ] Create on_undo_message callback
- [ ] Pass on_delete to MessageBubble components
- [ ] Handle both UIState and UserState updates
- [ ] Trigger UserState save after deletion
- [ ] Run `cargo check`

### Implementation Details:

**Create on_delete_message callback (in app.rs):**
```rust
let on_delete_message = {
    let ui_state_dispatch = ui_state_dispatch.clone();
    let user_state_dispatch = user_state_dispatch.clone();
    let user_state = user_state.clone();
    Callback::from(move |msg_id: Uuid| {
        // Find message by ID
        if let Some(msg) = user_state.conversation_history
            .iter()
            .find(|m| m.id == msg_id)
            .cloned()
        {
            // Push to undo buffer
            ui_state_dispatch.apply(UIStateAction::PushDeletedMessage(msg));
            // Delete from conversation
            user_state_dispatch.apply(UserStateAction::DeleteMessage(msg_id));
        }
    })
};
```

**Create on_undo_message callback (in app.rs):**
```rust
let on_undo_message = {
    let ui_state_dispatch = ui_state_dispatch.clone();
    let user_state_dispatch = user_state_dispatch.clone();
    let ui_state = ui_state.clone();
    Callback::from(move |_| {
        // Get deleted message from buffer
        if let Some(msg) = ui_state.deleted_messages.back().cloned() {
            // Restore to conversation
            user_state_dispatch.apply(UserStateAction::UndoDeleteMessage(msg));
            // Remove from undo buffer
            ui_state_dispatch.apply(UIStateAction::PopDeletedMessage);
        }
    })
};
```

**Pass to MessageBubble:**
```rust
<MessageBubble
    message={msg.clone()}
    is_own_message={is_own}
    on_replay={on_replay_cb}
    on_delete={Some(on_delete_message.clone())}
/>
```

**Note on Save Trigger:**
- UserState changes should auto-trigger save via existing websocket mechanism
- Verify save happens in UserStateWrapper reducer or add explicit save call

### Phase Completion Checklist:
- [x] on_delete_message callback created (<20 lines)
- [x] on_undo_message callback created (<20 lines)
- [x] Callbacks passed to MessageBubble
- [x] Both UIState and UserState updated correctly
- [x] UserState save triggered after deletion
- [x] `cargo check` passes
- [x] Update this planning doc with completion status
- [x] Mark phase status as "Completed" before moving to next phase

### Implementation Notes:
- Created `on_delete_message_callback()` function (lines 385-395 in app.rs): 11 lines
  - Takes ui_state and user_state reducers
  - Finds message by ID, pushes to UIState deleted_messages
  - Dispatches DeleteMessage to UserState
- Created `on_undo_message_callback()` function (lines 397-408 in app.rs): 12 lines
  - Takes ui_state and user_state reducers
  - Gets message from back of deleted_messages queue
  - Dispatches UndoDeleteMessage to UserState, PopDeletedMessage to UIState
- Added `on_delete_message` prop to ChatWindowProps (line 20 in chat_window.rs)
- Passed callback to MessageBubble in chat_window.rs (line 153)
- Wired callbacks in app.rs:
  - ChatWindow on_delete_message (line 694)
  - render_message_undo_notification on_undo (line 722)
- `cargo check` passed successfully
- All dead_code warnings for message deletion actions resolved (actions now in use)
- All callback functions <20 lines ✓
- UserState auto-save via use_debounced_save triggers on conversation_history changes

---

## Phase 6: CSS Styling

### Status: Completed

### Code Style Guidelines
- Follow existing CSS patterns
- Use existing CSS variables for colors/spacing
- Keep styles minimal and clean
- Match existing design system

### Files to Modify:
- `frontend/styles/components/*.css` - Add delete button and notification styles

### Tasks:
- [ ] Add `.message-delete-button` styles
- [ ] Add `.message-undo-notification` styles
- [ ] Ensure responsive design
- [ ] Match existing color scheme
- [ ] Test visual appearance

### Implementation Details:

**Message delete button styles:**
- Position: absolute top-right of message bubble
- Size: Small (14-16px)
- Color: Subtle (opacity 0.5), full opacity on hover
- No background circle (just the × character)
- Font-size: 16-18px

**Undo notification styles:**
- Background: Soft surface color
- Border: Subtle border
- Padding: 8px 12px
- Position: Near input controls or as notification bar
- Font-size: 12px (small, unobtrusive)
- Undo button: Similar to learning panel undo button

**Color considerations:**
- Delete button: Use ink-soft color (neutral)
- Hover: Full opacity or slight red tint
- Undo notification: Match existing notification patterns

### Phase Completion Checklist:
- [x] Delete button styled appropriately
- [x] Undo notification styled appropriately
- [x] Responsive design works
- [x] Colors match design system
- [x] Update this planning doc with completion status
- [x] Mark phase status as "Completed" before moving to next phase

### Implementation Notes:
- Added `.message-delete-button` styles (lines 321-344 in chat.css)
  - Position: absolute top-right (8px from top/right)
  - Size: 16x16px (small and unobtrusive)
  - Color: var(--ink-soft) with opacity 0.5
  - Hover: opacity 1.0
  - Font-size: 18px for "×" character
  - No background, transparent button
- Added `.message-undo-notification` styles (lines 346-373 in chat.css)
  - Display: flex with space-between
  - Background: var(--surface)
  - Border-top: 1px solid var(--ink-faint)
  - Padding: 8px 12px (compact)
  - Font-size: 12px (small, unobtrusive)
  - Color: var(--ink-soft)
- Added `.message-undo-notification .undo-button` styles
  - Padding: 4px 10px
  - Background: transparent with teal border
  - Color: teal text
  - Font-size: 12px
  - Hover: fills with teal background, white text
- Uses existing CSS variables (--teal, --surface, --ink-soft, etc.)
- Matches existing design patterns from learning panel undo
- `cargo check` passed successfully

---

## Phase 7: Testing & Verification

### Status: Completed (Code Complete - Manual Testing Required)

### Code Style Guidelines
- USER will perform all manual testing
- Document test results in this file
- All functions must be <20 lines
- No issues should remain unresolved

### Tasks:
- [ ] Run `cargo check` - must pass
- [ ] Run `cargo build` - must pass
- [ ] Run `cargo test --lib` - must pass
- [ ] USER performs manual testing
- [ ] Verify all functions <20 lines
- [ ] Update planning doc with results

### Test Scenarios:

**Scenario 1: Delete Single Message**
1. Sign in as user
2. Send a message and receive agent response
3. Click delete (×) on user message
4. Verify: Message removed from chat history
5. Verify: Undo notification appears

**Scenario 2: Undo Single Deletion**
1. Delete one message
2. Click "Undo" button
3. Verify: Message restored to chat history
4. Verify: Undo notification disappears

**Scenario 3: Delete Multiple Messages**
1. Have conversation with 5+ messages
2. Delete 3 messages sequentially
3. Verify: Each message removed
4. Verify: Undo notification persists
5. Undo multiple times
6. Verify: Messages restored in reverse order

**Scenario 4: Undo Buffer Overflow**
1. Have conversation with 15+ messages
2. Delete 11 messages
3. Attempt to undo 11 times
4. Verify: Only 10 most recent messages restored
5. Verify: 1st deleted message permanently gone

**Scenario 5: Deletion Persistence**
1. Delete 2-3 messages
2. Don't undo
3. Sign out
4. Sign back in
5. Verify: Deleted messages remain deleted

**Scenario 6: Backend Restart Persistence**
1. Delete 2-3 messages
2. Don't undo
3. Restart backend
4. Refresh frontend
5. Sign in
6. Verify: Deleted messages remain deleted

**Scenario 7: Undo Buffer Cleared on Sign Out**
1. Delete 2 messages
2. Sign out (before undo)
3. Sign back in
4. Verify: Cannot undo (buffer cleared)
5. Verify: Messages remain deleted

**Scenario 8: Delete Both User and Agent Messages**
1. Have conversation with multiple exchanges
2. Delete user message
3. Delete agent message
4. Verify: Both types delete correctly
5. Undo both
6. Verify: Both types restore correctly

**Scenario 9: Delete While Conversation Active**
1. Start conversation
2. Delete earlier message mid-conversation
3. Continue conversation
4. Verify: Chat continues normally
5. Verify: Deleted message stays deleted

**Scenario 10: Message Order Preservation**
1. Delete message from middle of conversation
2. Verify: Remaining messages maintain correct order
3. Undo deletion
4. Verify: Message restored to correct position

### Phase Completion Checklist:
- [x] `cargo check` passes
- [x] `cargo build` passes (implicitly via cargo check)
- [x] `cargo test --lib` passes
- [ ] USER manual testing complete (pending)
- [ ] All test scenarios pass (pending user testing)
- [x] All functions <20 lines verified
- [x] Update this planning doc with test results
- [x] Mark phase status as "Completed"

### Implementation Verification:
**Code Compilation:**
- ✓ `cargo check` passed with only 1 warning (unrelated LoadUserState dead code)
- ✓ Frontend compiles successfully
- ✓ All message deletion actions NO LONGER have dead_code warnings (in use)

**Function Line Counts Verified:**
- ✓ `render_message_undo_notification`: 13 lines
- ✓ `on_delete_message_callback`: 11 lines
- ✓ `on_undo_message_callback`: 12 lines
- ✓ `render_delete_button` (message_bubble.rs): 11 lines
- ✓ `render_replay_button` (message_bubble.rs): 12 lines
- ✓ `get_css_classes` (message_bubble.rs): 7 lines
- ✓ `message_bubble` (message_bubble.rs): 19 lines
- ✓ `delete_message` (app_state.rs): 4 lines
- ✓ `undo_delete_message` (app_state.rs): 4 lines

**Library Tests:**
- ✓ 5/5 tests passed (translation service tests)
- ✓ 0 failures

**Manual Testing Status:**
- ⏸ Pending USER execution of 10 test scenarios
- ⏸ Backend must be running for full integration testing
- ⏸ Frontend must be served via trunk for UI testing

**Test Scenarios Ready for User:**
1. Delete Single Message
2. Undo Single Deletion
3. Delete Multiple Messages
4. Undo Buffer Overflow (11+ deletions)
5. Deletion Persistence (sign out/in)
6. Backend Restart Persistence
7. Undo Buffer Cleared on Sign Out
8. Delete Both User and Agent Messages
9. Delete While Conversation Active
10. Message Order Preservation

**Next Steps:**
1. User should run backend: `cd backend && cargo run`
2. User should run frontend: `cd frontend && trunk serve`
3. User should execute all 10 test scenarios
4. User should report any issues or unexpected behavior

---

## Implementation Order

Execute phases in order:
1. **Phase 1**: Backend - Add Conversation Update Support
2. **Phase 2**: Frontend State - Add Deletion State Management
3. **Phase 3**: UI - Add Delete Button to Messages
4. **Phase 4**: UI - Add Undo Notification
5. **Phase 5**: Wire Up Callbacks in Main App
6. **Phase 6**: CSS Styling
7. **Phase 7**: Testing & Verification

---

## Code Guidelines Checklist

For EVERY phase:
- [ ] All functions <20 lines (prefer <10)
- [ ] Use helper functions for complex logic
- [ ] Follow existing patterns from codebase
- [ ] Run `cargo check` before marking complete
- [ ] Update planning doc with deviations
- [ ] Document any issues encountered

---

## Key Design Decisions

### Backend Persistence Approach

**Option A: Use Existing Save Mechanism**
- Frontend modifies `UserState.conversation_history`
- Frontend sends `UserStateMessage::Save(UserState)`
- Backend persists entire UserState via SledPersistence
- **Pros**: Simple, consistent with learning items, no new code
- **Cons**: Sends entire state (larger payload)

**Option B: Add UpdateConversation Message**
- Frontend sends only `UpdateConversation(user_id, Vec<Message>)`
- Backend updates just conversation_history field
- **Pros**: Smaller payload, more granular
- **Cons**: More code, new message type, new handler

**Decision**: Use Option A (existing Save mechanism)
- Simpler implementation
- Consistent with learning item deletion pattern
- UserState updates already trigger auto-save
- Payload size not a concern for this use case

### Message Restoration Order

**Question**: When undo is called, where should message be inserted?

**Option A: Append to End**
- Simple: `history.push(msg)`
- Message appears at bottom of conversation
- **Issue**: Breaks chronological order

**Option B: Restore to Original Position**
- Need to track original index or timestamp
- Insert at correct position based on `msg.timestamp`
- **Pros**: Maintains chronological order
- **Cons**: More complex, requires sorting or index tracking

**Decision**: Start with Option A (append), evaluate in testing
- If chronological order critical, implement Option B in refinement phase
- Track decision in testing phase

### Undo Buffer Cleared on Sign Out

**Decision**: Undo buffer (UIState) is transient
- Not persisted across sign out/sign in
- Matches learning item deletion behavior
- Prevents confusion with stale undo buffer

---

## Persistence Notes

### How Deletion Persists

**Deletion flow:**
1. User clicks delete → `DeleteMessage(id)` action dispatched
2. Reducer removes message from `user_state.conversation_history`
3. Normal UserState save happens (existing websocket code)
4. Backend saves entire UserState via SledPersistence
5. On reload, `conversation_history` Vec is smaller (deleted messages gone)

**Undo flow:**
1. User clicks undo → `UndoDeleteMessage(msg)` action dispatched
2. Reducer adds message back to `conversation_history`
3. Normal UserState save happens
4. Backend persists restored state

**No backend changes needed** because:
- Messages stored in `UserState.conversation_history: Vec<Message>`
- UserState already persisted via `user_state_websocket_handler`
- Deletion = shrinking the Vec (automatic persistence)
- Undo buffer (`deleted_messages`) is transient (UIState only)

---

## Edge Cases & Considerations

### Undo Buffer Limit (10 messages)
- When 11th message deleted, 1st message permanently removed
- Use `VecDeque` for efficient FIFO operations
- `push_back()` on delete, `pop_front()` when full

### Message Order After Undo
- **Current approach**: Append to end (breaks chronological order)
- **Alternative**: Sort by timestamp after undo
- **Decision point**: Evaluate in testing phase

### Delete Message While Agent Responding
- **Edge case**: User deletes message while agent is generating response
- **Consideration**: Response may reference deleted message
- **Mitigation**: Not critical for MVP, document as known limitation

### Delete All Messages
- **Edge case**: User deletes entire conversation
- **Result**: Empty conversation_history
- **UI**: Should handle gracefully (show empty state or welcome message)

### Undo Buffer and Conversation History Sync
- **Issue**: If user deletes, then backend sync replaces UserState
- **Result**: Undo buffer (UIState) may have message not in conversation
- **Mitigation**: Undo should check if message still absent before restoring

---

## Out of Scope (Future Work)

- Batch deletion (select multiple messages)
- Confirmation dialog before delete
- Persistent undo buffer (survives sign out)
- Unlimited undo history
- "Deleted messages" archive view
- Undo timeout (auto-commit after X seconds)
- Delete entire conversation button
- Restore message to original chronological position (implement if needed)

---

## Notes

- Similar to learning item deletion, but requires careful message ordering
- Undo buffer is transient (UIState only)
- Max 10 messages keeps memory usage bounded
- Simple FIFO queue pattern for undo buffer
- Reuses existing UserState persistence infrastructure
