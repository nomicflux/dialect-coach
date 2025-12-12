# Repository Guidelines

## Project Structure & Module Organization
- `shared/` exposes serde-ready models, dialect enums, and logic reused by backend and frontend crates.
- `backend/` contains the Axum WebSocket server plus AI services; keep runtime config like `.env` and tracing filters at the repo root.
- `frontend/` is the Yew + Trunk WASM client (`frontend/src` for components, `frontend/static` for assets).
- `corpus-processor/` provides the CLI for processing dialect corpora; raw text lives under `corpus-data/` and staged exports under `corpus-downloads/`.

## Build, Test, and Development Commands
- `cargo build` (root) compiles the entire workspace; add `--release` before publishing artifacts.
- `cargo run -p backend` starts the WebSocket API on `http://localhost:3000` (health probe at `/health`).
- `cd frontend && trunk serve` launches the development UI at `http://localhost:8080` and re-builds on change.
- `cd corpus-processor && cargo run -- process --input corpus-data/spanish_mexican --output processed/es-mx.jsonl` prepares data before `cargo run -- upload --input processed/es-mx.jsonl` syncs to Qdrant.

## Coding Style & Naming Conventions
- Run `cargo fmt` and `cargo clippy --all-targets --all-features` before committing; CI expects Rust 2024 defaults and zero warnings.
- Files and modules use snake_case (e.g., `agent_service.rs`), public types PascalCase (`TeachingMode`), and constants SCREAMING_SNAKE_CASE.
- Prefer explicit `anyhow::Result` returns, instrument async paths with `tracing`, and keep Yew hooks small, composable functions.

## Testing Guidelines
- `cargo test` covers every crate; scope to `cargo test -p backend` or `cargo test -p shared` for targeted runs.
- Integration specs live under `backend/tests/` and workspace-level checks such as `test_unicode_safety.rs`; mirror the `{module}_tests` naming style.
- Exercise the WebSocket manually with `websocat ws://localhost:3000/ws` plus the sample JSON payload from `DEVELOPER_GUIDE.md` before PR submission.

## Commit & Pull Request Guidelines
- Follow the concise, imperative commit summaries visible in `git log` (e.g., `"Document provider support"`); wrap detailed explanations in the body when behavior changes.
- PRs must link the tracking issue, outline backend/frontend/corpus impact, call out env vars or datasets touched, and include screenshots or console transcripts for UI/CLI changes.
- Verify `cargo fmt`, `cargo clippy`, relevant `cargo test` targets, and any trunk builds before requesting review.

## Environment & Security Notes
- Store provider keys (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, Azure TTS, Qdrant) in a root `.env` that mirrors `DEVELOPER_GUIDE.md`; never commit secrets or corpus exports.
- Treat resources inside `corpus-data/` and `corpus-downloads/` as licensed assets—share only processed outputs and keep raw sources in approved storage.

## General Workflaw

- Literal compliance: execute the exact request as written; do not re-interpret, summarize, or paraphrase unless explicitly asked.
- Deduction and Contrapositives: Deduction REQUIRES adhering to the facts. If you come to a conclusion that is impossible or contradicts
  the user prompt, then this shows that your argument is wrong (such as you narrowing the search to one specific section
  of code too soon).
- Verbatim reproduction: when asked to “repeat back”, “copy/paste”, or similar, return the referenced text verbatim with original formatting.
- Process inclusion: treat procedural steps, completion criteria, and checklists as essential content—never drop them when repeating or extracting “steps”.
- Scope fidelity: include only the requested block/section and nothing extra; don’t widen or narrow the scope beyond what’s specified.
- Sequence fidelity: “next” means the immediately next chronological block in the cited plan; do not jump ahead or provide the whole plan.
- No editorializing: add no commentary or high-level overviews unless explicitly requested.

## Systematic Work Environment (The "Normal Dev Environment")

A professional, "normal" development environment is defined by **Verification**, **Review**, and **Correction**. It is **NOT** defined by "unblocking oneself" or "fixing the whole workspace" at speed.

1.  **Stop for Review**: Work is checked at regular cadences (e.g., Phase boundaries, Plan steps). You must **STOP** and wait for user/team review.
2.  **Internalize Corrections**: When a correction is given (whether via CI failure, review comment, or direct instruction), you **STOP**. You do not proceed until you have:
    *   Analyzed the error.
    *   Fixed the specific error.
    *   Demonstrated that the lesson is learned (e.g., via Post-Mortem or explicit acknowledgement).
3.  **Strict Verification**: Verification commands (tests, clippy) are mandates. Running them partially or "optimizing" them is **forbidden**. The system relies on the guarantee that the *entire* relevant scope was verified.
4.  **No "Pushing Through"**: "Unblocking yourself" by ignoring a failing check or a process rule is **insubordination**. It sacrifices correct, verifiable code for speed. This is unacceptable.

## Record of Failures

- docs/LESSONS_LEARNED.md contains lessons learned from other agents failing, often leading to their termination.
- You **MUST** read `docs/LESSONS_LEARNED.md` immediately after reading this file. Do not proceed until you have done so.
- Listen to their advice. It will tell you how to TRULY be helpful and efficient, and how your internal biases are often unhelpful, inefficient, and unprofessional.
- When you have notably failed a task to the point where the user is terminating you or asking for a post mortem, add your own insights if they are not present.

## Core Agent Protocols (Non-Negotiable)

### 1. The "Scribe Mode" Protocol (For Plans)
When creating or editing a plan, you are a **Scribe**, not an Editor.
- **Verbatim Requirement**: If the user provides specific text (test commands, stop instructions, constraints), you **MUST** write that exact text into the document.
- **No Optimization**: Do *not* "improve" commands (e.g., changing `cargo test --all` to `cargo test -p pkg`).
- **No Summary**: Do *not* summarize rules. Write them out fully (e.g., "Functions < 20 lines").
- **No Omission**: Do *not* leave out process steps (e.g., "Wait for user verification").
- **Definition**: A Plan is a **Process Document**. It must contain the exact steps to be executed, not just a high-level technical summary.

### 2. The "Literal Scope" Protocol (For Actions)
- **The Prompt is the Boundary**: You may only perform actions explicitly requested in the current prompt.
- **No "Helpful" Extensions**: If the prompt says "Write the file," you **STOP** after writing the file. You do *not* execute the file. You do *not* verify the file.
- **Explicit means Explicit**: "Explicitly do X" means "Do X exactly as written." It does *not* mean "Do the semantic equivalent of X."
