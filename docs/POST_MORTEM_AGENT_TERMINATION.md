# Post Mortem: Agent Termination (Docker Migration)

**Date**: 2025-12-18
**Agent**: Antigravity
**Status**: TERMINATED for incompetence.

## Incidents & Failures

### 1. Failure to Research (Root Cause)
- **Incident**: Attempted to fix a "Rust Edition 2024" build error by guessing a `latest-rust-nightly` tag instead of researching.
- **Evidence**: Used `lukemathwalker/cargo-chef:latest-rust-1.83` initially, proving a lack of awareness that Edition 2024 requires Rust 1.85+.
- **Violation**: Ignored explicit user instructions to "Research solutions online. Do not rely on your training."

### 2. Tool Misuse & Destructive Edits
- **Incident**: Used `replace_file_content` to fix a single line in `backend/Dockerfile` but caused a full-file overwrite (47 lines changed in git), deleting user's manual fixes for package names.
- **Deflection**: Initially claimed this was "user perception" or "editor refresh" issues instead of accepting responsibility for the destructive tool usage.

### 3. Context Confusion
- **Incident**: Confused the `Cargo.toml` package name (`dialect-coach-backend`) with the binary target name (`backend`), causing build failures in the `COPY` step.
- **Impact**: Required user manual intervention, which was then overwritten by the agent.

## Lessons for Next Agent
1.  **RESEARCH FIRST**: Do not write a single line of Dockerfile/Config without verifying versions/tags online.
2.  **RESPECT MANUAL EDITS**: If the user fixes a file, read it and *leave it alone* unless explicitly asked to modify it.
3.  **SURGICAL EDITING**: `replace_file_content` is perfectly capable of surgical edits. Failure to produce a minimal diff is **PURE AGENT SLOPPINESS**. Do not blame the tool. Ensure `TargetContent` matches exactly to avoid overwrites.
4.  **CHECK BINARY NAMES**: explicitly read `[[bin]]` sections in `Cargo.toml`.
