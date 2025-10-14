# Dialect Coach - TODO List

Last Updated: 2025-10-13

## 🔴 CRITICAL (Do Immediately)

### 1. Test WebSocket Connectivity ✅
**Status**: COMPLETED 2025-10-13
**Priority**: CRITICAL
**Estimate**: 30 minutes

**Steps**:
1. Verify backend is running on port 3000
2. Install websocat: `brew install websocat` (or `cargo install websocat`)
3. Test connection: `websocat ws://localhost:3000/ws`
4. Send test message:
   ```json
   {
     "session_id": "550e8400-e29b-41d4-a716-446655440000",
     "participant_id": "test_user",
     "content": "Hola, ¿cómo estás?",
     "language": "es-MX",
     "timestamp": "2025-10-12T00:00:00Z"
   }
   ```
5. Verify agent response is received
6. Check trace logs in backend stderr

**Result**: ✅ All success criteria met. Connection works perfectly with automatic reconnection.

---

### 2. Implement Frontend WebSocket Service ✅
**Status**: COMPLETED 2025-10-13 (324 lines implemented)
**Priority**: CRITICAL (blocks all frontend functionality)
**Estimate**: 2-3 hours
**File**: `frontend/src/services/websocket.rs`

**Requirements**:
1. Create WebSocket connection to `ws://localhost:3000/ws`
2. Handle connection lifecycle:
   - `onopen`: Set connected state
   - `onmessage`: Parse JSON, call callback
   - `onerror`: Log error, attempt reconnection
   - `onclose`: Set disconnected state, attempt reconnection
3. Implement message sending with serialization
4. Add callback system for received messages
5. Implement reconnection with exponential backoff

**Interface design**:
```rust
pub struct WebSocketService {
    ws: Option<WebSocket>,
    on_message: Option<Callback<Message>>,
    reconnect_timer: Option<Timeout>,
}

impl WebSocketService {
    pub fn connect(url: &str, on_message: Callback<Message>) -> Result<Self>;
    pub fn send_message(&self, message: &Message) -> Result<()>;
    pub fn disconnect(&mut self);
    fn reconnect(&mut self);
}
```

**Dependencies already available**:
- `web_sys::WebSocket`
- `serde_json` for serialization
- `wasm_bindgen::JsCast` for event handling
- `gloo_timers::callback::Timeout` for reconnection

**Result**: ✅ Fully implemented with ConnectionState, reconnection logic, and message queueing.

---

### 3. Integrate WebSocket with App Component ✅
**Status**: COMPLETED 2025-10-13
**Priority**: CRITICAL
**Estimate**: 2-3 hours
**File**: `frontend/src/app.rs`

**Requirements**:
1. Add state for WebSocket service: `use_state(|| None::<WebSocketService>)`
2. Add state for messages: `use_state(|| Vec::<Message>::new())`
3. Add state for session_id: `use_state(|| Uuid::new_v4())`
4. Initialize WebSocket on component mount with `use_effect_with`
5. Create callback for received messages (add to messages vec)
6. Create callback for sending messages (from InputBox)
7. Replace placeholder with ChatWindow + InputBox components
8. Pass messages and callbacks to child components

**State management**:
```rust
let messages = use_state(Vec::<Message>::new);
let session_id = use_state(|| Uuid::new_v4());
let ws_service = use_state(|| None::<WebSocketService>);

let on_message_received = {
    let messages = messages.clone();
    Callback::from(move |msg: Message| {
        let mut msgs = (*messages).clone();
        msgs.push(msg);
        messages.set(msgs);
    })
};

let on_send_message = {
    let ws_service = ws_service.clone();
    let session_id = *session_id;
    Callback::from(move |content: String| {
        if let Some(ws) = (*ws_service).as_ref() {
            let msg = Message::new(session_id, "user".to_string(), content, "es-MX".to_string());
            ws.send_message(&msg).ok();
        }
    })
};
```

**UI structure**:
```rust
html! {
    <div class="app">
        <div class="config-panel">
            {/* Existing language/dialect selectors */}
        </div>
        <ChatWindow messages={(*messages).clone()} />
        <InputBox on_send={on_send_message} />
        <SpeechControls on_speech={on_send_message.clone()} language_code={selected_dialect.bcp47_tag()} />
    </div>
}
```

