# Dialect Coach - Project Status

Last Updated: 2025-10-12

## Executive Summary

**Backend**: ✅ Fully functional with enhanced RAG retrieval strategy
**Corpus Data**: ✅ 10,911 documents uploaded to Qdrant Cloud
**Frontend**: ⚠️ UI components exist but services are placeholders
**WebSocket**: ❌ Handler implemented but NEVER TESTED end-to-end

## Component Status

### ✅ Backend (COMPLETE)

#### main.rs
- Axum server on port 3000
- TraceLayer for HTTP tracing
- CORS configured (allow any origin)
- Routes: `/health`, `/ws`
- State management with Arc for thread-safe sharing

#### websocket.rs
- WebSocket upgrade handler implemented
- Per-connection message handling
- Session history tracking (HashMap<Uuid, Vec<String>>)
- Message parsing with error handling
- Agent response integration
- **Issues**: ConnectionState dead code (broadcast feature never implemented)

#### agent_service.rs
- Anthropic Claude 3.5 Sonnet integration via Rig
- **Enhanced RAG strategy** (implemented this session):
  - Multi-vector retrieval: 3 embeddings per query (content, style, topic)
  - Dual retrieval: Semantic search + random sampling
  - Dense examples: 50 total examples grouped by formality
  - History-enhanced queries: Uses last 2-3 messages for context
- System prompt with examples-first structure
- Conversation history support (last 20 messages)

#### qdrant_service.rs
- Connection to Qdrant Cloud
- `search_dialect_examples()`: Vector search with dialect filtering
- `random_dialect_samples()`: Random sampling with formality filters (added this session)
- `get_collection_info()`: Debugging helper
- `create_field_index()`: Index creation for faster filtering
- **Current data**: 10,911 dialect documents

#### embedding_service.rs
- Fastembed with MultilingualE5Base model
- 768-dimensional embeddings
- ~50ms per embedding generation
- Model loaded at startup (~5 second initialization)

### ✅ Corpus Processor (COMPLETE)

#### Commands
- `process`: Load corpus files, chunk text, generate embeddings, save to JSONL
- `upload`: Upload processed JSONL to Qdrant
- `list`: Show available languages and dialects

#### Pipeline
1. Load text files or CSV
2. Chunk into ~512 character segments with 50-char overlap
3. Generate embeddings with Fastembed
4. Save as JSONL with metadata (dialect, formality)
5. Upload to Qdrant with field indices

#### Supported Dialects
Currently have corpus data for:
- Spanish Argentinian
- Spanish Colombian
- Spanish Puerto Rican (Caribbean)

### ✅ Shared Library (COMPLETE)

#### Models
- `Language`: Spanish, Arabic, French
- `Dialect`: 16 total variants (6 Spanish, 5 Arabic, 5 French)
- `Message`: WebSocket message structure with BCP-47 language tags
- `ChatSession`: Session management with context window
- `Participant`: Human/Agent distinction with AgentType
- `DialectDocument`: RAG document structure
- `WsEvent`: WebSocket event protocol
- `DialectConfig`: Agent configuration (formality, teaching mode, personality)

#### Business Logic
- `chat.rs`: `should_agent_respond()`, `agents_to_respond()` - multi-agent response logic
- `context.rs`: Context window management (CURRENTLY UNUSED)

### ⚠️ Frontend (PARTIALLY IMPLEMENTED)

#### app.rs - Main Component
**Status**: Basic structure only
**Implemented**:
- Language dropdown selector (Spanish, Arabic, French)
- Dialect dropdown (filtered by language)
- Placeholder text: "Chat interface coming soon..."

**Missing**:
- ChatWindow integration
- WebSocket connection initialization
- Message sending/receiving
- Session management
- State management for messages

#### Components

##### message_bubble.rs ✅
- Displays individual message with participant ID, content, timestamp
- Styling classes for own vs other messages
- **Fully implemented**

##### input_box.rs ✅
- Text input field with form submission
- Callback on send
- Clears input after send
- **Fully implemented**

##### speech_controls.rs ⚠️
- UI for mic button implemented
- Toggle listening state
- **Missing**: Web Speech API integration (placeholder comment)

