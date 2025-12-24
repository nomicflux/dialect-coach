# ErrorFinding Teaching Mode Implementation Plan

## Agreements Made

**Date: 2025-12-23**

User requirements (exact quotes):
- "Call this TeachingMode 'ErrorFinding', display name 'Find Agent Errors'"
- "The agent response must be instructed to _actively create errors_ (1-3) in its messages"
- "Prefer errors that are common to learners of the language (not random errors, but ones that English learners of the language are likely to make)"
- "the agent should speak naturally, and it should _under no circumstances_ point out its own errors"
- "Response agent should respond positively and encouragingly to user corrections"
- "Learning agents must take _both the latest user and latest agent messages_"
- "If the user corrects the errors in the agent message, then a new learning item (Mistake) is created"
- "Upon successful creation, item has an initial score of 10 instead of 0"
- "If the user uses the errors AS an error, OR corrects it incorrectly, the mistake item should still be created"
- "If used in the same erroneous way as the agent, score of 0"
- "If used with a different error than the agent, score of 5"
- "Analysis agent continues as before (it's still just a mistake learning item, should be treated the same way)"

Clarifications:
- "No, the entire point is that the user needs to see them without being told about them (hence why the agent must not explicitly point them out)" - errors are visible in response but not flagged
- "No, if the user doesn't address the error, no item" - no Mistake created if user ignores error
- "Mistakes are mistakes. By the time they get to the analysis agent, there is no difference." - analysis agent unchanged

## Explicitly Rejected

- Agent acknowledging its own errors
- Creating learning items when user doesn't address the error
- Any special handling in analysis agent for ErrorFinding-sourced Mistakes

## Issues Encountered

- Initial plan had 5 phases with dead code between phases (IntentionalError created in Phase 2 but not consumed until Phase 4)
- Restructured to 2 phases where each phase is fully functional

## Implementation Details

### IntentionalError struct
- `error_form: String` - what agent said (incorrect)
- `correct_form: String` - correct version
- `error_category: MistakeCategory` - reuses existing enum

### AgentResponse field
- `intentional_errors: Option<Vec<IntentionalError>>` - only populated for ErrorFinding mode

### LearningItem constructor
- `with_score(item: LearningItemType, dialect: Dialect, score: u8) -> Self` - creates item with specified initial score

### Score mapping
- User corrects correctly → score 10
- User uses same error → score 0
- User makes different error → score 5
- User ignores error → no item created

---

## Phase 1: Add ErrorFinding Enum Variant (Immersive Behavior)

### Code Style Checklist
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No dead code
- [ ] Tests for new code paths

### Subagent: kiss-code-generator

### Files to Update

**shared/src/models/dialect.rs**
- Add `ErrorFinding` variant to `TeachingMode` enum with `#[serde(rename = "error_finding")]`

**shared/src/models/user_state.rs**
- Add `TeachingMode::ErrorFinding => "Find Agent Errors"` to `teaching_mode_display()` match

**backend/src/agent_service/response/config.rs**
- Add `TeachingMode::ErrorFinding` to `temperature_for_mode()` match, returning `0.3` (same as Immersive)
- Add `TeachingMode::ErrorFinding` to `tokens_per_mode()` match, returning `256` (same as Immersive)

**backend/src/agent_service/response/teaching.rs**
- Add `TeachingMode::ErrorFinding` to `mode_description()` match, returning `IMMERSIVE_DESC` (temporary)

**backend/src/agent_service/learning.rs**
- Add `TeachingMode::ErrorFinding` to `skip_mode()` match, returning `true` (temporary - skip learning like Immersive)
- Add `TeachingMode::ErrorFinding` to `learning_mode_context()` match, returning empty string
- Add `TeachingMode::ErrorFinding` to `learning_output_format_spec()` match, returning empty string

**frontend/src/components/utility_sidebar/settings.rs**
- Add `<option value="error_finding" selected={us.teaching_mode == TeachingMode::ErrorFinding}>{"Find Agent Errors"}</option>` to teaching mode dropdown, before Debug option