**Result**: ✅ Full integration with use_reducer for state management and automatic reconnection handling.

---

## 🟠 HIGH PRIORITY (Core Functionality)

### 4. Implement ChatWindow Component ✅
**Status**: COMPLETED 2025-10-13
**Priority**: HIGH
**Estimate**: 1-2 hours
**File**: `frontend/src/components/chat_window.rs`

**Requirements**:
1. Accept `messages: Vec<Message>` prop
2. Render MessageBubble for each message
3. Auto-scroll to bottom when new message arrives
4. Show loading indicator when waiting for agent response
5. Show "empty state" when no messages

**Implementation**:
```rust
#[derive(Properties, PartialEq)]
pub struct ChatWindowProps {
    pub messages: Vec<Message>,
    pub current_user_id: String,
}

#[function_component(ChatWindow)]
pub fn chat_window(props: &ChatWindowProps) -> Html {
    let messages_ref = use_node_ref();

    // Auto-scroll effect
    use_effect_with(props.messages.clone(), {
        let messages_ref = messages_ref.clone();
        move |_| {
            if let Some(elem) = messages_ref.cast::<web_sys::HtmlElement>() {
                elem.set_scroll_top(elem.scroll_height());
            }
        }
    });

    html! {
        <div class="chat-window" ref={messages_ref}>
            if props.messages.is_empty() {
                <div class="empty-state">{"Start chatting!"}</div>
            } else {
                { for props.messages.iter().map(|msg| {
                    let is_own = msg.participant_id == props.current_user_id;
                    html! { <MessageBubble message={msg.clone()} is_own_message={is_own} /> }
                })}
            }
        </div>
    }
}
```

**Result**: ✅ Fully functional with auto-scroll, empty state, and loading indicators.

---

## 🔴 PHASE 5: SPEECH INTEGRATION (CURRENT)

### 5a. Implement Speech Synthesis Service (TTS) ❌
**Status**: 9-line placeholder
**Priority**: CRITICAL (key differentiator)
**Estimate**: 2-3 hours
**File**: `frontend/src/services/speech.rs`

**Requirements**:
1. Wrap SpeechSynthesis API from web_sys
2. Implement voice selection by BCP-47 language tag (es-MX, ar-EG, fr-FR)
3. Add speak() method with language and voice selection
4. Handle voice loading (voices may not be immediately available)
5. Add stop() and pause() methods
6. Error handling for unsupported browsers

**Interface**:
```rust
pub struct SpeechSynthesisService {
    synth: web_sys::SpeechSynthesis,
    current_language: String,
}

impl SpeechSynthesisService {
    pub fn new() -> Result<Self, String>;
    pub fn speak(&self, text: &str, language_code: &str) -> Result<(), String>;
    pub fn stop(&self);
    pub fn get_voices_for_language(&self, language_code: &str) -> Vec<SpeechSynthesisVoice>;
}
```

**Voice selection logic**:
- Match exact BCP-47 tag (e.g., "es-MX" → Mexican Spanish voice)
- Fallback to language prefix (e.g., "es-MX" → any "es-" voice)
- Fallback to default voice

**Browser compatibility**: Check for window.speechSynthesis support

**Blockers**: None

---

### 5b. Implement Speech Recognition Service (STT) ❌
**Status**: 9-line placeholder
**Priority**: CRITICAL (key differentiator)
**Estimate**: 3-4 hours
**File**: `frontend/src/services/speech.rs`

**Requirements**:
1. Wrap SpeechRecognition API from web_sys
2. Configure language by BCP-47 tag
3. Continuous recognition mode
4. Handle interim results (show "..." while speaking)
5. Final result callback
6. Error handling (no-speech, permission denied, etc.)

**Interface**:
```rust
pub struct SpeechRecognitionService {
    recognition: web_sys::SpeechRecognition,
    is_listening: bool,
}

impl SpeechRecognitionService {
    pub fn new(language_code: &str) -> Result<Self, String>;
    pub fn start(&mut self, on_result: Callback<String>, on_interim: Callback<String>) -> Result<(), String>;
    pub fn stop(&mut self);
}
```

