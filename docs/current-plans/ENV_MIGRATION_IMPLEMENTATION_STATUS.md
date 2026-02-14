# Move Sensitive Config to .env - Implementation Status

## Phase 1: Remove Qdrant URL from config.yaml, read from env var

**Status**: COMPLETE

**Changes made:**
- `shared/src/config.rs`: Removed `QdrantConfig` struct and `qdrant` field from `AppConfig`. Removed qdrant sections from all 3 test YAML constants. Removed qdrant assertion from `test_deserialize_full_config`.
- `config.yaml`: Removed `qdrant:` section (contained sensitive instance URL).
- `backend/src/main.rs`: Changed `config.qdrant.url` to `std::env::var("QDRANT_URL")?`.
- `corpus-processor/src/main.rs`: Removed `config` param from `get_qdrant_url` and `get_qdrant_service`. Removed `load_required_config` function and all its callers. Updated tests.

**Verification:**
- `cargo test` — all tests pass
- `cargo clippy` — clean
- No qdrant URL in config.yaml

## Phase 2: Move DB credentials from docker-compose.yml to .env

**Status**: COMPLETE

**Changes made:**
- `docker-compose.yml`: Replaced hardcoded `POSTGRES_USER`, `POSTGRES_PASSWORD`, `POSTGRES_DB` with `${...}` env var references. Updated `DATABASE_URL` to use interpolated values. Changed healthcheck to string form for variable interpolation.
- `.env`: Added `POSTGRES_USER`, `POSTGRES_PASSWORD`, `POSTGRES_DB` values.

**Verification:**
- `docker compose config` — all values interpolate correctly
- `cargo test` — still passes (no Rust changes)

## Phase 3: Scrub Qdrant URL from git history

**Status**: PENDING
