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

**Checking a UI feature requires visual confirmation. No exceptions.**
Look at the rendered result in a browser as part of every UI check. The only
excuse is having literally no access to any view. Measurements, computed
styles, bounding boxes, `scrollWidth`, passing tests, and a successful deploy
are annotations on a screenshot - none of them is the check, and none may
stand in for it. A number that reads clean while the pixels are wrong is the
normal case, not the rare one: a rope whose bounding box reported
`clippedLeft: false` was rendering cut flat at the viewport edge.

## Build Commands

```bash
cargo check    # Check all crates
cargo test     # Run library
cd backend && cargo run       # Run backend server
cd frontend && trunk serve    # Run frontend dev server
```

## Code Modification Rules

### CRITICAL: Modify Existing Code In Place - Never Create Parallel Implementations

When asked to change how existing functionality works:

**ALWAYS:**
1. **Modify the existing function in place** - change its implementation, signature, return type as needed
2. **Update ALL callers in the same phase** - if signature changes, every callsite must be updated
3. **Update ALL tests for that function** - tests must reflect the new behavior
4. **Delete any code made obsolete by the changes**

**NEVER:**
1. Create a new function with a similar name/purpose alongside the old one
2. Leave old implementations "for compatibility" or "for tests"
3. Allow old and new versions to coexist
4. Update only some callers to use new version while leaving others on old

**Red Flags - If you see these, you're doing it wrong:**
- Creating functions like `foo_v2()`, `new_foo()`, `foo_completion()` when `foo()` already exists
- Compiler warnings about unused functions that were previously in use
- Two functions that do similar things with different APIs (e.g., `retry_chat_call` and `retry_completion_call`)
- Thinking "I'll create new version and update callers later"

**The Principle:**
Code evolves in place. When requirements change, existing code adapts. Rewriting alongside existing code creates divergent implementations, abandoned tests, and technical debt.

**Exception:**
The ONLY time to create a new function is when adding genuinely NEW functionality that doesn't replace anything existing.

## General Instructions

- You are responsible for everything in all relevent CLAUDE.md files. You will be tested on material in them. Failure to
  address anything in any CLAUDE.md file will be grounds for termination.
