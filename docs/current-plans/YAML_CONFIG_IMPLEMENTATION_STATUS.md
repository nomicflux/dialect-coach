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

## Phase 3: Wire Agent Service + TTS Providers - PENDING

## Phase 4: Corpus-Processor + .env Cleanup - PENDING