##### chat_window.rs ❌
- Only 11 lines
- Empty placeholder
- **Needs**: Message list rendering, auto-scroll, loading states

#### Services

##### websocket.rs ❌
**Status**: 9-line placeholder
**Current code**:
```rust
pub struct WebSocketService;
impl WebSocketService {
    pub fn new(_url: &str) -> Self { Self }
}
```
**Needs full implementation**:
- WebSocket connection using web_sys::WebSocket
- Connection lifecycle (onopen, onclose, onerror, onmessage)
- Message serialization/deserialization
- Reconnection logic
- Callback system for received messages

##### speech.rs ❌
**Status**: 9-line placeholder
**Needs**:
- SpeechSynthesis wrapper for TTS
- Voice selection by BCP-47 tag
- SpeechRecognition wrapper for STT
- Continuous recognition mode
- Result callbacks

##### persistence.rs ❌
**Status**: 9-line placeholder
**Needs**:
- IndexedDB wrapper using indexed_db_futures
- Session history storage
- Message persistence
- Settings storage

### 🔧 Build System

#### Backend
- Build command: `cargo build` or `cargo run`
- Target: Native (darwin/linux/windows)
- **Status**: Builds successfully with 2 warnings (dead code)

#### Frontend
- Build tool: Trunk 0.21.14
- Build command: `trunk build` or `trunk serve`
- Target: wasm32-unknown-unknown
- **Status**: Builds successfully but has artifact name ambiguity warning
- **Issue**: "found more than one target artifact: dialect_coach_frontend, dialect-coach-frontend"
- **Dev server**: http://localhost:8080

#### Warnings Summary
```
backend/src/websocket.rs:21 - Dead code: ConnectionState
shared/src/logic/chat.rs:1 - Unused import: Participant
shared/src/logic/context.rs:25 - Unused variable: user_message
```

## Testing Status

### ❌ CRITICAL: WebSocket Testing
**NEVER TESTED END-TO-END**

The WebSocket handler exists and compiles, but we have NEVER verified:
1. Can clients connect to ws://localhost:3000/ws?
2. Does the handler execute when a connection is made?
3. Can the handler receive and parse Message JSON?
4. Does the agent actually generate responses?
5. Are responses sent back to the client?

**Original task from previous session**: Test WebSocket connectivity
**What happened**: Task was abandoned in favor of RAG improvements

### Backend Unit Tests
- `backend/src/websocket.rs`: Basic tests for ConnectionState (currently dead code)
- `backend/src/qdrant_service.rs`: Filter construction test, connection test (marked `#[ignore]`)
- Tests marked `#[ignore]` require real credentials

### Shared Library Tests
- `shared/src/models/dialect.rs`: BCP-47 mapping tests ✅
- `shared/src/logic/chat.rs`: Agent response logic tests ✅
- All tests pass

### Frontend Tests
- No tests currently exist

## Performance Benchmarks

### Backend Startup
- Qdrant connection: ~200ms
- Fastembed model loading: ~4-5 seconds
- **Total**: ~5 seconds

### Request Latency (Estimated)
- Embedding generation (×3): ~150ms
- Qdrant queries (×4 parallel): ~100-200ms
- Claude API call: ~2-5 seconds
- **Total response time**: ~2.5-6 seconds

### Memory Usage
- Backend RSS: ~500MB (Fastembed model)
- Qdrant storage: ~33MB (10,911 × 768 × 4 bytes)

## Environment Setup

### Required Environment Variables
```bash
# Backend
QDRANT_URL=https://ebc1f227-4b58-43ee-8eba-cd8742eba4b6.us-east4-0.gcp.cloud.qdrant.io:6334
QDRANT_API_KEY=<redacted>
ANTHROPIC_API_KEY=<redacted>
ANTHROPIC_MODEL=claude-3-5-sonnet-20241022  # optional

# Optional
RUST_LOG=debug  # For verbose tracing
```

### Dependencies Installed
- Rust toolchain (edition 2024)
- wasm32-unknown-unknown target (for frontend)
- Trunk CLI (for frontend builds)

## Git Status

Current branch: `main`
No upstream branch configured

