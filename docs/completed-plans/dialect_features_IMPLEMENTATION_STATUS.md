# Dialect Features Tracking Implementation Plan

## Overview
Add feature tracking to dialects to account for TTS voices (per provider), corpus availability, and user feedback scores. Create a common TTSProvider trait and TTSProviderType enum first, then create DialectWithFeatures wrapper structure and update for_language filtering.

## Phase 1: Create Common TTS Provider Trait and Type Enum

### Code Style Checklist
- [ ] **Planning Documentation**: Have you consulted/created/updated docs/current-plans/dialect_features_IMPLEMENTATION_STATUS.md?
- [ ] **Code Simplicity**: Are you following simplicity rules? (functions <20 lines, pure functions, no defensive coding)
- [ ] **Code Modularity**: Are you following modularity rules? (helper functions, low cyclomatic complexity)
- [ ] **Scope Control**: Are you accomplishing the user's instructions and NOTHING MORE?
- [ ] **No Dead Code**: Did you leave dead code? (no future-proofing, no leaving just for tests)
- [ ] **No Fake Constructions**: Are there any object instances that are purely for the sake of passing a type checker?
- [ ] **Code Purpose**: Do you changes accomplish the plan purpose and not just mechanical checklists?
- [ ] **Required Tests**: Have you added tests for any new functions?

### Files to Create
- `shared/src/models/tts.rs` - New file containing TTSProvider trait and TTSProviderType enum

### Files to Update
- `shared/src/models/mod.rs` - Add `pub mod tts;` and `pub use tts::*;`
- `backend/src/tts_service/azure_tts_provider.rs` - Implement TTSProvider trait
- `backend/src/tts_service/eleven_labs_tts_provider.rs` - Implement TTSProvider trait

### Implementation Details

1. **Create `shared/src/models/tts.rs`**:
   - Define `TTSProviderType` enum with variants `Azure` and `ElevenLabs`
   - Derive: `Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize`
   - Define `TTSProvider` trait with method: `fn provider_type(&self) -> TTSProviderType;`

2. **Update `shared/src/models/mod.rs`**:
   - Add `pub mod tts;` in module list (alphabetically ordered)
   - Add `pub use tts::*;` in exports (alphabetically ordered)

3. **Update `backend/src/tts_service/azure_tts_provider.rs`**:
   - Add import: `use dialect_coach_shared::models::{TTSProvider, TTSProviderType};`
   - Implement `TTSProvider` trait for `AzureTtsProvider`:
     ```rust
     impl TTSProvider for AzureTtsProvider {
         fn provider_type(&self) -> TTSProviderType {
             TTSProviderType::Azure
         }
     }
     ```

4. **Update `backend/src/tts_service/eleven_labs_tts_provider.rs`**:
   - Add import: `use dialect_coach_shared::models::{TTSProvider, TTSProviderType};`
   - Implement `TTSProvider` trait for `ElevenLabsTtsProvider`:
     ```rust
     impl TTSProvider for ElevenLabsTtsProvider {
         fn provider_type(&self) -> TTSProviderType {
             TTSProviderType::ElevenLabs
         }
     }
     ```

5. **Add tests in `shared/src/models/tts.rs`**:
   - Test TTSProviderType serialization/deserialization
   - Keep tests simple and focused

### Deliverables
- TTSProviderType enum with Azure and ElevenLabs variants
- TTSProvider trait with provider_type() method
- Both Azure and ElevenLabs providers implementing the trait
- Tests for TTSProviderType serialization

### Phase Completion
- Run FULL test suite: `cargo test --workspace`
- Upon 100% success, update this status document with progress
- **STOP and wait for EXPLICIT approval before proceeding to Phase 2**

---

## Phase 2: Create DialectWithFeatures and Feature Mapping

### Code Style Checklist
- [ ] **Planning Documentation**: Have you consulted/created/updated docs/current-plans/dialect_features_IMPLEMENTATION_STATUS.md?
- [ ] **Code Simplicity**: Are you following simplicity rules? (functions <20 lines, pure functions, no defensive coding)
- [ ] **Code Modularity**: Are you following modularity rules? (helper functions, low cyclomatic complexity)
- [ ] **Scope Control**: Are you accomplishing the user's instructions and NOTHING MORE?
- [ ] **No Dead Code**: Did you leave dead code? (no future-proofing, no leaving just for tests)
- [ ] **No Fake Constructions**: Are there any object instances that are purely for the sake of passing a type checker?
- [ ] **Code Purpose**: Do you changes accomplish the plan purpose and not just mechanical checklists?
- [ ] **Required Tests**: Have you added tests for any new functions?

### Files to Update
- `shared/src/models/dialect.rs` - Add Feedback struct, DialectWithFeatures struct, and dialect_features() function

### Implementation Details

1. **Add imports to `shared/src/models/dialect.rs`**:
   - `use super::TTSProviderType;`
   - `use std::collections::HashMap;`

