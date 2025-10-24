# Dialect Coach - Project-Specific Guidelines

## Architecture Overview

**Workspace Structure:**
- `shared/` - Common types and business logic (Message, Dialect, Session, AgentResponse, etc.)
- `backend/` - Axum WebSocket server + AI agent service
- `frontend/` - Yew WASM application
- `corpus-processor/` - CLI tool for dialect data management

**Key Data Types:**
- `Message` - Chat message with `content: AgentResponse`
- `AgentResponse` - Wrapper struct with `response: String` field
- `Dialect` - Enum of supported language variants
- `ChatSession` - Session management with message history

## Development Workflow

**Before any work:**
1. Check if planning doc exists in docs/current-plans/
2. Consider your available tools - which is most appropriate for this task?
3. For architecture or multi-file changes: Create/update planning doc first

**When changing shared types (Message, Dialect, etc.):**
- These types are used across all three crates
- Changes affect: shared code, backend, frontend, and tests in all crates
- Document the scope before making changes

**Testing:**
- Tests exist in all three crates
- Type changes require updating test constructors
- 100% test pass rate required

## Build Commands

```bash
cargo check          # Check all crates
cargo test --lib     # Run library tests
cd backend && cargo run       # Run backend server
cd frontend && trunk serve    # Run frontend dev server
```