### Deliverables
- ErrorFinding mode selectable in frontend dropdown
- ErrorFinding mode functions identically to Immersive (natural conversation, no learning items)
- All exhaustive matches handle ErrorFinding

### Phase End Checklist
- [ ] Run `cargo test` - 100% pass required
- [ ] Run `cargo clippy` - fix ALL errors and warnings, remove any dead code
- [ ] Run `trunk build` in frontend/ - must compile
- [ ] Update Implementation Status table below
- [ ] `git commit -m "Phase 1 (Add ErrorFinding mode - Immersive behavior) complete"`
- [ ] STOP and wait for explicit user approval before Phase 2

---

## Phase 2: Complete ErrorFinding Implementation

### Code Style Checklist
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No dead code
- [ ] Tests for new code paths

### Subagent: modular-builder

### Files to Update

**shared/src/models/agent.rs**
- Add `IntentionalError` struct with fields: `error_form: String`, `correct_form: String`, `error_category: MistakeCategory`
- Add `#[serde(default)] pub intentional_errors: Option<Vec<IntentionalError>>` field to `AgentResponse`
- Update `AgentResponse::new()` to initialize `intentional_errors: None`

**shared/src/models/mod.rs**
- Add `IntentionalError` to public exports

**shared/src/lib.rs**
- Add `IntentionalError` to re-exports if needed

**shared/src/models/learning_item.rs**
- Add `pub fn with_score(item: LearningItemType, dialect: Dialect, score: u8) -> Self` constructor

**backend/src/agent_service/response/config.rs**
- Update `temperature_for_mode()` for ErrorFinding to return `0.5`
- Update `tokens_per_mode()` for ErrorFinding to return `512`

**backend/src/agent_service/response/teaching.rs**
- Add `ERROR_FINDING_DESC` constant with instructions for intentional error generation
- Update `mode_description()` for ErrorFinding to return `ERROR_FINDING_DESC`

**backend/src/agent_service/response/system_content.rs**
- Add helper function to return mode-specific output format
- For ErrorFinding, output format must include `intentional_errors` array in JSON schema

**backend/src/agent_service/learning.rs**
- Add `intentional_errors: &'a [IntentionalError]` field to `LearningAgentParams`
- Update `skip_mode()` to return `false` for ErrorFinding
- Update `learning_mode_context()` for ErrorFinding with error analysis instructions
- Update `learning_output_format_spec()` for ErrorFinding with handled_errors format
- Add `HandledError` struct for parsing learning agent output
- Add `ErrorFindingOutput` struct for parsing learning agent output
- Add `generate_error_finding_items()` async method to `LearningAgent`
- Add `score_for_handling()` helper function
- Update `generate_learning_items()` to branch to `generate_error_finding_items()` for ErrorFinding mode

**backend/src/agent_service/response/generation.rs**
- Update `build_learning_params()` signature to accept `intentional_errors` parameter
- Update `attach_learning_items()` to extract `intentional_errors` from parsed response and pass to `build_learning_params()`

### Deliverables
- Response agent generates 1-3 intentional errors for ErrorFinding mode
- Response agent returns errors as structured data in `intentional_errors` field
- Learning agent analyzes user's handling of each intentional error
- Mistake items created with scores: 10 (corrected), 5 (different error), 0 (same error)
- No item created if user ignores error
- Full end-to-end ErrorFinding functionality working

### Phase End Checklist
- [ ] Run `cargo test` - 100% pass required
- [ ] Run `cargo clippy` - fix ALL errors and warnings, remove any dead code
- [ ] Run `trunk build` in frontend/ - must compile
- [ ] Update Implementation Status table below
- [ ] `git commit -m "Phase 2 (Complete ErrorFinding implementation) complete"`
- [ ] STOP and wait for explicit user approval

---

## Implementation Status

| Phase | Description | Status |
|-------|-------------|--------|
| 1 | Add ErrorFinding enum + all match arms + frontend (Immersive behavior) | Not Started |
| 2 | Complete ErrorFinding implementation (response + learning agents) | Not Started |
