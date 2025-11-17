# Reasoning Budget Implementation Status

**Created:** 2025-11-15
**Status:** In Progress
**Owner:** zen-code-architect

## Problem Analysis

### Root Cause
OpenAI GPT-5 mini's Responses API shares a single max_output_tokens budget between reasoning tokens and text response tokens. When reasoning consumes the entire budget, no tokens remain for the actual response.

**Observed Behavior:**
- Request with max_tokens=512
- GPT-5 mini uses 512 reasoning tokens
- Zero tokens left for text response
- Result: "Incomplete" status with "max_output_tokens" reason
- Error: "No text in response"

### Research Findings

**OpenAI Responses API Limitations:**
- Single `max_output_tokens` parameter covers BOTH reasoning and text output
- `reasoning_tokens` counted as part of `output_tokens`
- NO separate parameter for reasoning vs output token budgets
- Only control: `reasoning.effort` (minimal/low/medium/high) affects consumption rate

**Verified from Documentation:**
- Microsoft Azure OpenAI docs confirm no separate reasoning token cap
- `max_output_tokens` is the total budget (reasoning + text)
- `reasoning_tokens` appears in `output_tokens_details` for reporting only
- Setting `reasoning.effort="minimal"` reduces but doesn't eliminate reasoning

### Solution Design

**Approach:**
Add configurable per-channel reasoning budgets that increase max_tokens for OpenAI requests only.

**Design Decision:**
Rather than complex budget-splitting logic, we simply increase the max_tokens parameter for OpenAI by the reasoning_budget amount. This is direct, simple, and aligns with our ruthless simplicity principle.

**Formula:**
```
OpenAI: actual_max_tokens = requested_max_tokens + reasoning_budget
Anthropic: actual_max_tokens = requested_max_tokens (unchanged)
```

**Benefits:**
- ✅ Simple: one addition operation
- ✅ OpenAI-specific: doesn't affect Anthropic
- ✅ Configurable: per-channel control
- ✅ Backwards-compatible: defaults to 200 tokens

## Environment Variables

### New Variables

**Global Default:**
```bash
OPENAI_REASONING_BUDGET=200  # Default reasoning budget for all OpenAI channels
```

**Per-Channel Overrides:**
```bash
RESPONSE_REASONING_BUDGET=200   # Override for response channel
LEARNING_REASONING_BUDGET=300   # Override for learning channel
ANALYSIS_REASONING_BUDGET=250   # Override for analysis channel
```

### Fallback Chain
For each channel (e.g., RESPONSE):
1. Try `RESPONSE_REASONING_BUDGET` (channel-specific)
2. Fallback to `OPENAI_REASONING_BUDGET` (global default)
3. Fallback to `200` (hardcoded default)

### Example Configurations

**Example 1: Use defaults**
```bash
# No reasoning budget vars set
# All channels get 200 token reasoning budget
```

**Example 2: Global override**
```bash
OPENAI_REASONING_BUDGET=300
# All channels get 300 token reasoning budget
```

**Example 3: Per-channel tuning**
```bash
OPENAI_REASONING_BUDGET=200
RESPONSE_REASONING_BUDGET=150    # Response needs less reasoning
LEARNING_REASONING_BUDGET=400    # Learning needs more reasoning
# Analysis gets default 200
```

---

# REVISED IMPLEMENTATION PLAN

## Critical Lesson Learned

**DEAD CODE IS NEVER ACCEPTABLE.**

Previous attempt (Phases 1-2) violated the "No Dead Code" rule by:
- Adding `reasoning_budget` field in Phase 1 but not using it (dead code warning)
- Creating infrastructure for future phases instead of delivering working functionality

All dead code has been removed. Plan revised to deliver working functionality at each phase.

## Phase 1: Apply Hardcoded Reasoning Budget to OpenAI Requests ✅ COMPLETE

**Completed:** 2025-11-15

**Subagent:** kiss-code-generator

**Code Style Checklist:**
- [x] Functions <20 lines
- [x] Pure functions where possible
- [x] No defensive coding
- [x] Helper functions for complex logic
- [x] Low cyclomatic complexity
- [x] **NO DEAD CODE** - every line must be used in this phase

**Files Modified:**
- `backend/src/agent_service/provider.rs`

**Tasks Completed:**
1. ✅ In `UnifiedCompletionAgent::completion()`, located the OpenAI match arm
2. ✅ Calculate `actual_max_tokens = request.max_tokens + 200` (hardcoded budget)
3. ✅ Set `rig_request.max_tokens = Some(actual_max_tokens)`
4. ✅ Set `reasoning.effort="minimal"` via `additional_params`
5. ✅ Anthropic branch remains unchanged
6. ✅ Add debug logging showing budget application

