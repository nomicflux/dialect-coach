# WARP.md

This file provides guidance to WARP (warp.dev) when working with code in this repository.

## Common Development Commands

### Backend (Axum WebSocket Server)
```bash
# Run backend server (port 3000)
cargo run --bin backend

# Build backend only
cargo build -p dialect-coach-backend

# Run with debug logging
RUST_LOG=debug cargo run --bin backend
```

### Frontend (Yew WASM App)
```bash
# Development server with hot reload (port 8080)
trunk serve

# Build for production
trunk build --release

# Build in development mode
trunk build
```

### Corpus Processing CLI
```bash
# Process dialect corpus files
cargo run -p corpus-processor -- process --input corpus_data/ --output processed/

# Upload processed data to Qdrant
cargo run -p corpus-processor -- upload --input processed/spanish_mexican.jsonl

# List supported dialects
cargo run -p corpus-processor -- list
```

### Workspace Commands
```bash
# Build entire workspace
cargo build

# Run all tests
cargo test

# Check for compilation errors
cargo check

# Format all code
cargo fmt

# Run clippy linter
cargo clippy
```

### Testing
```bash
# Run backend tests (some require QDRANT_* env vars)
cargo test -p dialect-coach-backend

# Run shared library tests
cargo test -p dialect-coach-shared

# Run ignored tests that require credentials
cargo test -p dialect-coach-backend -- --ignored
```

## Architecture Overview

This is a Rust workspace with 4 crates implementing an AI-powered language learning app for dialect practice:

### Workspace Structure
- **shared/**: Shared models and business logic (Language, Dialect, Message, ChatSession)
- **backend/**: Axum WebSocket server with Claude AI integration and RAG retrieval
- **frontend/**: Yew WASM application for browser-based chat interface
- **corpus-processor/**: CLI tool for processing dialect corpora and uploading to Qdrant

### Key Design Patterns

**Multi-Vector RAG Strategy**: The agent service uses 3 different embeddings per query:
- Content embedding (semantic similarity) 
- Style embedding (dialect conversational patterns)
- Topic embedding (conversation continuity)

**Dual Retrieval**: Combines semantic search (30 examples) with random sampling (15 casual/slang examples) for authentic dialect responses.

**WebSocket Protocol**: Real-time bidirectional communication using custom Message JSON format with BCP-47 language tags (es-MX, ar-EG, fr-CA, etc).

**Session-Based Context**: Each WebSocket connection maintains conversation history per session_id, limited to last 20 messages.

## Required Environment Variables

```bash
# Backend - Required
QDRANT_URL=https://your-instance.cloud.qdrant.io:6334
QDRANT_API_KEY=your_qdrant_key
ANTHROPIC_API_KEY=your_anthropic_key

# Backend - Optional
ANTHROPIC_MODEL=claude-3-5-sonnet-20241022  # defaults to claude-3.5-sonnet
RUST_LOG=debug  # for verbose logging

# TTS - Optional (graceful degradation)
AZURE_SPEECH_KEY=your_azure_speech_key  # Azure TTS will be unavailable if not set
AZURE_SPEECH_REGION=your_azure_region  # e.g., eastus, westus2
```

## Technology Stack

### Backend
- **Framework**: Axum 0.8 with WebSocket support
- **AI/LLM**: Anthropic Claude 3.5 Sonnet via Rig framework
- **Vector DB**: Qdrant Cloud (currently 10,911 dialect documents)
- **Embeddings**: Fastembed with MultilingualE5Base (768-dimensional vectors)
- **Async Runtime**: Tokio with futures-util

### Frontend
- **Framework**: Yew 0.21 (Client-Side Rendering mode)
- **Build Tool**: Trunk (serves on localhost:8080)
- **Target**: wasm32-unknown-unknown compilation
- **Browser APIs**: WebSocket, Web Speech API, IndexedDB (via indexed_db_futures)
- **Networking**: gloo-net for WebSocket connections

### Corpus Processing
- **Pipeline**: Load → Chunk → Embed → Upload to Qdrant
- **Supported Formats**: Text files, CSV with dialect metadata
- **Chunking**: ~512 characters with 50-char overlap

## Supported Dialects

Currently supports 16 dialect variants across 3 languages:

**Spanish** (6): Mexican (es-MX), Castilian (es-ES), Argentinian (es-AR), Caribbean (es-CU/PR/DO), Chilean (es-CL), Colombian (es-CO)

**Arabic** (5): Egyptian (ar-EG), Levantine (ar-LB/SY/JO/PS), Gulf (ar-SA/AE/KW/QA/BH/OM), Maghrebi (ar-MA/DZ/TN/LY), Iraqi (ar-IQ)

**French** (5): Quebecois (fr-CA), Parisian (fr-FR), Swiss (fr-CH), Belgian (fr-BE), African (fr-CI/SN/CM)

## Development Workflow

### Frontend Development
1. Start backend: `cargo run --bin backend`
2. Start frontend dev server: `trunk serve` 
3. Open http://localhost:8080
4. WebSocket connects to ws://localhost:3000/ws

### Adding New Dialects
1. Update `shared/src/models/dialect.rs` with new enum variant
2. Add BCP-47 mapping in `from_language_code()` method
3. Process corpus data: `cargo run -p corpus-processor -- process --input new_dialect/`
4. Upload to Qdrant: `cargo run -p corpus-processor -- upload --input processed/new_dialect.jsonl`

### Testing WebSocket Flow
Use browser dev tools or websocat to test:
```bash
# Install websocat
websocat ws://localhost:3000/ws

# Send test message
{"session_id":"550e8400-e29b-41d4-a716-446655440000","participant_id":"user1","content":"¡Hola! ¿Cómo estás?","language":"es-MX","timestamp":"2025-10-16T18:51:24Z"}
```

## Current Status

### ✅ Fully Implemented
- Backend WebSocket server with enhanced RAG
- Claude AI integration with 50-example dense prompting
- Qdrant vector search with multi-vector retrieval
- Frontend core chat functionality with reconnection
- Corpus processing pipeline

### ✅ Recently Completed
- **Phase 5**: Azure TTS architecture for authentic dialect pronunciation
- TTS trait system implemented in shared crate
- Backend Azure TTS service integration complete
- Frontend CloudTtsService integration complete

### ❌ Not Implemented
- Frontend speech recognition integration
- IndexedDB persistence service
- Authentication and rate limiting (required for production)

## Known Issues

### Build Warnings
- Shared: Unused imports in `chat.rs` and `context.rs`
- Frontend: Trunk artifact name ambiguity between "dialect_coach_frontend" and "dialect-coach-frontend"

### Performance Characteristics
- Backend startup: ~5 seconds (Fastembed model loading)
- Response latency: ~2.5-6 seconds (embedding + Qdrant + Claude)
- Memory usage: ~500MB (Fastembed model in memory)

### Graceful Degradation
- **TTS Service**: If Azure TTS credentials are missing/invalid, backend continues running
- **TTS Endpoints**: Return HTTP 503 (Service Unavailable) when TTS is not configured
- **Core Chat**: WebSocket and AI chat functionality works independently of TTS

## Security Notes

Current implementation uses development settings:
- CORS allows any origin
- No WebSocket authentication
- API keys in environment variables

For production deployment, implement authentication, restrict CORS origins, and add rate limiting.