# YAML Config Implementation Status

## Overview
Moving non-sensitive configuration from `.env` to `config.yaml`.

## Phase 1: Config Infrastructure in Shared Crate - COMPLETE

### What Was Done
- Added `serde_yaml = "0.9"` and `anyhow.workspace = true` to `shared/Cargo.toml`
- Created `shared/src/config.rs` with all config structs:
  - `AppConfig`, `ServerConfig`, `QdrantConfig`, `PersistenceConfig`
  - `LlmConfig`, `AnthropicProviderConfig`, `OpenAiProviderConfig`
  - `ChannelConfig`, `ChannelsConfig`
  - `TtsConfig`, `ElevenLabsTtsConfig`, `AzureTtsConfig`
  - `RateLimitsConfig`
  - `load_config()` function
- Added `pub mod config;` to `shared/src/lib.rs`
- Created `config.yaml` at repo root with default values

### Tests
- `test_deserialize_full_config` - verifies all fields deserialize from YAML
- `test_optional_fields_deserialize_as_none` - verifies `~` maps to `None`
- `test_optional_fields_deserialize_with_values` - verifies optional fields with values
- `test_load_config_from_file` - verifies loading the committed `config.yaml`
- All 275 shared tests pass (including 4 new config tests)

### Clippy
- Clean, no warnings

## Phase 2: Wire Simple Backend Services - COMPLETE

### What Was Done
- `backend/src/main.rs` - Loads `config.yaml` after dotenvy, passes config sections to init functions, bind address from config
- `backend/src/startup.rs` - Changed signatures: `init_qdrant(url, api_key)`, `init_persistence(database_url, db_path)`, `init_rate_limiter(&RateLimitsConfig)`
- `backend/src/qdrant_service.rs` - Deleted `from_env()` method, updated ignored test to use `new()` directly
- `backend/src/persistence/mod.rs` - Deleted `get_db_path()`, changed `create_persistence()` to accept `database_url` and `db_path` params
- `backend/src/rate_limiter/config.rs` - Deleted `from_env()`, removed `use std::env`, added `from_yaml_config(&RateLimitsConfig)`, added test
- `backend/src/bin/admin.rs` - Loads `config.yaml`, uses `config.server.backend_url`

### Config sources after Phase 2
- Qdrant URL: `config.yaml` / API key: `.env`
- Bind address: `config.yaml`
- DB path: `config.yaml` / DATABASE_URL: `.env`
- Rate limits: `config.yaml`
- Admin backend URL: `config.yaml` / ADMIN_TOKEN: `.env`

### Tests
- All tests pass (0 failures across all crates)
- New test: `test_from_yaml_config` in rate_limiter/config.rs

### Clippy
- Clean, no warnings

## Phase 3: Wire Agent Service + TTS Providers - COMPLETE

### What Was Done
- `backend/src/agent_service.rs` - Deleted `channel_env()`, `load_reasoning_budget()`. Replaced `load_channel_agent()` to accept `&ChannelConfig` + `&LlmConfig`. Replaced `from_env()` with `from_config(&LlmConfig)`. Replaced all tests with config-struct-based tests.
- `backend/src/startup.rs` - Changed `init_agent` to accept `&LlmConfig`, `init_tts` to accept `&TtsConfig`
- `backend/src/main.rs` - Pass `&config.llm` to `init_agent`, `&config.tts` to `init_tts`
- `backend/src/tts_service/eleven_labs_tts_provider.rs` - Replaced `from_env()` with `from_config(&ElevenLabsTtsConfig)`
- `backend/src/tts_service/azure_tts_provider.rs` - Replaced `from_env()` with `from_config(&AzureTtsConfig)`
- `backend/src/bin/rag_tester.rs` - Uses `AgentService::from_config()` with config.yaml

### Config sources after Phase 3
- All channel providers, models, reasoning budgets: `config.yaml`
- All API keys (ANTHROPIC_API_KEY, OPENAI_API_KEY, per-channel overrides): `.env`
- ElevenLabs URL + model: `config.yaml` / API key: `.env`
- Azure region: `config.yaml` / speech key: `.env`

### Tests
- All tests pass (0 failures across all crates)
- Replaced env-var-based agent tests with config-struct-based tests

### Clippy
- Clean, no warnings

## Phase 4: Corpus-Processor + .env Cleanup - COMPLETE

