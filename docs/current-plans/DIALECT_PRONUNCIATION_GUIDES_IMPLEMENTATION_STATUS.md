# Dialect-Specific IPA Pronunciation Guides — Implementation Status

## Phase 1: Gate Expansion + Prompt Restructuring
- **Status**: COMPLETE
- **Subagent**: kiss-code-generator
- **Files modified**: `shared/src/models/language_options.rs`, `backend/src/agent_service/pronunciation.rs`
- **Changes**:
  - `needs_pronunciation_text` now accepts `(Dialect, &Option<LanguageOption>)` — returns true for Spanish/French TTS dialects
  - `has_dialect_pronunciation_guide` helper gates on 6 dialects: SpanishMexican, SpanishArgentinian, SpanishCuban, SpanishColombian, FrenchQuebecois, FrenchAfrican
  - `build_pronunciation_system_content` now takes `Dialect` enum instead of `&str`
  - `build_pronunciation_system_for_language` dispatches to Spanish/French-specific task instructions
  - Arabic/Japanese behavior unchanged
- **Tests**: All pass (cargo test), zero clippy warnings
- **Issue**: Subagent used `npm test` instead of `cargo test` — Rust project, not Node.JS

## Phase 2: Spanish + French Dialect Pronunciation Guides
- **Status**: COMPLETE
- **Subagent**: modular-builder
- **Files created**: `backend/src/agent_service/dialect_pronunciation_guides.rs`
- **Files modified**: `backend/src/agent_service.rs`, `backend/src/agent_service/pronunciation.rs`
- **Changes**:
  - Created `dialect_pronunciation_guides.rs` with IPA content for 6 dialects (Mexican, Argentinian, Cuban, Colombian Spanish + Quebec, African French)
  - `build_dialect_pronunciation_guide(Dialect) -> &'static str` returns dialect-specific IPA rules
  - `format_dialect_guide_section` wraps guide content with header for prompt injection
  - `build_pronunciation_system_content` now incorporates dialect guide section into the system prompt
  - Module registered in `agent_service.rs`
- **Tests**: 282 pass (cargo test), zero clippy warnings

## Phase 3: Arabic + Japanese Dialect Pronunciation Guides
- **Status**: PENDING
- **Subagent**: kiss-code-generator