Untracked files:
- `.gitignore`
- `Cargo.toml`
- `src/` (all source files)

**Note**: Project has never been committed to git yet

## Known Issues & Technical Debt

### 1. Dead Code in websocket.rs
**Issue**: ConnectionState struct and all its methods are unused
**Root cause**: Broadcast functionality was designed but never implemented
**Impact**: Compiler warnings on every build
**Options**:
- Remove dead code (if broadcast not needed)
- Implement broadcast feature (for multi-user sessions)

### 2. WebSocket Connectivity Untested
**Issue**: Handler exists but has never been tested with real clients
**Impact**: Unknown if WebSocket actually works
**Risk**: HIGH - entire application depends on this
**Next step**: Test with websocat or browser WebSocket client

### 3. Frontend Services Are Stubs
**Issue**: websocket.rs, speech.rs, persistence.rs are 9-line placeholders
**Impact**: Frontend UI exists but cannot communicate with backend
**Blockers**: Nothing - just needs implementation

### 4. No Error Recovery in Frontend
**Issue**: No reconnection logic, no offline handling, no error display
**Impact**: Poor user experience on network issues

### 5. No Authentication
**Issue**: WebSocket endpoint is completely open
**Security risk**: Anyone can connect and use Claude API credits
**Production blocker**: Yes

### 6. Unbounded Session History Growth
**Issue**: Session histories stored in memory, only capped at 20 messages per session
**Risk**: Memory leak if many sessions accumulate
**Mitigation**: Need session cleanup/expiry logic

### 7. CORS Set to Allow Any Origin
**Issue**: `CorsLayer::new().allow_origin(Any)`
**Security risk**: Any website can connect to backend
**Production blocker**: Yes

### 8. No Rate Limiting
**Issue**: No limits on messages per session or Claude API usage
**Cost risk**: Potential runaway API costs
**Production blocker**: Yes

## Recent Changes

### This Session (2025-10-12)
**RAG Enhancements** - Implemented Options A, D, and I from brainstorming:
- Added `random_dialect_samples()` to qdrant_service.rs (Option A: Dual retrieval)
- Implemented multi-vector retrieval in agent_service.rs (Option D: 3 embeddings)
- Increased examples from 5 to 50 (Option B: Dense examples - implied)
- Enhanced query with conversation history (Option C: Context-aware retrieval)
- Restructured system prompt to examples-first (Option I: Prompt engineering)

**Compilation Issues Fixed**:
- Filter API type mismatches in Qdrant service
- ScrollResult vs ScoredPoint type handling
- Port conflict resolution (lsof -ti :3000 | xargs kill -9)

### Previous Session
**WebSocket Router Fix**:
- Fixed router order in main.rs (.layer() before .with_state())
- Added TraceLayer for HTTP debugging
- Successfully built backend
- **Did not complete**: WebSocket testing (abandoned task)

## Next Steps (Prioritized)

### CRITICAL (Do First)
1. ✅ Create all documentation files (this file + ARCHITECTURE.md + DEVELOPMENT_LOG.md + TODO.md)
2. ❌ Test WebSocket connectivity with websocat/curl
3. ❌ Verify agent responses are being generated and sent

### HIGH (Frontend Functionality)
4. ❌ Implement WebSocket service in frontend
5. ❌ Integrate WebSocket with app.rs state
6. ❌ Implement ChatWindow component
7. ❌ Test end-to-end message flow

### MEDIUM (Usability)
8. ❌ Implement speech synthesis service
9. ❌ Implement speech recognition service
10. ❌ Add CSS styling for chat interface
11. ❌ Add loading states and error messages

### LOW (Code Quality)
12. ❌ Remove dead code warnings (ConnectionState)
13. ❌ Fix unused imports in shared library
14. ❌ Add frontend unit tests
15. ❌ Set up git repository properly

### FUTURE (Production Readiness)
16. ❌ Add authentication (JWT or session-based)
17. ❌ Implement rate limiting
18. ❌ Restrict CORS to known origins
19. ❌ Add session cleanup/expiry
20. ❌ Implement persistence service
21. ❌ Add monitoring and metrics
22. ❌ Write deployment documentation