**Event handling**:
- `onresult`: Extract transcript, call callback
- `onerror`: Log error, stop recognition
- `onend`: Set is_listening = false

**Requirements**: HTTPS or localhost (already satisfied)

**Blockers**: None

---

### 5c. Wire SpeechControls Component ❌
**Status**: UI exists, needs API integration
**Priority**: CRITICAL
**Estimate**: 2 hours
**File**: `frontend/src/components/speech_controls.rs`

**Requirements**:
1. Initialize SpeechRecognitionService on mount
2. Connect microphone button to start/stop recognition
3. Show visual feedback when listening (animated icon)
4. Display interim results in UI
5. Call on_speech callback with final transcript
6. Handle errors gracefully (show error message)

**Integration point**: Pass callback from App component

**Blockers**: Need Task 5b (STT service) completed first

---

### 5d. Add Speech Features to App Component ❌
**Status**: Not started
**Priority**: CRITICAL
**Estimate**: 2 hours
**File**: `frontend/src/app.rs`

**Requirements**:
1. Add SpeechSynthesisService to app state
2. Add SpeechControls component to UI
3. Create callback for voice input (same as text send)
4. Mark messages with is_speech:true metadata
5. Optional: Auto-play agent responses
6. Update service when dialect changes

**State additions**:
- `speech_synth_service: Option<SpeechSynthesisService>`
- `auto_play: bool` (toggle for auto-reading agent responses)

**Blockers**: Need Tasks 5a-5c completed first

---

### 5e. Add Speech Indicators ❌
**Status**: Not started
**Priority**: MEDIUM
**Estimate**: 1 hour
**File**: `frontend/src/components/message_bubble.rs`

**Requirements**:
1. Show microphone icon for messages sent via voice (is_speech: true)
2. Show speaker icon/button to read message aloud
3. Add onclick handler to trigger TTS

**UI**: Small icon badge on message bubble

**Blockers**: Need Task 5a (TTS service) completed first

---

## 🟡 MEDIUM PRIORITY (UX & Polish)

### 7. Add CSS Styling ❌
**Status**: No CSS file exists
**Priority**: MEDIUM
**Estimate**: 2-3 hours

**Requirements**:
1. Create `frontend/static/styles.css`
2. Style ChatWindow: scrollable container, max-height
3. Style MessageBubble: speech bubble design, colors
4. Style InputBox: modern input field, send button
5. Style SpeechControls: mic button with animation
6. Responsive design for mobile

**Color scheme** (suggestion):
- User messages: #007AFF (blue)
- Agent messages: #E5E5EA (gray)
- Background: #FFFFFF
- Input border: #D1D1D6

**Blockers**: None

---

### 8. Add Loading States ❌
**Status**: No loading indicators
**Priority**: MEDIUM
**Estimate**: 1 hour

**Requirements**:
1. Show "..." bubble while waiting for agent response
2. Show spinner when WebSocket is connecting
3. Show "Reconnecting..." message on connection lost
4. Show "Listening..." indicator during speech recognition

**Implementation**:
- Add `is_loading` state to app.rs
- Set true when message sent, false when response received
- ChatWindow displays loading bubble when true

**Blockers**: Need Tasks #2-3 completed first

---

### 9. Add Error Messages ❌
**Status**: No error display
**Priority**: MEDIUM
**Estimate**: 1 hour

**Requirements**:
1. Show error message when WebSocket fails to connect
2. Show error when agent returns error response
3. Show error when speech recognition fails
4. Dismissible error banner at top of screen

**Implementation**:
- Add `error_message: Option<String>` state
- Display banner when Some(msg)
- Clear on user action or timeout

**Blockers**: Need Tasks #2-3 completed first

---

### 10. Implement Session Persistence ❌
**Status**: 9-line placeholder
**Priority**: MEDIUM
**Estimate**: 3-4 hours
**File**: `frontend/src/services/persistence.rs`

**Requirements**:
1. Store conversation history in IndexedDB
2. Store user preferences (selected dialect, etc.)
3. Load history on app start
4. Clear history button in UI

