# Add gender to agent

## Phase 1: Infrastructure (COMPLETED)
- Created TTS struct with Provider enum, Voice name, and Gender fields
- Set all voices to FemalePresenting by default
- Gender is determined by TTS voice

## Phase 2: Update Response Agent to Use Gender in Prompts (IN PROGRESS)

### Changes Made:
1. **Modified `speaker_desc()` function** - lines 45-65
   - Added `gender: &Gender` parameter
   - Modified role descriptions to include gender presentation
   - Examples: "You are a female-presenting native X speaker..." or "You are a male-presenting native X speaker..."
   - Kept function <20 lines

2. **Modified `build_system_content()` function** - lines 277-341
   - Calls `dialect_features()` to get `DialectWithFeatures` with TTS voices
   - Extracts first available TTSVoice from `tts_voices` map
   - Gets gender field from the TTSVoice
   - Defaults to `Gender::FemalePresenting` if no voices available
   - Passes gender to `speaker_desc()`
   - Minimal changes to existing logic

3. **Updated all callers of `speaker_desc()`**
   - Only called from `build_system_content()` (line 291)
   - Updated with gender parameter

4. **Added unit tests for `speaker_desc()`**
   - Test with `Gender::MalePresenting` - verify prompt includes "male-presenting"
   - Test with `Gender::FemalePresenting` - verify prompt includes "female-presenting"
   - Verify rest of prompt content is correct

### Implementation Status:
- [x] speaker_desc() updated with gender parameter
- [x] build_system_content() extracts and uses gender
- [x] All callers updated
- [x] Unit tests added
- [x] Tests passing (12/12 tests pass)
- [x] Clippy clean (no warnings or errors)
- [x] Functions under 20 lines (speaker_desc: 13 lines, extract_gender_from_dialect: 7 lines)

### Test Results:
- test_speaker_desc_male_presenting_formal - PASSED
- test_speaker_desc_female_presenting_informal - PASSED
- test_speaker_desc_includes_gender_in_all_formalities - PASSED
- All existing tests remain passing (12/12 total)
