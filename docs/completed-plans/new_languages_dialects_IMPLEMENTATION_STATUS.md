# New Languages and Dialects Implementation Plan

## Overview
Add two new languages (English, Japanese) and 13 new dialects to the system:
- Spanish: Andalusian (1 new)
- French: Ch'ti (1 new)
- English: General American, RP, Australian, Irish, Scottish, South African (6 new)
- Japanese: Tokyo, Kansai, Tohoku, Kyushu, Hokkaido (5 new)

## Phase 1: Add Languages and Dialects to Enums and Core Methods

**Status: PENDING** ⏳

### Code Style Checklist
- [ ] **Planning Documentation**: Have you consulted/created/updated docs/current-plans/new_languages_dialects_IMPLEMENTATION_STATUS.md?
- [ ] **Code Simplicity**: Are you following simplicity rules? (functions <20 lines, pure functions, no defensive coding)
- [ ] **Code Modularity**: Are you following modularity rules? (helper functions, low cyclomatic complexity)
- [ ] **Scope Control**: Are you accomplishing the user's instructions and NOTHING MORE?
- [ ] **No Dead Code**: Did you leave dead code? (no future-proofing, no leaving just for tests)
- [ ] **No Fake Constructions**: Are there any object instances that are purely for the sake of passing a type checker?
- [ ] **Code Purpose**: Do you changes accomplish the plan purpose and not just mechanical checklists?
- [ ] **Required Tests**: Have you added tests for any new functions?

### Files to Update
- `shared/src/models/language.rs` - Add English and Japanese variants, update all methods and tests
- `shared/src/models/dialect.rs` - Add all 13 new dialect variants, update all methods

### Implementation Details

1. **Update `shared/src/models/language.rs`**:
   - Add `English` and `Japanese` variants to `Language` enum (after French, line 10)
   - Update `all()` method (line 17) to include `Language::English, Language::Japanese`
   - Update `code()` method (lines 22-27) to add:
     - `Language::English => "en"`
     - `Language::Japanese => "ja"`
   - Update `name()` method (lines 30-35) to add:
     - `Language::English => "English"`
     - `Language::Japanese => "Japanese"`
   - Update test `test_all_languages()` (line 58) to expect 5 languages instead of 3
   - Add test cases for new languages in `test_language_code()`:
     - `assert_eq!(Language::English.code(), "en");`
     - `assert_eq!(Language::Japanese.code(), "ja");`

