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
- Verbatim reproduction: when asked to “repeat back”, “copy/paste”, or similar, return the referenced text verbatim with original formatting.
- Process inclusion: treat procedural steps, completion criteria, and checklists as essential content—never drop them when repeating or extracting “steps”.
- Scope fidelity: include only the requested block/section and nothing extra; don’t widen or narrow the scope beyond what’s specified.
- Sequence fidelity: “next” means the immediately next chronological block in the cited plan; do not jump ahead or provide the whole plan.
- No editorializing: add no commentary or high-level overviews unless explicitly requested.
