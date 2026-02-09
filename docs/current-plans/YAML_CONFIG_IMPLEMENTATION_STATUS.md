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

## Phase 2: Wire Simple Backend Services - PENDING

## Phase 3: Wire Agent Service + TTS Providers - PENDING

## Phase 4: Corpus-Processor + .env Cleanup - PENDING
