# Gender Bot Implementation Plan

## User Requirements (from gender_bot.md)

- Agents will need to register whether they are male-presenting or female-presenting
- This depends on the TTS voice
- We need to create a TTS struct with:
  - Provider enum (ElevenLabs, Azure)
  - Voice name (String)
  - Gender (MalePresenting, FemalePresenting)
  - Set all to FemalePresenting for now, I will update with actual values after research
  - Gender is entirely determined by TTS for now
- Gender will need to be fed into the response preamble to guide the response agent to know its gender
  - This is especially important for languages that change declensions and conjugations based on gender

## Current System Analysis

**Existing Types:**
- `shared/src/models/tts.rs`: Contains `TTSProviderType` enum (Azure, ElevenLabs)
- `shared/src/models/dialect.rs`: Contains `Dialect` enum for language variants
- `backend/src/agent_service/response.rs`: Contains `speaker_desc()` and `build_system_content()` that build agent prompts
- Prompts are built in `build_system_content()` and include role description via `speaker_desc()`

**Data Flow:**
1. Dialect enum determines which dialect features to use
2. `speaker_desc()` creates role description like "You are a native X speaker..."
3. This description is fed into system prompt via `build_system_content()`
4. System prompt guides agent responses

## Implementation Design

### Phase 1: Add Gender and TTS Voice Types to Shared Models

**Subagent:** modular-builder

**Code Style Checklist:**
- [ ] Functions <20 lines (prefer <10)
- [ ] Pure functions where possible (data in, data out, no side effects)
- [ ] No defensive coding (trust types, no unnecessary validation)
- [ ] Helper functions for complex logic
- [ ] Low cyclomatic complexity
- [ ] No dead code
- [ ] No fake constructions (no placeholder instances just to satisfy type checker)
- [ ] Tests for all new functions

**Goal:** Define the TTS voice configuration struct with gender presentation.

**Files to create:**
- None (modifying existing)

**Files to modify:**
- `shared/src/models/tts.rs`: Add `Gender` enum and `TTSVoice` struct
- `shared/src/models/dialect.rs`: Modify `get_tts_voices()` and `build_voice_map()` to return/build `TTSVoice` structs
- Any files that call `get_tts_voices()` or use `DialectWithFeatures.tts_voices`

**Implementation steps:**
1. Add `Gender` enum to `shared/src/models/tts.rs`:
   - `MalePresenting`
   - `FemalePresenting`
2. Add `TTSVoice` struct to `shared/src/models/tts.rs`:
   - `provider: TTSProviderType`
   - `voice_name: String`
   - `gender: Gender`
3. MODIFY existing `get_tts_voices()` function in `shared/src/models/dialect.rs` (line 420):
   - Change return type from `HashMap<TTSProviderType, Option<String>>` to `HashMap<TTSProviderType, Option<TTSVoice>>`
   - Update implementation to return `TTSVoice` structs with all `Gender::FemalePresenting` for now
4. MODIFY existing `build_voice_map()` function in `shared/src/models/dialect.rs` (line 407):
   - Change signature to accept voice data and build `TTSVoice` structs
   - Set gender to `FemalePresenting` for all voices
5. Update ALL callers of `get_tts_voices()` to work with new return type
6. Update `DialectWithFeatures` struct (line 386) to use new type
7. Write unit tests for:
   - `Gender` serialization/deserialization
   - `TTSVoice` serialization/deserialization
   - `get_tts_voices()` returns TTSVoice with correct gender

**Deliverables:**
- `Gender` enum with serde traits
- `TTSVoice` struct with serde traits
- Modified `get_tts_voices()` returning `HashMap<TTSProviderType, Option<TTSVoice>>` with FemalePresenting genders
- All callers of `get_tts_voices()` updated to work with new return type
- Tests passing at 100%

**End of Phase:**
- [x] Run FULL test suite: `cargo test` - ALL 370+ TESTS PASSED
- [x] Verify 100% test success (anything less is failure) - 100% SUCCESS
- [x] Run `cargo clippy` and fix ALL errors and warnings - CLEAN
- [x] Remove all dead code (no exceptions, no excuses) - NO DEAD CODE
- [x] Update this status document with progress - UPDATED
- [x] Git add and commit: `git commit -m "Phase 1 (Gender and TTS types) complete"` - COMMITTED (add1f6be)
- [x] **STOP and wait for explicit approval before Phase 2**

**Phase 1 Status: COMPLETE** (2025-11-21)
- Gender enum and TTSVoice struct added
- All voices set to FemalePresenting
- get_tts_voices() modified to return TTSVoice
- All callers updated (backend TTS providers)
- All tests passing, clippy clean

### Phase 2: Update Response Agent to Use Gender in Prompts

