# Language Plans - Implementation Plan

**Goal:** Implement "Language Plans" — structured, multi-step learning sequences (e.g., "Spanish Prepositional Verbs") that guide the user through specific topics with review steps, replacing the ad-hoc nature of simple "Learning Goals".

**Context:**
- UI has been redesigned to a "Hybrid Dashboard" with a **Utility Sidebar**.
- Plans will reside in the `Learning` tab of the Utility Sidebar.
- Plan creation will likely require a specialized view or modal (to be defined in Phase 2).

## Code Style & Standards (Global)

All code written in this plan must adhere to these rules from `AGENTS.md`:
*   **Module Size:** Modules must be < 200 lines. Break large components into submodules (e.g., `components/plans/`).
*   **Function Size:** Functions must be < 20 lines. Use helper functions for layout/logic.
*   **Architecture:** Use pure functions and stateless architectures where possible.
*   **Naming:** `snake_case` for modules/functions, `UpperCamelCase` for types.
*   **Testing:** Every pure function must have a unit test.
*   **Formatting:** `cargo fmt` and `cargo clippy --all` must pass after *every* unit of work.

---

## Phase 1: Core Data Structures & Persistence

**Goal:** Define the data models for Language Plans and integrate them into the `shared` crate and `UserState`. Ensure they can be serialized/deserialized and manipulated, but do not expose them to the UI yet.

### Subphase 1.1: Shared Types
*   **Action Items:**
    *   Create `shared/src/models/plan.rs`.
    *   Define `LanguagePlan`, `PlanStep`, `StepType`, `CompletionCriteria`, `PlanStatus`, `StepStatus` structs/enums (as defined in Brainstorm Option A, but simplified for Phase 1).
    *   **Simplifications for Phase 1:**
        *   `StepType`: Only `Learning` and `Review`.
        *   `CompletionCriteria`: Only `Manual` and `MessageCount`.
    *   Export in `shared/src/models/mod.rs`.
    *   Add `language_plans: Vec<LanguagePlan>` and `active_plan_id: Option<Uuid>` to `UserState` (`shared/src/models/user_state.rs`).
    *   Initialize as empty in `UserState::new()`.
*   **Files:**
    *   [NEW] `shared/src/models/plan.rs`
    *   [MODIFY] `shared/src/models/mod.rs`
    *   [MODIFY] `shared/src/models/user_state.rs`

### Subphase 1.2: Logic & Helpers
*   **Action Items:**
    *   Implement helper methods on `LanguagePlan` (e.g., `new()`, `current_step()`, `advance_step()`, `is_complete()`) in `shared/src/models/plan.rs`.
    *   Write unit tests for these helpers in the same file.
*   **Files:**
    *   [MODIFY] `shared/src/models/plan.rs`

### Subphase 1.3: Verification
*   **Action Items:**
    *   Run `cargo test -p shared` to ensure strict correctness of the new models.
    *   Run `cargo clippy --all` to ensure no warnings.

**STOP & WAIT:**
> Run `cargo test --all` and `cargo clippy --all` until 100% clean.
> Update `docs/current-plans/language_plans_STATUS.md`.
> Wait for user approval to proceed to Phase 2.

---

## Phase 2: Reactivity & State Management

**Goal:** Enable the Frontend to create, update, and delete plans via Redux-style actions. This lays the groundwork for the UI without building the visual components yet.