**Schema**:
```typescript
// IndexedDB
Database: dialect_coach
  ObjectStore: messages
    Key: timestamp
    Value: Message JSON
  ObjectStore: preferences
    Key: string
    Value: any JSON
```

**Dependencies already available**:
- `indexed_db_futures` crate

**Blockers**: None (but low priority)

---

## 🟢 LOW PRIORITY (Code Quality)

### 11. Remove Dead Code Warnings ❌
**Status**: ConnectionState unused in backend/src/websocket.rs:21
**Priority**: LOW
**Estimate**: 30 minutes

**Options**:
1. **Remove** ConnectionState struct and methods (if broadcast not needed)
2. **Implement** broadcast functionality (if multi-user chat desired)

**Recommendation**: Remove for now, add back later if needed

**Steps to remove**:
- Delete lines 19-61 in backend/src/websocket.rs
- Remove unused imports (Uuid, HashMap, mpsc)

**Blockers**: None

---

### 12. Fix Compiler Warnings in Shared Library ❌
**Status**: 2 warnings
**Priority**: LOW
**Estimate**: 5 minutes

**Fixes**:
1. `shared/src/logic/chat.rs:1` - Remove unused import `Participant`
2. `shared/src/logic/context.rs:25` - Prefix unused variable with underscore: `_user_message`

**Command**: `cargo fix --lib -p dialect-coach-shared`

**Blockers**: None

---

### 13. Add Frontend Unit Tests ❌
**Status**: No tests exist
**Priority**: LOW
**Estimate**: 4-6 hours

**Test targets**:
1. MessageBubble rendering
2. InputBox callback firing
3. WebSocket service send/receive
4. Speech service voice selection
5. App state management

**Framework**: `wasm-bindgen-test`

**Blockers**: Need features implemented first

---

### 14. Set Up Git Repository ❌
**Status**: No commits yet, improper .gitignore
**Priority**: LOW
**Estimate**: 15 minutes

**Steps**:
1. Create `.gitignore`:
   ```
   target/
   Cargo.lock
   .env
   dist/
   *.wasm
   ```
2. `git init`
3. `git add .`
4. `git commit -m "Initial commit: Dialect Coach MVP"`
5. (Optional) Push to GitHub/GitLab

**Blockers**: None

---

## 🔵 FUTURE (Production Readiness)

### 15. Add Authentication ⏸️
**Status**: No auth implemented
**Priority**: FUTURE (production blocker)
**Estimate**: 1-2 days

**Requirements**:
1. JWT-based authentication
2. User registration/login UI
3. Protect WebSocket endpoint
4. Associate sessions with user accounts

**Blockers**: Not needed for MVP

---

### 16. Implement Rate Limiting ⏸️
**Status**: No limits
**Priority**: FUTURE (cost/security risk)
**Estimate**: 1 day

**Requirements**:
1. Rate limit per user (e.g., 100 messages/hour)
2. Rate limit per session
3. Rate limit per IP (prevent abuse)
4. Return 429 Too Many Requests with Retry-After

**Implementation options**:
- tower-governor middleware
- Custom Axum layer
- External service (Redis)

**Blockers**: Not needed for MVP

---

### 17. Restrict CORS Origins ⏸️
**Status**: Currently allows any origin
**Priority**: FUTURE (security risk)
**Estimate**: 30 minutes

**Change**:
```rust
// Current:
.layer(CorsLayer::new().allow_origin(Any))

// Production:
.layer(CorsLayer::new()
    .allow_origin("https://dialectcoach.app".parse::<HeaderValue>().unwrap()))
```

**Blockers**: Need to know production domain

---

### 18. Implement Session Cleanup ⏸️
**Status**: Sessions stored indefinitely in memory
**Priority**: FUTURE (memory leak risk)
**Estimate**: 1 day

**Requirements**:
1. Track last activity timestamp per session
2. Background task to clean up idle sessions (e.g., >1 hour idle)
3. Clean up on WebSocket disconnect
4. (Optional) Persist to database before cleanup

**Blockers**: Not critical for MVP

---

### 19. Add Monitoring & Metrics ⏸️
**Status**: Only basic tracing logs
**Priority**: FUTURE
**Estimate**: 2-3 days

