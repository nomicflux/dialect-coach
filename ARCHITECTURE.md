# Dialect Coach - System Architecture

## Overview

Dialect Coach is an AI-powered language learning application that helps users practice authentic dialect variations through conversational AI agents. The system uses RAG (Retrieval-Augmented Generation) to ground agent responses in real dialect corpus examples.

## Technology Stack

### Backend
- **Language**: Rust
- **Web Framework**: Axum 0.8 with WebSocket support
- **AI/LLM**: Anthropic Claude 3.5 Sonnet via Rig framework
- **Vector Database**: Qdrant Cloud (10,911 dialect documents)
- **Embeddings**: Fastembed with MultilingualE5Base model (768-dimensional vectors)
- **Async Runtime**: Tokio

### Frontend
- **Language**: Rust (compiled to WebAssembly)
- **Framework**: Yew 0.21 (CSR mode)
- **Build Tool**: Trunk
- **Browser APIs**: WebSocket, Web Speech API (SpeechSynthesis, SpeechRecognition)
- **Storage**: IndexedDB (via indexed_db_futures)

### Shared Library
- **Models**: Shared data structures (Message, Dialect, ChatSession, etc.)
- **Business Logic**: Chat session management, agent response triggers
- **Serialization**: Serde for JSON

### Corpus Processing
- **Tool**: CLI application for processing dialect corpora
- **Pipeline**: Load → Chunk → Embed → Upload to Qdrant
- **Supported Formats**: Text files, CSV

## Workspace Structure

```
dialect-coach/
├── shared/              # Shared models and business logic
│   ├── models/
│   │   ├── language.rs      # Language enum (Spanish, Arabic, French)
│   │   ├── dialect.rs       # Dialect variants and configuration
│   │   ├── message.rs       # Chat message structure
│   │   ├── session.rs       # ChatSession with context window
│   │   ├── participant.rs   # Human/Agent participant types
│   │   ├── protocol.rs      # WebSocket request/reply protocol
│   │   └── corpus.rs        # DialectDocument for RAG
│   └── logic/
│       ├── chat.rs          # Agent response logic
│       └── context.rs       # Context window management
│
├── backend/             # Axum WebSocket server
│   ├── main.rs              # Server entry point, router setup
│   ├── websocket.rs         # WebSocket handler
│   ├── agent_service.rs     # Claude integration with RAG
│   ├── qdrant_service.rs    # Vector search
│   └── embedding_service.rs # Fastembed wrapper
│
├── frontend/            # Yew WASM application
│   ├── app.rs               # Main app component
│   ├── components/
│   │   ├── chat_window.rs   # Message display (placeholder)
│   │   ├── message_bubble.rs # Individual message component
│   │   ├── input_box.rs     # Text input with submit
│   │   └── speech_controls.rs # Mic button (placeholder)
│   └── services/
│       ├── connection.rs    # The single reconnecting WebSocket connection
│       ├── speech.rs        # Web Speech API (PLACEHOLDER - 9 lines)
│       └── persistence.rs   # IndexedDB (PLACEHOLDER - 9 lines)
│
└── corpus-processor/    # CLI for corpus ingestion
    ├── main.rs              # CLI commands (process, upload, list)
    ├── loaders.rs           # File loading
    ├── chunking.rs          # Text chunking
    ├── embeddings.rs        # Embedding generation
    └── qdrant.rs            # Qdrant upload
```

## Data Flow

### 1. User Message Flow
```
User types/speaks message
    ↓
Frontend InputBox component
    ↓
WebSocket.send(Message JSON)
    ↓
Backend websocket.rs handler
    ↓
Parse Message, extract dialect
    ↓
Store in session history (Arc<Mutex<HashMap<Uuid, Vec<String>>>>)
    ↓
agent_service.generate_response()
```

### 2. RAG Retrieval Flow (Enhanced Strategy)
```
User message + conversation history
    ↓
Build enhanced query (last 2-3 messages)
    ↓
Generate 3 embeddings (multi-vector retrieval):
    • Content embedding (semantic similarity)
    • Style embedding ("casual conversational response in {dialect}")
    • Topic embedding (conversation summary)
    ↓
Parallel retrieval:
    • 10 content-similar examples
    • 10 style-similar examples
    • 10 topic-similar examples
    • 15 random casual/slang examples (dual retrieval)
    ↓
Deduplicate to 50 total examples
    ↓
Group by formality:
    • 25 casual examples
    • 20 slang examples
    • 5 other examples
    ↓
Build RAG context with structured sections
```

### 3. LLM Generation Flow
```
RAG context (50 examples)
    ↓
System prompt:
    "# AUTHENTIC {DIALECT} SPEECH PATTERNS
     [Examples organized by formality]

     # YOUR ROLE
     You are a native {dialect} speaker...

     # CRITICAL RULES
     1. MIMIC THE PATTERNS
     2. STAY CASUAL
     3. NO TEACHING MODE
     4. BE BRIEF
     5. USE DIALECT MARKERS"
    ↓
Claude 3.5 Sonnet (via Rig)
    ↓
Generated dialect response
    ↓
Add to session history
    ↓
Send back via WebSocket as Message JSON
    ↓
Frontend displays in ChatWindow
```

## Key Design Decisions

### 1. Multi-Vector Retrieval
Instead of single embedding search, we generate 3 different embeddings to capture:
- **Semantic content**: What the user is talking about
- **Stylistic cues**: How dialect speakers express casual conversation
- **Topic continuity**: Overall conversation theme

This ensures we retrieve both relevant AND stylistically appropriate examples.