### What Was Done
- `corpus-processor/Cargo.toml` - Added `serde_yaml = "0.9"`
- `corpus-processor/src/main.rs` - Loads `config.yaml` optionally, `get_qdrant_url()` and `get_qdrant_service()` accept config as fallback, all 5 callers updated
- `.env.example` - Replaced with secrets-only version (API keys, JWT_SECRET, ADMIN_TOKEN, DATABASE_URL, per-channel API key overrides)

### Tests
- All 96 corpus-processor tests pass (including 3 new `get_qdrant_url` tests)
- All workspace tests pass (0 failures)

### Clippy
- Clean, no warnings

## Phase 5: Make Models Required + Remove Provider Defaults - COMPLETE

### What Was Done
- `shared/src/config.rs` - Changed `AnthropicProviderConfig.model` and `OpenAiProviderConfig.model` from `Option<String>` to `String`
- `config.yaml` - Changed `openai.model: ~` to `openai.model: "gpt-4o"`
- `backend/src/agent_service/provider.rs` - Changed `anthropic()` and `openai()` constructors from `model: Option<String>` to `model: String`, removed hardcoded `CLAUDE_3_5_SONNET`/`GPT_4O` fallbacks, removed unused imports, deleted default-model tests, updated all remaining tests to pass explicit model strings
- `backend/src/agent_service.rs` - Updated `load_channel_agent` model resolution to use `unwrap_or_else` (channel → provider hierarchy), updated test helper `make_llm_config` with explicit model strings
- `backend/src/agent_service/planning/mod.rs` - Updated test to pass explicit model string
- `backend/src/planning_handler.rs` - Updated test to pass explicit model string
- `corpus-processor/src/main.rs` - Updated test `AppConfig` construction with explicit model strings

### Tests
- All workspace tests pass (217 backend + shared + frontend)
- All 10 corpus-processor tests pass
- Clippy clean

## Phase 6: Eliminate Hardcoded Defaults and Silent Failures - COMPLETE

### What Was Done
- `backend/src/rate_limiter/config.rs` - Deleted `from_fetch()`, `parse_value()`, `Default` impl, and 4 associated tests. Only `new()`, `from_yaml_config()`, and `test_from_yaml_config` remain.
- `backend/src/rate_limiter/service.rs` - Replaced `RateLimitConfig::default()` with `load_test_rate_limit_config()` helper that loads from config.yaml. Tests now use single source of truth.
- `backend/src/tts_handler.rs` - Replaced `RateLimitConfig::default()` with config.yaml loading in test.
- `corpus-processor/src/main.rs` - Config loading moved after `Cli::parse()` (so `--help` and arg validation work without config). Commands that need config (`Delete`, `Upload`, `Update`, `Status`, `ResetCollection`) call `load_required_config()` which fails fast with descriptive error. `Process` and `List` don't require config. `get_qdrant_url` signature changed to `&AppConfig` (infallible return). `get_qdrant_service` signature changed to `&AppConfig`.
- `corpus-processor/tests/test_cli_integration.rs` - Deleted 4 tests (`test_cli_upload_missing_qdrant_url`, `test_cli_upload_empty_qdrant_url`, `test_cli_status_missing_qdrant_url`, `test_cli_status_empty_qdrant_url`) — these tested scenarios that can't occur now that config.yaml always provides a qdrant URL.
- `backend/src/qdrant_service.rs` - Changed `search_named_vector` from `.map()` to `.filter_map()`, replacing `unwrap_or(Dialect::SpanishMexican)` with `tracing::warn` + skip for unrecognized dialects.

### Tests
- All workspace tests pass (210 backend + 275 shared + 9 frontend)
- All 91 corpus-processor tests pass (9 CLI integration + 82 unit/integration)
- Clippy clean

## Final State

All non-sensitive configuration now lives in `config.yaml`. `.env` contains only secrets:
- API keys: QDRANT_API_KEY, ANTHROPIC_API_KEY, ANTHROPIC_ADMIN_API_KEY, OPENAI_API_KEY, ELEVEN_LABS_API_KEY, AZURE_SPEECH_KEY
- Auth: JWT_SECRET, ADMIN_TOKEN
- Database: DATABASE_URL (contains embedded credentials)
- Per-channel: {CHANNEL}_API_KEY overrides