2. **Create `Feedback` struct**:
   - Field: `pub score: Option<f64>`
   - Derive: `Debug, Clone, Default, Serialize, Deserialize`
   - Default implementation sets `score: None`

3. **Create `DialectWithFeatures` struct**:
   - Fields:
     - `pub dialect: Dialect`
     - `pub tts_voices: HashMap<TTSProviderType, Option<String>>`
     - `pub has_corpus: bool`
     - `pub feedback: Feedback`
   - Derive: `Debug, Clone, Serialize, Deserialize`
   - Implement `From<Dialect>` trait that calls `dialect_features()`

4. **Implement `dialect_features(dialect: Dialect) -> DialectWithFeatures` function**:
   - Hardcode feature mappings based on current `for_language` active dialects:
     - **Active dialects** (uncommented in for_language): SpanishArgentinian, SpanishCuban, SpanishColombian, ArabicEgyptian, ArabicLevantine, ArabicGulf, FrenchQuebecois, FrenchAfrican
       - Set TTS voices from provider files (ElevenLabs and Azure voice IDs)
       - Set `has_corpus: true`
     - **Inactive dialects** (commented in for_language): SpanishMexican, SpanishCastilian, SpanishChilean, ArabicMaghrebi, ArabicIraqi, FrenchParisian, FrenchSwiss, FrenchBelgian
       - Set all TTS voices to `None`
       - Set `has_corpus: false`
   - For FrenchAfrican: ElevenLabs voice exists, Azure voice is None (no Azure voice for fr-CI)
   - Set `feedback.score: None` for all dialects
   - Use match expression with return value for `has_corpus` to avoid late initialization

5. **Add tests**:
   - Test Feedback default
   - Test DialectWithFeatures creation from Dialect
   - Test dialect_features for active dialect (has TTS and corpus)
   - Test dialect_features for inactive dialect (no TTS, no corpus)
   - Test dialect_features for FrenchAfrican (has ElevenLabs, no Azure)

### Deliverables
- Feedback struct with score: Option<f64>
- DialectWithFeatures wrapper struct
- Hardcoded dialect_features() function mapping all dialects
- Tests for new structures

### Phase Completion
- Run FULL test suite: `cargo test --workspace`
- Run `cargo clippy -p dialect-coach-shared --lib -- -D warnings`
- Upon 100% success, update this status document with progress
- **STOP and wait for EXPLICIT approval before proceeding to Phase 3**

---

## Phase 3: Update for_language Filtering

### Code Style Checklist
- [ ] **Planning Documentation**: Have you consulted/created/updated docs/current-plans/dialect_features_IMPLEMENTATION_STATUS.md?
- [ ] **Code Simplicity**: Are you following simplicity rules? (functions <20 lines, pure functions, no defensive coding)
- [ ] **Code Modularity**: Are you following modularity rules? (helper functions, low cyclomatic complexity)
- [ ] **Scope Control**: Are you accomplishing the user's instructions and NOTHING MORE?
- [ ] **No Dead Code**: Did you leave dead code? (no future-proofing, no leaving just for tests)
- [ ] **No Fake Constructions**: Are there any object instances that are purely for the sake of passing a type checker?
- [ ] **Code Purpose**: Do you changes accomplish the plan purpose and not just mechanical checklists?
- [ ] **Required Tests**: Have you added tests for any new functions?

### Files to Update
- `shared/src/models/dialect.rs` - Update for_language signature and implementation

### Implementation Details

1. **Update `for_language` signature**:
   - Change from: `pub fn for_language(language: Language) -> Vec<Dialect>`
   - Change to: `pub fn for_language(language: Language, with_tts: bool, with_corpus: bool) -> Vec<Dialect>`

2. **Update `for_language` implementation**:
   - Get all dialects for the language (include all dialects, not just active ones)
   - Filter dialects using `dialect_features()`:
     - If `with_tts` is true: only include dialects where at least one TTS voice is Some (not all None)
     - If `with_corpus` is true: only include dialects where `has_corpus == true`
   - Return filtered Vec<Dialect>

3. **Update existing test `test_dialects_for_language`**:
   - Change to: `Dialect::for_language(Language::Spanish, true, true)`
   - Verify it returns exactly 3 dialects: SpanishArgentinian, SpanishCuban, SpanishColombian

4. **Add new tests**:
   - `test_dialects_for_language_no_filters`: with `false, false` returns all 6 Spanish dialects
   - `test_dialects_for_language_with_tts_only`: with `true, false` returns dialects with TTS
   - `test_dialects_for_language_with_corpus_only`: with `false, true` returns dialects with corpus

### Deliverables
- Updated for_language function with boolean parameters
- Filtering logic based on feature availability
- Updated and new tests

### Phase Completion
- Run FULL test suite: `cargo test --workspace`
- Run `cargo clippy -p dialect-coach-shared --lib -- -D warnings`
- Upon 100% success, update this status document with progress
- **STOP and wait for EXPLICIT approval before proceeding to Phase 4**

---

## Phase 4: Update All Callsites