### 2. Dual Retrieval Strategy
Combines:
- **Semantic search**: 30 examples from vector similarity
- **Random sampling**: 15 casual/slang examples for stylistic variety

This prevents over-fitting to semantic similarity while ensuring authentic dialect patterns.

### 3. Dense Examples
Provides 50 total examples (up from original 5) to give the LLM:
- More stylistic patterns to mimic
- Greater vocabulary diversity
- Better coverage of dialect markers

### 4. Examples-First Prompting
System prompt structure:
1. Show all examples first (grouped by formality)
2. Then provide instructions

This makes the LLM focus on pattern mimicry rather than abstract instructions.

### 5. Session-Based Context
Each WebSocket connection maintains conversation history per session_id:
```rust
pub session_histories: Arc<Mutex<HashMap<Uuid, Vec<String>>>>
```
Limited to last 20 messages to prevent unbounded growth.

## WebSocket Protocol

### Message Format
```json
{
  "session_id": "uuid-v4",
  "participant_id": "user1 | agent",
  "content": "message text",
  "dialect": "spanish_mexican",  // Dialect enum (serde ID format)
  "timestamp": "2025-10-12T..."
}
```

### Dialect Serialization
Dialect enum is serialized using serde ID format:
- `spanish_mexican` → SpanishMexican
- `spanish_argentinian` → SpanishArgentinian
- `spanish_colombian` → SpanishColombian
- `spanish_cuban` → SpanishCuban
- `arabic_egyptian` → ArabicEgyptian
- `french_quebecois` → FrenchQuebecois
- etc.

## Supported Dialects

### Spanish (6 variants)
- Mexican (spanish_mexican)
- Castilian/Spain (spanish_castilian)
- Argentinian (spanish_argentinian)
- Cuban (spanish_cuban)
- Chilean (spanish_chilean)
- Colombian (spanish_colombian)

### Arabic (5 variants)
- Egyptian (arabic_egyptian)
- Levantine (arabic_levantine)
- Gulf (arabic_gulf)
- Maghrebi (arabic_maghrebi)
- Iraqi (arabic_iraqi)

### French (5 variants)
- Quebecois (french_quebecois)
- Parisian (french_parisian)
- Swiss (french_swiss)
- Belgian (french_belgian)
- African (french_african)

## Environment Variables

### Backend
```bash
QDRANT_URL=https://xxx.cloud.qdrant.io:6334
QDRANT_API_KEY=xxx
ANTHROPIC_API_KEY=xxx
ANTHROPIC_MODEL=claude-3-5-sonnet-20241022  # optional, defaults to claude-3.5-sonnet
RUST_LOG=debug  # optional, for tracing
```

### Corpus Processor
```bash
QDRANT_URL=https://xxx.cloud.qdrant.io:6334
QDRANT_API_KEY=xxx
```

## Deployment Topology

```
[User Browser]
    ↓ One WebSocket (ws://localhost:3000/ws): every request, reply and usage push
[Backend Server - Axum]
    ↓ HTTPS/gRPC
[Qdrant Cloud] (Vector DB)
    ↓ HTTPS API
[Anthropic Claude API]
```

## Current Status

### ✅ Fully Implemented
- Backend WebSocket server with Axum
- Agent service with Claude integration
- Enhanced RAG with multi-vector retrieval
- Qdrant service with 10,911 documents
- Embedding service with Fastembed
- Corpus processing pipeline
- All shared models and business logic
- Frontend UI components (MessageBubble, InputBox, SpeechControls)

### ⚠️ Partially Implemented
- Frontend app.rs (basic language selector only)
- Frontend components exist but not integrated

### ❌ Not Implemented
- Frontend WebSocket service (9-line placeholder)
- Frontend speech service (9-line placeholder)
- Frontend persistence service (9-line placeholder)
- End-to-end WebSocket testing (NEVER TESTED)

## Known Issues

### Dead Code Warnings
`backend/src/websocket.rs:21` - ConnectionState struct and methods never used
- Was intended for broadcast functionality
- Currently each WebSocket connection is isolated
- Should either implement broadcast or remove

### Compiler Warnings
`shared/src/logic/chat.rs:1` - Unused import: Participant
`shared/src/logic/context.rs:25` - Unused variable: user_message

### Frontend Build Issue
Trunk detects multiple target artifacts:
- "dialect_coach_frontend"
- "dialect-coach-frontend"

May need to specify `data-bin` or `data-target-name` in index.html.

## Performance Characteristics

### Startup Time
- Backend: ~5 seconds (Qdrant connection + Fastembed model loading)
- Frontend: ~2 seconds (WASM compilation + initialization)

### Response Latency
- Embedding generation: ~50-100ms (3 embeddings × 30ms each)
- Qdrant search: ~50-200ms (4 parallel queries)
- Claude generation: ~2-5 seconds (depends on response length)
- **Total**: ~2.5-6 seconds per response

### Memory Usage
- Backend: ~500MB (Fastembed model loaded)
- Qdrant: 10,911 documents × 768 dimensions × 4 bytes = ~33MB vectors

## Security Considerations

### Current Implementation
- CORS: Allow any origin (`CorsLayer::new().allow_origin(Any)`)
- No authentication on WebSocket endpoint
- API keys in environment variables (not encrypted)

### Production Recommendations
- Implement authentication (JWT or session-based)
- Restrict CORS to known frontend origins
- Use secrets management (AWS Secrets Manager, etc.)
- Rate limiting on WebSocket connections
- Input sanitization for user messages
