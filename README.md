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
- `cd corpus-processor && cargo run -- process --input corpus-data/spanish_mexican --output processed/es-mx.jsonl` prepares data before `cargo run -- upload --input processed/es-mx.jsonl` syncs to Qdrant. The upload (and other Qdrant commands) require `QDRANT_URL` in `.env` or via the `--url` flag.

## Running with Ollama in Docker

The local Compose overlay selects `qwen3.8:27b-q4_K_M` for the backend's LLM calls, including OCR. It uses the existing frontend, database, and Docker network. Set model tags in `config.ollama.yaml` and keep the model files in the Compose volume.

For the 36 GiB Apple Silicon host, allocate 28 GiB to the existing Colima profile. Restarting Colima briefly interrupts its Docker workloads.

```sh
colima stop && colima start --memory 28 --cpus 8
```

From the repository root, build the backend image and download the model before starting the application.

```sh
docker buildx bake backend
docker compose -f docker-compose.yml -f docker-compose.ollama.yml up -d ollama
docker compose -f docker-compose.yml -f docker-compose.ollama.yml exec ollama ollama pull qwen3.8:27b-q4_K_M
docker compose -f docker-compose.yml -f docker-compose.ollama.yml up -d
```

The frontend opens at [http://localhost:8080](http://localhost:8080). The Ollama container defaults to a 22 GiB memory limit, an 8,192-token context, and a 1 GiB llama.cpp prompt cache. The local OCR channel gives vision requests up to 300 seconds to finish, including model startup. Set `request_timeout_seconds` on a channel to tune its deadline for the runtime and hardware. The backend reads the existing PostgreSQL, Qdrant, and authentication settings.

## Coding Style & Naming Conventions
- Run `cargo fmt` and `cargo clippy --all-targets --all-features` before committing; CI expects Rust 2024 defaults and zero warnings.
- Files and modules use snake_case (e.g., `agent_service.rs`), public types PascalCase (`TeachingMode`), and constants SCREAMING_SNAKE_CASE.
- Prefer explicit `anyhow::Result` returns, instrument async paths with `tracing`, and keep Yew hooks small, composable functions.

## Testing Guidelines
- `cargo test` covers every crate; scope to `cargo test -p backend` or `cargo test -p shared` for targeted runs.
- Integration specs live under `backend/tests/` and workspace-level checks such as `test_unicode_safety.rs`; mirror the `{module}_tests` naming style.
- Exercise the WebSocket manually with `websocat ws://localhost:3000/ws` and the sample `ClientEnvelope` request from `DEVELOPER_GUIDE.md` before PR submission. Every request, reply and usage push travels over this one connection; a reply carries its request's `id`.

## Commit & Pull Request Guidelines
- Follow the concise, imperative commit summaries visible in `git log` (e.g., `"Document provider support"`); wrap detailed explanations in the body when behavior changes.
- PRs must link the tracking issue, outline backend/frontend/corpus impact, call out env vars or datasets touched, and include screenshots or console transcripts for UI/CLI changes.
- Verify `cargo fmt`, `cargo clippy`, relevant `cargo test` targets, and any trunk builds before requesting review.

## Environment & Security Notes
- Store all secrets and instance-specific config in a root `.env` (gitignored); never commit secrets or corpus exports.
- Set `QDRANT_URL` and `QDRANT_API_KEY` for retrieval. Hosted LLM and TTS credentials depend on the configured services. See `DEVELOPER_GUIDE.md` for the full list.
- `docker-compose.yml` reads `POSTGRES_USER`, `POSTGRES_PASSWORD`, and `POSTGRES_DB` from `.env` for the local Postgres instance.
- `config.yaml` holds only non-sensitive configuration (server bind address, LLM model names, TTS settings, rate limits).
- Treat resources inside `corpus-data/` and `corpus-downloads/` as licensed assets—share only processed outputs and keep raw sources in approved storage.
- DO NOT use large blocks or entire files as TargetContent for editing tools. EVen the SLIGHTEST change to newlines and
  formatting makes it impossible for the user to properly review changes.
