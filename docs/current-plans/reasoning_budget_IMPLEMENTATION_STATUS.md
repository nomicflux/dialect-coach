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

## Phase 1: Apply Hardcoded Reasoning Budget to OpenAI Requests

**Subagent:** kiss-code-generator

**Code Style Checklist:**
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] Helper functions for complex logic
- [ ] Low cyclomatic complexity
- [ ] **NO DEAD CODE** - every line must be used in this phase

**Files to Modify:**
- `backend/src/agent_service/provider.rs`

**Tasks:**
1. In `UnifiedCompletionAgent::completion()`, locate the OpenAI match arm
2. Calculate `actual_max_tokens = request.max_tokens + 200` (hardcoded budget)
3. Set `rig_request.max_tokens = Some(actual_max_tokens)`
4. Set `reasoning.effort="minimal"` via `additional_params`
5. Anthropic branch remains unchanged
6. Add debug logging showing budget application

**Deliverables:**
- OpenAI requests use `max_tokens = requested + 200`
- Anthropic requests unchanged
- `reasoning.effort="minimal"` set for OpenAI
- Debug logging shows budget calculation
- All tests pass (100%)
- Zero clippy warnings

**Verification:**
This phase delivers COMPLETE functionality: OpenAI gets extra tokens for reasoning.
No dead code - every line is used immediately.

**Phase Completion:**
- Run `cargo test` - require 100% success
- Run `cargo clippy` - require ZERO warnings
- Update this document with Phase 1 completion
- `git add . && git commit -m "Phase 1 (Apply hardcoded reasoning budget to OpenAI) complete"`
- STOP and wait for explicit approval

---

## Phase 2: Make Reasoning Budget Configurable via Environment Variables

**Subagent:** kiss-code-generator

**Code Style Checklist:**
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] Helper functions for complex logic
- [ ] Low cyclomatic complexity
- [ ] **NO DEAD CODE** - every line must be used in this phase

**Files to Modify:**
- `backend/src/agent_service/provider.rs`
- `backend/src/agent_service.rs`

**Tasks:**
1. Add `reasoning_budget: u32` field to `ProviderAgentConfig`
2. Update both constructors to accept `reasoning_budget` parameter
3. Add `reasoning_budget` field to `UnifiedCompletionAgent`
4. Update `new()` to accept and store `reasoning_budget`
5. In `completion()`, replace hardcoded `200` with `self.reasoning_budget`
6. In `agent_service.rs`, add `load_reasoning_budget(prefix: &str) -> u32` helper
7. Update `load_channel_agent()` to call helper and pass to configs
8. Update all test constructors to use reasoning_budget parameter

**Deliverables:**
- Environment variable loading with fallback chain works
- Each channel can have independent budget
- Tests pass with different budget values
- Zero clippy warnings

**Verification:**
This phase makes the working feature configurable.
No dead code - all fields are used immediately.

**Phase Completion:**
- Run `cargo test` - require 100% success
- Run `cargo clippy` - require ZERO warnings
- Update this document with Phase 2 completion
- `git add . && git commit -m "Phase 2 (Make reasoning budget configurable) complete"`
- STOP and wait for explicit approval

---

## Phase 3: Add Tests for Reasoning Budget Functionality

**Subagent:** kiss-code-generator

**Code Style Checklist:**
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] Helper functions for complex logic
- [ ] Low cyclomatic complexity

**Files to Modify:**
- `backend/src/agent_service/provider.rs` (add tests)
- `backend/src/agent_service.rs` (add tests)

**Tasks:**
1. Add test: `test_openai_applies_reasoning_budget()` - verify budget added to max_tokens
2. Add test: `test_anthropic_ignores_reasoning_budget()` - verify Anthropic unchanged
3. Add test: `test_load_reasoning_budget_channel_specific()` - verify channel override
4. Add test: `test_load_reasoning_budget_global_fallback()` - verify global default
5. Add test: `test_load_reasoning_budget_default_fallback()` - verify hardcoded default
6. Add test: `test_reasoning_effort_minimal()` - verify reasoning.effort set

**Deliverables:**
- 6 new tests covering reasoning budget functionality
- All tests pass (100%)
- Zero clippy warnings

**Phase Completion:**
- Run `cargo test` - require 100% success
- Run `cargo clippy` - require ZERO warnings
- Update this document with Phase 3 completion
- `git add . && git commit -m "Phase 3 (Add reasoning budget tests) complete"`
- STOP and wait for explicit approval

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

- [ ] Phase 1: Apply Hardcoded Reasoning Budget to OpenAI Requests
- [ ] Phase 2: Make Reasoning Budget Configurable via Environment Variables
- [ ] Phase 3: Add Tests for Reasoning Budget Functionality
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