**Subagent:** kiss-code-generator

**Code Style Checklist:**
- [ ] Functions <20 lines (prefer <10)
- [ ] Pure functions where possible (data in, data out, no side effects)
- [ ] No defensive coding (trust types, no unnecessary validation)
- [ ] Helper functions for complex logic
- [ ] Low cyclomatic complexity
- [ ] No dead code
- [ ] No fake constructions (no placeholder instances just to satisfy type checker)
- [ ] Tests for all new functions

**Goal:** Modify agent prompt generation to include gender information.

**Files to modify:**
- `backend/src/agent_service/response.rs`: Update `speaker_desc()` to accept and use gender

**Implementation steps:**
1. Update `speaker_desc()` signature:
   - Add parameter `gender: &Gender`
   - Modify role description to include gender: "You are a [male/female]-presenting native X speaker..."
2. Update `build_system_content()`:
   - Extract gender from `dialect` parameter via existing `dialect_features()` and `get_tts_voices()` functions
   - Get first available TTSVoice from the map and extract its gender field
   - Pass gender to `speaker_desc()`
3. Update `GenerateResponseParams` if needed:
   - Check if gender needs to be passed explicitly or derived from dialect
4. Write unit tests for:
   - `speaker_desc()` with `MalePresenting` gender
   - `speaker_desc()` with `FemalePresenting` gender
   - Verify prompt includes gender description

**Deliverables:**
- `speaker_desc()` includes gender in role description
- `build_system_content()` derives and uses gender
- Tests passing at 100%
- Clippy clean with no warnings

**End of Phase:**
- [x] Run FULL test suite: `cargo test` - ALL 370+ TESTS PASSED
- [x] Verify 100% test success (anything less is failure) - 100% SUCCESS
- [x] Run `cargo clippy` and fix ALL errors and warnings - CLEAN
- [x] Remove all dead code (no exceptions, no excuses) - NO DEAD CODE
- [x] Update this status document with progress - UPDATED
- [x] Git add and commit: `git commit -m "Phase 2 (Gender in prompts) complete"` - COMMITTED (a599f672)
- [x] **STOP and wait for explicit approval before Phase 3**

**Phase 2 Status: COMPLETE** (2025-11-21)
- Modified speaker_desc() to accept gender parameter
- Added extract_gender_from_dialect() helper function
- Updated build_system_content() to extract and use gender
- Gender included in agent prompts ("male-presenting" or "female-presenting")
- Fixed test environment pollution in agent_service tests
- All tests passing, clippy clean

### Phase 3: Verification and Documentation

**Orchestrator responsibility** (no subagent)

**Goal:** Verify end-to-end gender flow and document changes.

**Implementation steps:**
1. Run full test suite: `cargo test`
2. Run clippy: `cargo clippy`
3. Verify no dead code remains
4. Update this document with completion status

**Deliverables:**
- All tests passing (100%)
- Clippy clean
- No dead code
- Status document updated

**End of Phase:**
- [x] Run FULL test suite: `cargo test` - ALL 370+ TESTS PASSED
- [x] Verify 100% test success (anything less is failure) - 100% SUCCESS
- [x] Run `cargo clippy` and fix ALL errors and warnings - CLEAN
- [x] Verify no dead code remains - NO DEAD CODE
- [x] Update this status document with final completion status - UPDATED
- [x] Git add and commit: `git commit -m "Phase 3 (Verification) complete - gender_bot feature complete"` - COMMITTED (c81f5cd0)
- [x] **Feature complete**

**Phase 3 Status: COMPLETE** (2025-11-21)
- All tests passing (100%)
- Clippy clean
- No dead code
- End-to-end gender flow verified

## Implementation Summary

**Feature: COMPLETE** (2025-11-21)

All three phases completed successfully:
1. **Phase 1**: Gender and TTSVoice types added to shared models
2. **Phase 2**: Gender integrated into agent response prompts
3. **Phase 3**: End-to-end verification complete

**Key Changes:**
- `Gender` enum: MalePresenting, FemalePresenting
- `TTSVoice` struct: provider, voice_name, gender
- Modified `get_tts_voices()` to return TTSVoice structs
- Modified `speaker_desc()` to include gender in prompts
- All voices currently set to FemalePresenting (ready for user research)

**Testing:**
- 370+ tests passing (100%)
- Clippy clean
- No dead code
- Test environment pollution fixed

## Notes

- TTS struct includes gender as a property of the voice configuration
- All dialects start with `FemalePresenting` voices (user will update with actual voice research later)
- Gender from TTS configuration is fed into agent response preambles for grammatical agreement
- This is about creating the TTS configuration structure and using gender in prompts, not implementing TTS playback