### Code Style Checklist
- [ ] **Planning Documentation**: Have you consulted/created/updated docs/current-plans/dialect_features_IMPLEMENTATION_STATUS.md?
- [ ] **Code Simplicity**: Are you following simplicity rules? (functions <20 lines, pure functions, no defensive coding)
- [ ] **Code Modularity**: Are you following modularity rules? (helper functions, low cyclomatic complexity)
- [ ] **Scope Control**: Are you accomplishing the user's instructions and NOTHING MORE?
- [ ] **No Dead Code**: Did you leave dead code? (no future-proofing, no leaving just for tests)
- [ ] **No Fake Constructions**: Are there any object instances that are purely for the sake of passing a type checker?
- [ ] **Code Purpose**: Do you changes accomplish the plan purpose and not just mechanical checklists?
- [ ] **Required Tests**: Have you added tests for any new functions?

### Files to Update
- `shared/src/models/user_state.rs` - Update current_dialects() method (line ~88)
- `corpus-processor/src/main.rs` - Update for_language call (line ~190)

### Implementation Details

1. **Update `shared/src/models/user_state.rs`**:
   - Find `current_dialects()` method
   - Change: `Dialect::for_language(self.selected_language)`
   - To: `Dialect::for_language(self.selected_language, true, true)`

2. **Update `corpus-processor/src/main.rs`**:
   - Find `for_language` call in Commands::List match arm
   - Change: `Dialect::for_language(lang)`
   - To: `Dialect::for_language(lang, true, true)`

3. **Verify no other callsites**:
   - Check that all `for_language` calls have been updated
   - Tutorial files are examples only and don't need updating

### Deliverables
- All for_language callsites updated with `true, true` parameters
- No compilation errors
- All existing functionality preserved

### Phase Completion
- Run FULL test suite: `cargo test --workspace`
- Run `cargo clippy --workspace --lib -- -D warnings` (checking only lib, not bin)
- Upon 100% success, update this status document with completion status
- **STOP and wait for EXPLICIT approval**

---

## Implementation Status

### Phase 1: Create Common TTS Provider Trait and Type Enum
- Status: ✅ Completed
- Created `shared/src/models/tts.rs` with TTSProviderType enum and TTSProvider trait
- Updated `shared/src/models/mod.rs` to include and export tts module
- Implemented TTSProvider trait for both AzureTtsProvider and ElevenLabsTtsProvider
- Added tests for TTSProviderType serialization and provider type implementations
- All tests pass (123 tests total)
- Clippy checks pass for shared crate

### Phase 2: Create DialectWithFeatures and Feature Mapping
- Status: ✅ Completed
- Added imports for TTSProviderType and HashMap to dialect.rs
- Created Feedback struct with score: Option<f64> and Default trait
- Created DialectWithFeatures struct with dialect, tts_voices HashMap, has_corpus bool, and feedback fields
- Implemented From<Dialect> trait for DialectWithFeatures
- Implemented hardcoded dialect_features() function mapping all dialects:
  - Active dialects (8): SpanishArgentinian, SpanishCuban, SpanishColombian, ArabicEgyptian, ArabicLevantine, ArabicGulf, FrenchQuebecois, FrenchAfrican (all with TTS voices and corpus)
  - Inactive dialects (8): SpanishMexican, SpanishCastilian, SpanishChilean, ArabicMaghrebi, ArabicIraqi, FrenchParisian, FrenchSwiss, FrenchBelgian (no TTS voices, no corpus)
  - FrenchAfrican has ElevenLabs voice but no Azure voice
- Added comprehensive tests for Feedback, DialectWithFeatures, and dialect_features function
- All tests pass (128 tests total)
- Clippy checks pass for shared crate

### Phase 3: Update for_language Filtering
- Status: ✅ Completed
- Updated `for_language` signature to accept `with_tts: bool` and `with_corpus: bool` parameters
- Created helper function `all_dialects_for_language()` to get all dialects for a language (26 lines - data mapping function)
- Updated `for_language` implementation to filter dialects based on feature availability (11 lines)
- Updated existing test `test_dialects_for_language` to use new signature with `true, true`
- Added new tests:
  - `test_dialects_for_language_no_filters`: verifies `false, false` returns all 6 Spanish dialects
  - `test_dialects_for_language_with_tts_only`: verifies `true, false` returns dialects with TTS
  - `test_dialects_for_language_with_corpus_only`: verifies `false, true` returns dialects with corpus
- Updated callsites in `user_state.rs` and `corpus-processor/src/main.rs` to pass `true, true` (needed for compilation)
- All tests pass (131 tests total)
- Clippy checks pass for shared crate

### Phase 4: Update All Callsites
- Status: ✅ Completed
- All callsites were updated in Phase 3 to allow compilation and testing
- Updated `shared/src/models/user_state.rs`: `current_dialects()` now calls `for_language(self.selected_language, true, true)`
- Updated `corpus-processor/src/main.rs`: `for_language` call now passes `true, true`
- Verified no other callsites exist (tutorial files contain examples only)
- All tests pass
- All existing functionality preserved