2. **Update `shared/src/models/dialect.rs`**:
   
   **Add new dialect variants to enum** (after line 23 for Spanish, after line 47 for French):
   ```rust
   // Spanish dialects - add after SpanishColombian
   #[serde(rename = "spanish_andalusian")]
   SpanishAndalusian,
   
   // French dialects - add after FrenchAfrican
   #[serde(rename = "french_chti")]
   FrenchChti,
   
   // English dialects - add new section
   #[serde(rename = "english_general_american")]
   EnglishGeneralAmerican,
   #[serde(rename = "english_rp")]
   EnglishRP,
   #[serde(rename = "english_australian")]
   EnglishAustralian,
   #[serde(rename = "english_irish")]
   EnglishIrish,
   #[serde(rename = "english_scottish")]
   EnglishScottish,
   #[serde(rename = "english_south_african")]
   EnglishSouthAfrican,
   
   // Japanese dialects - add new section
   #[serde(rename = "japanese_tokyo")]
   JapaneseTokyo,
   #[serde(rename = "japanese_kansai")]
   JapaneseKansai,
   #[serde(rename = "japanese_tohoku")]
   JapaneseTohoku,
   #[serde(rename = "japanese_kyushu")]
   JapaneseKyushu,
   #[serde(rename = "japanese_hokkaido")]
   JapaneseHokkaido,
   ```

   **Update `language()` method** (lines 53-72):
   - Add `Self::SpanishAndalusian => Language::Spanish` to Spanish match arm
   - Add `Self::FrenchChti => Language::French` to French match arm
   - Add new match arm for English dialects:
     ```rust
     Self::EnglishGeneralAmerican
     | Self::EnglishRP
     | Self::EnglishAustralian
     | Self::EnglishIrish
     | Self::EnglishScottish
     | Self::EnglishSouthAfrican => Language::English,
     ```
   - Add new match arm for Japanese dialects:
     ```rust
     Self::JapaneseTokyo
     | Self::JapaneseKansai
     | Self::JapaneseTohoku
     | Self::JapaneseKyushu
     | Self::JapaneseHokkaido => Language::Japanese,
     ```

   **Update `id()` method** (lines 77-97):
   - Add `Self::SpanishAndalusian => "spanish_andalusian"`
   - Add `Self::FrenchChti => "french_chti"`
   - Add all 6 English dialect IDs:
     - `Self::EnglishGeneralAmerican => "english_general_american"`
     - `Self::EnglishRP => "english_rp"`
     - `Self::EnglishAustralian => "english_australian"`
     - `Self::EnglishIrish => "english_irish"`
     - `Self::EnglishScottish => "english_scottish"`
     - `Self::EnglishSouthAfrican => "english_south_african"`
   - Add all 5 Japanese dialect IDs:
     - `Self::JapaneseTokyo => "japanese_tokyo"`
     - `Self::JapaneseKansai => "japanese_kansai"`
     - `Self::JapaneseTohoku => "japanese_tohoku"`
     - `Self::JapaneseKyushu => "japanese_kyushu"`
     - `Self::JapaneseHokkaido => "japanese_hokkaido"`

   **Update `name()` method** (lines 100-121):
   - Add `Self::SpanishAndalusian => "Andalusian Spanish"`
   - Add `Self::FrenchChti => "Ch'ti French"`
   - Add all 6 English dialect names:
     - `Self::EnglishGeneralAmerican => "General American"`
     - `Self::EnglishRP => "British English (RP)"`
     - `Self::EnglishAustralian => "Australian English"`
     - `Self::EnglishIrish => "Irish English"`
     - `Self::EnglishScottish => "Scottish English"`
     - `Self::EnglishSouthAfrican => "South African English"`
   - Add all 5 Japanese dialect names:
     - `Self::JapaneseTokyo => "Tokyo Japanese"`
     - `Self::JapaneseKansai => "Kansai Japanese"`
     - `Self::JapaneseTohoku => "Tohoku Japanese"`
     - `Self::JapaneseKyushu => "Kyushu Japanese"`
     - `Self::JapaneseHokkaido => "Hokkaido Japanese"`

   **Update helper functions** (lines 124-153):
   - Update `all_spanish_dialects()` to include `Self::SpanishAndalusian`
   - Update `all_french_dialects()` to include `Self::FrenchChti`
   - Add `all_english_dialects()` function:
     ```rust
     fn all_english_dialects() -> Vec<Dialect> {
         vec![
             Self::EnglishGeneralAmerican,
             Self::EnglishRP,
             Self::EnglishAustralian,
             Self::EnglishIrish,
             Self::EnglishScottish,
             Self::EnglishSouthAfrican,
         ]
     }
     ```
   - Add `all_japanese_dialects()` function:
     ```rust
     fn all_japanese_dialects() -> Vec<Dialect> {
         vec![
             Self::JapaneseTokyo,
             Self::JapaneseKansai,
             Self::JapaneseTohoku,
             Self::JapaneseKyushu,
             Self::JapaneseHokkaido,
         ]
     }
     ```

   **Update `all_dialects_for_language()` method** (lines 155-161):
   - Add cases:
     - `Language::English => Self::all_english_dialects()`
     - `Language::Japanese => Self::all_japanese_dialects()`

   **Update `all()` method** (lines 177-196):
   - Add `Self::SpanishAndalusian` to Spanish section
   - Add `Self::FrenchChti` to French section
   - Add all 6 English dialects
   - Add all 5 Japanese dialects

   **Update `from_id()` method** (lines 199-220):
   - Add `"spanish_andalusian" => Some(Self::SpanishAndalusian)`
   - Add `"french_chti" => Some(Self::FrenchChti)`
   - Add all 6 English dialect ID mappings
   - Add all 5 Japanese dialect ID mappings

   **Update `get_tts_voices()` function** (lines 286-312):
   - Add match arms for all new dialects, all returning `build_voice_map(None, None)` initially (no TTS configured yet)

   **Update `has_corpus()` function** (lines 315-327):
   - No changes needed - new dialects will return `false` by default (no corpus data yet)