**Deliverables:**
- ✅ OpenAI requests use `max_tokens = requested + 200`
- ✅ Anthropic requests unchanged
- ✅ `reasoning.effort="minimal"` set for OpenAI
- ✅ Debug logging shows budget calculation
- ✅ All tests pass (78/78)
- ✅ Zero clippy warnings

**Verification:**
This phase delivers COMPLETE functionality: OpenAI gets extra tokens for reasoning.
No dead code - every line is used immediately.

**Code Added (provider.rs:225-235):**
```rust
let actual_max_tokens = request.max_tokens + 200;
rig_request.max_tokens = Some(actual_max_tokens);
tracing::debug!(
    "OpenAI reasoning budget: adding 200 tokens to max_tokens={}",
    request.max_tokens
);
rig_request.additional_params = Some(serde_json::json!({
    "reasoning": {
        "effort": "minimal"
    }
}));
```

---

## Phase 2: Make Reasoning Budget Configurable via Environment Variables ✅ COMPLETE

**Completed:** 2025-11-15

**Code Style Checklist:**
- [x] Functions <20 lines
- [x] Pure functions where possible
- [x] No defensive coding
- [x] Helper functions for complex logic
- [x] Low cyclomatic complexity
- [x] **NO DEAD CODE** - every line must be used in this phase

**Files Modified:**
- `backend/src/agent_service/provider.rs`
- `backend/src/agent_service.rs`

**Tasks Completed:**
1. ✅ Added `reasoning_budget: u32` field to `ProviderAgentConfig` struct
2. ✅ Updated `ProviderAgentConfig::anthropic()` to accept `reasoning_budget` parameter
3. ✅ Updated `ProviderAgentConfig::openai()` to accept `reasoning_budget` parameter
4. ✅ Added `reasoning_budget: u32` field to `UnifiedCompletionAgent` struct
5. ✅ Updated `UnifiedCompletionAgent::new()` to accept and store `reasoning_budget`
6. ✅ In `completion()` method, replaced hardcoded `200` with `self.reasoning_budget as u64`
7. ✅ In `agent_service.rs`, added `load_reasoning_budget(prefix: &str) -> u32` helper (6 lines)
8. ✅ Updated `load_channel_agent()` to call `load_reasoning_budget(prefix)` and pass to both configs
9. ✅ Updated `CompletionAgentFactory::build()` to pass `config.reasoning_budget` to `UnifiedCompletionAgent::new()`
10. ✅ Updated all test constructors in `provider.rs` to include `reasoning_budget` parameter

**Helper Function Implementation:**
```rust
fn load_reasoning_budget(channel_prefix: &str) -> u32 {
    channel_env(channel_prefix, "REASONING_BUDGET")
        .and_then(|v| v.parse::<u32>().ok())
        .or_else(|| env::var("OPENAI_REASONING_BUDGET").ok().and_then(|v| v.parse::<u32>().ok()))
        .unwrap_or(200)
}
```

This implements the fallback chain:
1. Try `{CHANNEL}_REASONING_BUDGET` (channel-specific)
2. Fallback to `OPENAI_REASONING_BUDGET` (global default)
3. Fallback to `200` (hardcoded default)

**Test Results:**
- ✅ All tests pass: 78 passed (backend), 130 passed (all crates)
- ✅ Zero clippy warnings
- ✅ No dead code - every field and function is used immediately

**Verification:**
- ✅ Can trace path: environment variable → `load_reasoning_budget()` → `ProviderAgentConfig` → `UnifiedCompletionAgent` → `completion()` method line 231
- ✅ No compiler warnings about unused fields
- ✅ 100% test pass rate
- ✅ Clippy shows zero warnings

**Phase Completion:**
- [x] Run `cargo test` - 100% success
- [x] Run `cargo clippy` - ZERO warnings
- [x] Update this document with Phase 2 completion
- Ready for: `git add . && git commit -m "Phase 2 (Load reasoning budget from environment) complete"`

---

## Phase 3: Add Tests for Reasoning Budget Functionality ✅ COMPLETE

**Completed:** 2025-11-15

**Code Style Checklist:**
- [x] Functions <20 lines
- [x] Pure functions where possible
- [x] No defensive coding
- [x] Helper functions for complex logic
- [x] Low cyclomatic complexity

**Files Modified:**
- `backend/src/agent_service/provider.rs` (added 2 tests)
- `backend/src/agent_service.rs` (added 4 tests)

**Tests Added:**

1. ✅ `test_load_reasoning_budget_channel_specific()` (agent_service.rs:289-298, 10 lines)
   - Sets `RESPONSE_REASONING_BUDGET=300`
   - Calls `load_reasoning_budget("RESPONSE")`
   - Asserts result equals `300`
   - Cleans up environment variable

2. ✅ `test_load_reasoning_budget_global_fallback()` (agent_service.rs:301-311, 11 lines)
   - Sets `OPENAI_REASONING_BUDGET=250`
   - Removes channel-specific variable
   - Calls `load_reasoning_budget("LEARNING")`
   - Asserts result equals `250`
   - Cleans up environment variable

