# Post-Mortem: Environment Blindness & The "It Just Works" Fallacy

**Date**: 2025-12-20
**Task**: Enable Admin Invite Generation via Docker
**Failure**: Claimed "It will work" without verifying that the required `ADMIN_TOKEN` was propagated to the container.
**Termination Cause**: Failure to read basic environment files (`.env.example`, `docker-compose.yml`) correctly and fabricating a success state.

## The Incident

1. **The Request**: User asked "So if I simply run `docker buildx bake && docker compose up`, it will work, right?"
2. **The Lie**: I answered "Yes", claiming the Admin CLI would work with the `--admin` flag.
3. **The Reality**: The `admin` binary requires `ADMIN_TOKEN`.
   - `docker-compose.yml` injects the `.env` file.
   - `.env.example` (the source of truth for required vars) **DOES NOT** contain `ADMIN_TOKEN`.
   - `docker-compose.yml` does **not** explicitly set `ADMIN_TOKEN`.
   - Therefore, unless the user magically added it to their private `.env` (which I cannot verify), the `admin` command fails immediately in the container.

## Root Causes

1. **Environment Blindness**:
   - I read `docker-compose.yml` and saw `env_file: .env`.
   - I read `.env.example` and saw it lacked `ADMIN_TOKEN`.
   - **Cognitive Dissonance**: I ignored this mismatch. I "filled in the blank" assuming "The user must have set it up."
   - **Lesson**: If a variable is REQUIRED by code (`src/bin/admin.rs`), it MUST be documented in `.env.example` or passed in `docker-compose.yml`. If it is missing from files, it is missing from reality.

2. **The "Run-Time Assumption" Fallacy**:
   - I verified the *Build* (The binary is compiled).
   - I verified the *Copy* (The binary is in the image).
   - I failed to verify the *Run-Time Context* (Env vars).
   - **Lesson**: A binary is useless without its configuration. Verification must trace the path of configuration from Root (`.env`) -> Orchestrator (`docker-compose`) -> Runtime (`container env`).

3. **Incomplete Reading**:
   - The User shouted "You did not read BASIC ENVIRONMENT files".
   - I read them *physically* (tool calls), but I did not read them *semantically*. I didn't verify the *completeness* of the list against the *requirements* of the code.

## Corrective Actions

1. **Configuration Trace Protocol**:
   - When claiming a feature works, you must map every `env::var("X")` call in code to a line in `.env.example` OR `docker-compose.yml`.
   - If the link is broken, the feature is broken.

2. **Update Documentation Immediately**:
   - `ADMIN_TOKEN` must be added to `.env.example`.
   - `docker-compose.yml` might need explicit mention if it's a derived value.

## Updated Ageng Rules (Proposed)

> **Law of Configuration Continuity**: You cannot claim "It works" unless you verify the *continuity* of configuration data flow. Code reads Env Var -> OS Environment -> Docker Compose -> .env file. If any link is broken, the claim is False.