### Subphase 2.1: App State Actions
*   **Action Items:**
    *   Add `UserStateAction` variants in `frontend/src/app/app_state.rs`:
        *   `AddLanguagePlan(LanguagePlan)`
        *   `DeleteLanguagePlan(Uuid)`
        *   `SetActivePlan(Option<Uuid>)`
        *   `AdvancePlanStep(Uuid)` (advances the specified plan if it's active/ready)
*   **Files:**
    *   [MODIFY] `frontend/src/app/app_state.rs`

### Subphase 2.2: Reducer Implementation
*   **Action Items:**
    *   Implement the reducer logic for these actions in `app_state.rs`.
    *   Ensure they mutate the `UserState` correctly (using the helper methods from Phase 1 where possible, or implementing pure state transitions).
*   **Files:**
    *   [MODIFY] `frontend/src/app/app_state.rs`

### Subphase 2.3: Verification
*   **Action Items:**
    *   Write unit tests for the reducer in `app_state.rs` (or a separate test module if it's too large) to verify that dispatching these actions correctly updates the state.
    *   *Note: Since these are effectively "backend" logic running in the frontend, they must be tested.*
*   **Files:**
    *   [MODIFY] `frontend/src/app/app_state.rs`

**STOP & WAIT:**
> Run `cargo test --all` and `cargo clippy --all` until 100% clean.
> Update `docs/current-plans/language_plans_STATUS.md`.
> Wait for user approval to proceed to Phase 3.

---

## Phase 3: UI - Plan List & Creation

**Goal:** Allow users to view existing plans and create new ones within the Utility Sidebar.

### Subphase 3.1: Plan Components Structure
*   **Action Items:**
    *   Create directory `frontend/src/components/utility_sidebar/learning/plans/`.
    *   Create `mod.rs` in that directory.
    *   Create `plan_list_item.rs`: A small component to render a single plan card (Title, Progress Bar, Status).
    *   Create `plan_list.rs`: Renders the list of plans.
*   **Files:**
    *   [NEW] `frontend/src/components/utility_sidebar/learning/plans/mod.rs`
    *   [NEW] `frontend/src/components/utility_sidebar/learning/plans/plan_list_item.rs`
    *   [NEW] `frontend/src/components/utility_sidebar/learning/plans/plan_list.rs`

### Subphase 3.2: Integration into Utility Sidebar
*   **Action Items:**
    *   Modify `frontend/src/components/utility_sidebar/learning.rs` to include the `PlanList` component.
    *   Add a toggle or section header "Language Plans" to separate it from "Learning Items".
    *   *Constraint:* Ensure it fits visually within the sidebar (narrow width).
*   **Files:**
    *   [MODIFY] `frontend/src/components/utility_sidebar/learning.rs`

### Subphase 3.3: Plan Creation UI (Simplified)
*   **Action Items:**
    *   Create `plan_creator.rs` in the plans directory.
    *   Implement a simple form to create a "Quick Plan":
        *   Title input.
        *   (For Phase 3/MVP) Just creates a plan with 1 default step or allows adding steps via a very simple interface.
        *   "Save" button dispatches `AddLanguagePlan`.
    *   Add a "+ Plan" button in `PlanList` to show this creator.
*   **Files:**
    *   [NEW] `frontend/src/components/utility_sidebar/learning/plans/plan_creator.rs`

### Subphase 3.4: Verification
*   **Action Items:**
    *   Run the app (`trunk serve`) and verify you can create a plan and see it in the list.
    *   Verify deleting a plan works.
    *   Verify `cargo clippy` is clean.

**STOP & WAIT:**
> Run `cargo test --all` and `cargo clippy --all` until 100% clean.
> Update `docs/current-plans/language_plans_STATUS.md`.
> Wait for user approval to proceed to Phase 4.

---

## Phase 4: UI - Active Plan Execution

**Goal:** Display the active plan prominently and allow the user to advance steps.

### Subphase 4.1: Active Plan Display Component
*   **Action Items:**
    *   Create `active_plan_display.rs` in `plans/`.
    *   UI:
        *   Shows Title of active plan.
        *   Shows Current Step Title & Instructions.
        *   Shows "Complete Step" button (Manual completion).
    *   Hook up "Complete Step" button to `AdvancePlanStep` action.
*   **Files:**
    *   [NEW] `frontend/src/components/utility_sidebar/learning/plans/active_plan_display.rs`

### Subphase 4.2: Integration
*   **Action Items:**
    *   Update `PlanList` to clearly highlight the active plan or hide it if it's shown elsewhere.
    *   Update `LearningPanel` (`learning.rs`) to show `ActivePlanDisplay` at the very top, pinned.
*   **Files:**
    *   [MODIFY] `frontend/src/components/utility_sidebar/learning/plans/plan_list.rs`
    *   [MODIFY] `frontend/src/components/utility_sidebar/learning.rs`

### Subphase 4.3: Manual Verification
*   **Action Items:**
    *   Verify visual hierarchy: Active plan at top, list below.
    *   Verify completing a step updates the UI (Step 1 -> Step 2).
    *   Verify completing the last step marks plan as Completed.

**STOP & WAIT:**
> Run `cargo test --all` and `cargo clippy --all` until 100% clean.
> Update `docs/current-plans/language_plans_STATUS.md`.
> Wait for user approval to proceed to Phase 5.

---

## Phase 5: Agent Context Integration

**Goal:** Feed the Active Plan context to the AI Agent so it can help the user learn.

### Subphase 5.1: Shared Model Updates
*   **Action Items:**
    *   Update `UserMessageWithContext` in `shared/src/models/message.rs`.
    *   Add field `active_plan_step: Option<ActivePlanStep>` (define `ActivePlanStep` struct which is a simplified view of the step for the agent).
*   **Files:**
    *   [MODIFY] `shared/src/models/message.rs`

### Subphase 5.2: Frontend Context Injection
*   **Action Items:**
    *   Update `frontend/src/app.rs` (in `on_send_message`).
    *   Logic: Check `state.active_plan_id`. If exists, find the plan, get the current step, construct `ActivePlanStep`, and attach to message.
*   **Files:**
    *   [MODIFY] `frontend/src/app.rs`

### Subphase 5.3: Backend Prompt Engineering
*   **Action Items:**
    *   Update `backend/src/agent_service.rs`.
    *   In `generate_response` (or `build_system_prompt`), check for `active_plan_step`.
    *   If present, append the "ACTIVE LEARNING PLAN" section to the system prompt (as drafted in Brainstorm).
*   **Files:**
    *   [MODIFY] `backend/src/agent_service.rs`

### Subphase 5.4: Final Verification
*   **Action Items:**
    *   Run full `cargo test --all`.
    *   Manual Verification: Start a plan, send a message. Check backend logs or behavior to verify the agent "knows" about the plan.

**STOP & WAIT:**
> Run `cargo test --all` and `cargo clippy --all` until 100% clean.
> Update `docs/current-plans/language_plans_STATUS.md`.
> **Task Complete.**
