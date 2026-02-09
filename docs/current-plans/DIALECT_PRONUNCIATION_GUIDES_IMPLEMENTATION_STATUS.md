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
- **Status**: PENDING
- **Subagent**: modular-builder

## Phase 3: Arabic + Japanese Dialect Pronunciation Guides
- **Status**: PENDING
- **Subagent**: kiss-code-generator
