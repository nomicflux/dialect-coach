# Implementation Plan - Organic Active Learning Plans

We will implement an "organic" flow where items from the Learning Plan are promoted to the active practice list **only when they are used in conversation** (detected by the AI). We rely on the deterministic UUID generation to link the "shadow" items in the Plan with the "live" items returned by the Agent.

## Phase 1: Shared Models & Helper Deduplication
**COMPLETED**

## Phase 2: Organic Promotion Logic
**COMPLETED**

## Phase 3: Completion Logic
**COMPLETED**

---

## Phase 4: Backend Organic Promotion Fix (Deep Trace)

### Context
- The initial Organic Promotion Logic (Phase 2) failed because the `AnalysisAgent` only received "active" items. Plan items are "hidden" and were not being passed to the agent, so they were never scored, and thus never promoted.
- **Root Cause**: `run_agents_with_analysis` (normal user message path) and `call_agent_for_conversation_action` (system action path) both needed to explicitly extract and bundle plan items for the analysis step.

### Unit of Work
- Modify `backend/src/websocket/agents.rs` to extract items from the Active Plan's current step and inject them into `generate_analysis`.
- **Deliverable**: Analysis Agent scores plan items, enabling the frontend promotion logic to work.

### Action Items
1.  **Modify `agents.rs`**:
    -   Add `combine_plan_and_user_items` helper.
    -   Update `call_agent_for_conversation_action` to use combined items.
    -   Update `run_agents_with_analysis` to use combined items (CRITICAL: this was the missed path in initial fix).
2.  **Verification**:
    -   Unit test `test_combine_plan_and_user_items`.
    -   Verify via `cargo test`.
    -   Verify via `cargo clippy`.

### Verification
- **Automated**:
    -   `cargo test --all` (Passed 100%)
    -   `cargo clippy --all` (Passed 100%)