### Deliverables
- `Language` enum includes English and Japanese variants
- All `Language` methods (`all()`, `code()`, `name()`) handle new languages
- `Dialect` enum includes all 13 new dialect variants
- All `Dialect` methods (`language()`, `id()`, `name()`, `all()`, `from_id()`, helper functions) handle new dialects
- TTS and corpus functions handle new dialects (initially None/false)
- All existing tests updated and passing
- New test cases added for new languages

### Phase Completion
- [ ] Run formatting check: `cargo fmt --check` - Must pass
- [ ] Run lint check: `cargo clippy --workspace` - Must pass
- [ ] Run FULL test suite: `cargo test --workspace` - Must pass (100% success required)
- [ ] Status document updated with progress
- **ALL agents must STOP and wait for EXPLICIT approval before proceeding to Phase 2**

---

## Phase 2: Update Frontend UI Components

**Status: PENDING** ⏳

### Code Style Checklist
- [ ] **Planning Documentation**: Have you consulted/created/updated docs/current-plans/new_languages_dialects_IMPLEMENTATION_STATUS.md?
- [ ] **Code Simplicity**: Are you following simplicity rules? (functions <20 lines, pure functions, no defensive coding)
- [ ] **Code Modularity**: Are you following modularity rules? (helper functions, low cyclomatic complexity)
- [ ] **Scope Control**: Are you accomplishing the user's instructions and NOTHING MORE?
- [ ] **No Dead Code**: Did you leave dead code? (no future-proofing, no leaving just for tests)
- [ ] **No Fake Constructions**: Are there any object instances that are purely for the sake of passing a type checker?
- [ ] **Code Purpose**: Do you changes accomplish the plan purpose and not just mechanical checklists?
- [ ] **Required Tests**: Have you added tests for any new functions?

### Files to Update
- `frontend/src/components/settings_panel.rs` - Add English and Japanese to language dropdown
- `frontend/src/app/user_state/callbacks.rs` - Update language parsing to handle "english" and "japanese"
- `frontend/src/app/app_state.rs` - Update `default_dialect_for_language()` to return defaults for English and Japanese

### Implementation Details

1. **Update `frontend/src/components/settings_panel.rs`** (lines 57-59):
   - Add `<option>` elements for English and Japanese:
     ```rust
     <option value="english" selected={us.selected_language == Language::English}>{"English"}</option>
     <option value="japanese" selected={us.selected_language == Language::Japanese}>{"Japanese"}</option>
     ```

2. **Update `frontend/src/app/user_state/callbacks.rs`** (lines 16-19):
   - Add cases to language parsing match statement:
     ```rust
     "english" => Language::English,
     "japanese" => Language::Japanese,
     ```

3. **Update `frontend/src/app/app_state.rs`** (lines 470-473):
   - Add cases to `default_dialect_for_language()` function:
     ```rust
     Language::English => Dialect::EnglishGeneralAmerican,
     Language::Japanese => Dialect::JapaneseTokyo,
     ```

### Deliverables
- Language dropdown in settings panel includes English and Japanese options
- Language selection callback correctly parses "english" and "japanese" strings
- Default dialect selection works for English and Japanese languages

### Phase Completion
- [ ] Run formatting check: `cargo fmt --check` - Must pass
- [ ] Run lint check: `cargo clippy --workspace` - Must pass
- [ ] Run FULL test suite: `cargo test --workspace` - Must pass (100% success required)
- [ ] Status document updated with progress
- **ALL agents must STOP and wait for EXPLICIT approval before marking plan complete**

---

## Implementation Notes

- **Serde naming**: All new dialects follow existing pattern (`language_dialectname` format, lowercase with underscores)
- **TTS and corpus**: New dialects start with no TTS voices or corpus data configured - these can be added later as needed
- **Match statements**: All match statements must be exhaustive - Rust compiler will enforce this
- **Test coverage**: Existing tests should continue to pass, but may need updates for new language counts
- **No BCP-47**: BCP-47 functionality has been removed, so no updates needed for `bcp47_tag()` or `from_bcp47()` methods