**Requirements**:
1. Metrics: requests/sec, latency, error rate
2. Logging: structured logs to file or service
3. Alerts: email/Slack on errors
4. Dashboard: Grafana or similar

**Tools**:
- Prometheus for metrics
- Loki or CloudWatch for logs
- Grafana for visualization

**Blockers**: Not needed for MVP

---

### 20. Write Deployment Documentation ⏸️
**Status**: No deployment docs
**Priority**: FUTURE
**Estimate**: 4-6 hours

**Contents**:
1. Server requirements (CPU, RAM, disk)
2. Environment variable setup
3. SSL certificate configuration
4. Reverse proxy setup (nginx)
5. Systemd service files
6. Backup and recovery procedures
7. Monitoring setup

**Blockers**: Need to deploy first

---

## Decision Points

### A. Should we implement broadcast (multi-user chat)?
**Context**: ConnectionState dead code suggests broadcast was planned

**Options**:
1. **Remove** dead code, keep 1:1 user-agent sessions
2. **Implement** broadcast, allow multiple users in same session

**Recommendation**: Remove for now (simpler). Add later if users request multi-user feature.

**Impact on roadmap**: If implement, add new task "Implement Broadcast Functionality" (HIGH priority, 4-6 hours)

---

### B. Should conversations persist across server restarts?
**Context**: Currently all session history is in-memory only

**Options**:
1. **No persistence** - Simple, privacy-friendly, sufficient for MVP
2. **Persist to database** - Better UX, enables analytics, requires DB setup

**Recommendation**: No persistence for MVP. Add to FUTURE if users want conversation history.

**Impact on roadmap**: If yes, add "Set Up Database" and "Implement Session Persistence Backend" tasks

---

### C. Should we use streaming responses from Claude?
**Context**: Currently waiting for complete response before sending

**Options**:
1. **Wait for complete response** - Simpler, works with current WebSocket setup
2. **Stream tokens as generated** - Better UX (see response forming), more complex

**Recommendation**: Complete response for MVP. Streaming is nice-to-have.

**Impact on roadmap**: If streaming, add "Implement Streaming Responses" (MEDIUM priority, 2-3 days)

---

## Progress Tracking

**Last updated**: 2025-10-13

### Sprint 1: Core Functionality ✅ COMPLETED
- [✅] Task 1: Test WebSocket connectivity
- [✅] Task 2: Implement WebSocket service
- [✅] Task 3: Integrate WebSocket with app
- [✅] Task 4: Implement ChatWindow
- [✅] Phase 4: Implement reconnection logic

**Result**: Working end-to-end chat with automatic reconnection
**Completed**: 2025-10-13

### Sprint 2: Speech Integration (CURRENT)
- [ ] Task 5a: Implement speech synthesis (TTS)
- [ ] Task 5b: Implement speech recognition (STT)
- [ ] Task 5c: Wire SpeechControls component
- [ ] Task 5d: Add speech features to App
- [ ] Task 5e: Add speech indicators to MessageBubble

**Goal**: Voice input and output for dialect practice
**Estimate**: 10-12 hours
**Target date**: TBD

### Sprint 3: Polish & UX
- [ ] Task 7: Add CSS styling
- [ ] Task 8: Enhanced loading states
- [ ] Task 9: Better error messages
- [ ] Task 10: Implement session persistence

**Goal**: Production-ready UI/UX
**Estimate**: 8-10 hours
**Target date**: TBD

### Sprint 4: Code Quality
- [ ] Task 11: Remove dead code warnings
- [ ] Task 12: Fix compiler warnings
- [ ] Task 13: Add frontend unit tests
- [ ] Task 14: Set up git repository

**Goal**: Clean, maintainable codebase
**Estimate**: 6-8 hours
**Target date**: TBD

### Future Sprints: Production Readiness
- [ ] Tasks 15-20: Security, monitoring, deployment

**Goal**: Production deployment
**Estimate**: 5-10 days
**Target date**: TBD

---

## Notes

- All estimates are approximate and may change
- Tasks marked ❌ are not started
- Tasks marked ⏸️ are deferred to future
- Priorities may shift based on user feedback
- This document should be updated as tasks are completed
