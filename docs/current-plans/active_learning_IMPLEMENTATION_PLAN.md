# Implementation Plan - Organic Active Learning Plans

We will implement an "organic" flow where items from the Learning Plan are promoted to the active practice list **only when they are used in conversation** (detected by the AI). We rely on the deterministic UUID generation to link the "shadow" items in the Plan with the "live" items returned by the Agent.

## Phase 1: Shared Models & Helper Deduplication

### Code Style Guidelines (from AGENTS.md)
- **Run `cargo fmt` and `cargo clippy --all-targets --all-features`** before committing; CI expects Rust 2024 defaults and zero warnings.
- **Files and modules use snake_case**, public types PascalCase, and constants SCREAMING_SNAKE_CASE.
- **Functions should be <20 lines, and modules <200 lines**. Prefer small helper functions and submodules.
- Prefer explicit `anyhow::Result` returns, instrument async paths with `tracing`, and keep Yew hooks small, composable functions.

### Context
- **Files**:
    - `shared/src/models/agent.rs` (Verify UUID generation)
    - `frontend/src/app/app_state/user/helpers.rs` (Add deduplication logic)

### Unit of Work
- Verify deterministic UUID generation in `shared`.
- Update `add_learning_items_to_vec` in `frontend` to prevent duplicate items (same ID) from being added.
- **Deliverable**: A helper function that safely adds items without duplication, ensuring our "promotion" logic won't corrupt state.

### Action Items
1.  **Verify UUIDs**: Check `shared/src/models/agent.rs` to confirm `Mistake`, `Explained`, `Translated`, `Exploratory` use deterministic v5 UUIDs based on content. (No code change needed if correct, just verification).
2.  **Update Helper**: Open `frontend/src/app/app_state/user/helpers.rs`.
3.  **Modify `add_learning_items_to_vec`**:
    -   Refactor into smaller helpers if needed to stay under 20 lines.
    -   Iterate through incoming items.
    -   Only push if `!items.iter().any(|i| i.id == new_item.id)`.
4.  **Add Test**: Add a unit test in `helpers.rs` verifying that adding a duplicate item does not increase the vec length.

### Verification
- **Manual**: None for this phase (library code).
- **Automated**:
    -   `cargo test --all`
    -   `cargo clippy --all`

---

## Phase 2: Organic Promotion Logic

### Code Style Guidelines (from AGENTS.md)
- **Run `cargo fmt` and `cargo clippy --all-targets --all-features`** before committing; CI expects Rust 2024 defaults and zero warnings.
- **Files and modules use snake_case**, public types PascalCase, and constants SCREAMING_SNAKE_CASE.
- **Functions should be <20 lines, and modules <200 lines**. Prefer small helper functions and submodules.
- Prefer explicit `anyhow::Result` returns, instrument async paths with `tracing`, and keep Yew hooks small, composable functions.

### Context
- **Files**:
    - `frontend/src/app/app_state/user/reducer.rs`

### Unit of Work
- Implement the "promotion" logic in the reducer.
- When `UpdateScores` happens, check if any of the scored items match a "hidden" item in the active plan's current step.
- If match found, copy it to `learning_items`.
- **Deliverable**: The agent's feedback now "activates" plan items.

### Action Items
1.  **Open `reducer.rs`**: Locate `reduce_learning` -> `UpdateScores`.
2.  **Implement Promotion**:
    -   Create a new helper function `promote_plan_items(state: &mut UserState, analysis: &AgentAnalysis)` to strict adherence to <20 lines rule.
    -   Get `active_plan` from state.
    -   Get `current_step` from plan.
    -   Iterate `current_step.items`.
    -   If an item's ID matches a key in `analysis.scores` AND matches a `current_step` item AND is NOT in `state.learning_items`:
        -   Clone the item from `current_step`.
        -   Add to `state.learning_items`.
3.  **Apply Scores**: Ensure scores are applied to these new items as well.
4.  **Add Test**: Add unit test in `reducer.rs` or `tests.rs`:
    -   Setup Plan with 1 item.
    -   Trigger `UpdateScores` with matching ID.
    -   Assert `state.learning_items` has size 1.

### Verification
- **Manual**:
    -   Start app, start plan.
    -   Use a word from the plan in chat.
    -   See it appear in "Active Items" (Vocab HUD).
- **Automated**:
    -   `cargo test --all`
    -   `cargo clippy --all`

---

## Phase 3: Completion Logic

### Code Style Guidelines (from AGENTS.md)
- **Run `cargo fmt` and `cargo clippy --all-targets --all-features`** before committing; CI expects Rust 2024 defaults and zero warnings.
- **Files and modules use snake_case**, public types PascalCase, and constants SCREAMING_SNAKE_CASE.
- **Functions should be <20 lines, and modules <200 lines**. Prefer small helper functions and submodules.
- Prefer explicit `anyhow::Result` returns, instrument async paths with `tracing`, and keep Yew hooks small, composable functions.

### Context
- **Files**:
    - `frontend/src/app/app_state/user/reducer.rs`

### Unit of Work
- Implement the check for step completion.
- When scores update, check if all items in the current step meets the 80% threshold.
- **Deliverable**: Plan auto-advances when mastery is reached.

### Action Items
1.  **Open `reducer.rs`**: Add helper `check_step_completion(state: &mut UserState)` (keep <20 lines).
2.  **Implement Logic**:
    -   `let step_items = state.active_plan...items`.
    -   `let all_passed = step_items.all(item => state.learning_items.find(item.id).score >= 80)`.
    -   `if all_passed { state.active_plan.advance_step() }`.
3.  **Call Helper**: Call this at the end of `UpdateScores` in `reduce_learning`.
4.  **Add Test**: Add unit test:
    -   Setup Plan with 1 active item (score 70).
    -   Update score to 80.
    -   Assert `active_plan.current_step_index` increments.

### Verification
- **Manual**:
    -   Continue from Phase 2.
    -   Repeatedly use the word (or hack calling `UpdateScores` in dev/test) until score > 80.
    -   Verify Plan advances.
- **Automated**:
    -   `cargo test --all`
    -   `cargo clippy --all`
