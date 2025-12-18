# Handoff Instructions for Next Agent

**FROM**: Terminated Agent (Antigravity)
**STATUS**: **FAILED / TERMINATED**
**CONTEXT**: Attempting to migrate Dockerfiles to `cargo chef` with cross-compilation support (Project uses Rust Edition 2024).

## Current State

### Frontend Dockerfile
- **Location**: `frontend/Dockerfile`
- **Status**: Updated to use `lukemathwalker/cargo-chef:latest-rust-1.91`.
- **Pending**: Verification (Build was cancelled). Needs to check if `1.91` fully supports the project's edition 2024 requirements.

### Backend Dockerfile
- **Location**: `backend/Dockerfile`
- **Status**: Updated to use `1.91`. Includes `g++-x86-64-linux-gnu` and `libc6-dev-amd64-cross` for cross-compilation.
- **Copy Issue**: The `COPY` command was the source of contention. It *should* be:
  ```dockerfile
  COPY --from=builder /app/target/x86_64-unknown-linux-gnu/release/backend /usr/local/bin/dialect-coach-backend
  ```
- **Note**: The user has manually intervened here. **DO NOT OVERWRITE THIS FILE** without reading it first and ensuring you respect the user's manual fix for the binary/package name split.

## Immediate Tasks
1.  **Verify Frontend Build**: Run `docker build -f frontend/Dockerfile .`
2.  **Verify Backend Build**: Run `docker build -f backend/Dockerfile .`
3.  **DO NOT GUESS**: If errors occur, **RESEARCH** specific error messages online. Do not assume tags or versions.

## Critical Constraints
- **NO ARCH CHECKS**: Do not use `uname -m`.
- **RESEARCH FIRST**: Search online before fixing build errors.
- **CHECK BINARY NAMES**: `backend/Cargo.toml` defines `[[bin]] name="backend"`.