3. ✅ `test_load_reasoning_budget_default_fallback()` (agent_service.rs:314-321, 8 lines)
   - Ensures NO reasoning budget env vars are set
   - Calls `load_reasoning_budget("ANALYSIS")`
   - Asserts result equals `200` (hardcoded default)
   - Verifies hardcoded fallback works

4. ✅ `test_load_reasoning_budget_precedence()` (agent_service.rs:324-335, 12 lines)
   - Sets both `OPENAI_REASONING_BUDGET=100` and `RESPONSE_REASONING_BUDGET=400`
   - Calls `load_reasoning_budget("RESPONSE")`
   - Asserts result equals `400` (channel-specific takes precedence)
   - Cleans up both environment variables

5. ✅ `test_unified_completion_agent_uses_reasoning_budget()` (provider.rs:378-383, 6 lines)
   - Creates `ProviderAgentConfig` with `reasoning_budget: 350`
   - Builds agent using `CompletionAgentFactory::build()`
   - Verifies the agent is created successfully
   - Tests that field is properly stored in struct

6. ✅ `test_provider_agent_config_stores_reasoning_budget()` (provider.rs:386-389, 4 lines)
   - Creates `ProviderAgentConfig` with `reasoning_budget: 275`
   - Asserts `config.reasoning_budget == 275`
   - Tests direct field storage

**Test Results:**
- ✅ All tests pass: 84 passed (78 existing + 6 new)
- ✅ All backends: 262 total tests pass (84 backend + 22 frontend + 130 shared)
- ✅ Zero clippy warnings
- ✅ All test functions are <20 lines
- ✅ Each test cleans up environment variables
- ✅ Tests are independent and don't rely on execution order

**Verification:**
- [x] All 6 tests added and passing
- [x] Backend tests: 84/84 passing
- [x] Clippy: ZERO warnings
- [x] All test functions <20 lines
- [x] Environment variables properly cleaned up
- [x] Tests verify complete fallback chain

**Phase Completion:**
- [x] Run `cargo test` - 100% success (84 backend + 262 total)
- [x] Run `cargo clippy` - ZERO warnings
- [x] Update this document with Phase 3 completion
- Ready for: `git add . && git commit -m "Phase 3 (Add reasoning budget tests) complete"`

---

## Phase 4: Documentation

**Subagent:** kiss-code-generator

**Files to Modify:**
- `docs/current-plans/reasoning_budget_IMPLEMENTATION_STATUS.md` (this file)
- `README.md` or existing configuration documentation

**Tasks:**
1. Document all four environment variables with descriptions and defaults
2. Document behavior: OpenAI gets max_tokens + reasoning_budget
3. Document fallback chain clearly
4. Add example configurations for common use cases
5. Update README with new env var references
6. Mark this document status as "Complete"

**Deliverables:**
- All env vars documented
- Behavior explained clearly
- Example configurations provided
- Status updated to "Complete"

**Phase Completion:**
- Run `cargo test` - require 100% success
- Run `cargo clippy` - require ZERO warnings
- Update this document marking feature complete
- `git add . && git commit -m "Phase 4 (Documentation) complete"`
- STOP and wait for explicit approval

---

## Phase Status

- [x] Phase 1: Apply Hardcoded Reasoning Budget to OpenAI Requests - **COMPLETE** (2025-11-15)
- [x] Phase 2: Make Reasoning Budget Configurable via Environment Variables - **COMPLETE** (2025-11-15)
- [x] Phase 3: Add Tests for Reasoning Budget Functionality - **COMPLETE** (2025-11-15)
- [ ] Phase 4: Documentation

---

## Success Criteria

### Functional Requirements
- [ ] OpenAI requests receive `max_tokens = requested + reasoning_budget`
- [ ] Anthropic requests unchanged
- [ ] Per-channel budget overrides work (RESPONSE_REASONING_BUDGET, etc.)
- [ ] Fallback to OPENAI_REASONING_BUDGET works
- [ ] Default of 200 works when no env vars set
- [ ] All tests pass (100%)
- [ ] Zero clippy warnings at every phase

### Code Quality
- [ ] All functions <20 lines
- [ ] No defensive coding
- [ ] No dead code at any phase
- [ ] No TODO comments
- [ ] Clear, simple logic flow

### Documentation
- [ ] All env vars documented
- [ ] Behavior explained clearly
- [ ] Example configurations provided
- [ ] Fallback chain documented

---

## Notes

- Default reasoning budget: 200 tokens
- Only affects OpenAI providers (Anthropic unchanged)
- Each channel independently configurable
- Simple addition operation (no complex budget splitting)
- **CRITICAL: No dead code allowed at any phase**
