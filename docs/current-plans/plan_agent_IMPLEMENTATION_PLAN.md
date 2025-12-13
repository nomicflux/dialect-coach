# Plan Agent Implementation Blueprint

**Goal**: Implement the "Plan Agent" as a distinct fourth agent channel (`PLANNING`) using YAML output, specific configuration (low temp/high tokens), and robust retry-with-feedback logic.

**Strategy**: **Parallel Construction**. We will build the new YAML-based components (`Prompt`, `Parser`, `Feedback`) as *new* parallel helpers alongside the existing JSON code. This ensures the application remains fully functional at every step. The final phase will atomically switch the `generate_plan` logic and remove the obsolete JSON components.

## Plan Setup Protocol (PSP) Compliance
- **Output**: `docs/current-plans/plan_agent_IMPLEMENTATION_PLAN.md`
- **Phases**: 5 Atomic, Self-Contained Phases.
- **Checklists**: Full PSP checklists included per phase.
- **Verification**: Strict test/clippy/dead-code rules per phase.

---

## Phase 1: Infrastructure & Utilities

**Reasoning Level**: Low
**Goal**: Establish the foundational dependencies and string processing utilities required for YAML.

### Deliverables
1. `serde_yaml` dependency active.
2. `YAML_OUTPUT_INSTRUCTION` constant available.
3. `normalize_yaml_response` helper verified.

### Files to Update
- `backend/Cargo.toml`
- `backend/src/agent_service/util.rs`

### Action Items
1.  **Edit `backend/Cargo.toml`**: Add `serde_yaml` to dependencies.
2.  **Edit `backend/src/agent_service/util.rs`**:
    -   Add `pub const YAML_OUTPUT_INSTRUCTION`.
    -   Add `pub fn normalize_yaml_response(response: &str) -> String`.
    -   *Logic*: Similar to `clean_response`, strip ```yaml fencing.
3.  **Tests**: Add unit tests in `util.rs` for `normalize_yaml_response`.

### Code Style Checklist
- [ ] **Planning Documentation**: Have you consulted/created/updated docs/current-plans/[FEATURE].md?
- [ ] **Code Simplicity**: Are you following simplicity rules? (functions <20 lines, pure functions, no defensive coding)
- [ ] **Code Modularity**: Are you following modularity rules? (helper functions, low cyclomatic complexity)
- [ ] **Scope Control**: Are you accomplishing the user's instructions and NOTHING MORE?
- [ ] **No Dead Code**: Did you leave dead code? (no future-proofing, no leaving just for tests, no creating fields for future phases)
- [ ] **No Fake Constructions**: Are there any object instances that are purely for the sake of passing a type checker? (e.g. fake credentials, a blank user state)? This means the code should be rearchitected so that either the object doesn't need to be passed, or a real instance passed through instead.
- [ ] **Code Purpose**: Do you changes accomplish the plan purpose and not just mechanical checklists?
- [ ] **UI Consistency**: Are you using established UI styles? If you added new CSS, does it match appearance with the rest of the UI?
- [ ] **Required Tests**: Have you added tests for any new functions?

### Verification & Completion
- [ ] Run the **FULL** test suite (`cargo test --all`).
- [ ] Update status document with progress (if applicable).
- [ ] Run `cargo clippy --all` and fix **ALL** errors.
- [ ] **Zero Dead Code**: Remove any dead code. No excuses.
- [ ] **WAIT FOR EXPLICIT USER APPROVAL**.
- [ ] `git commit -m "Phase 1 (Infrastructure) complete"`

---

## Phase 2: YAML Prompt Component (Parallel)

**Reasoning Level**: Medium
**Goal**: Create the new YAML-specific prompt builders *without* removing the old JSON ones (yet).

### Deliverables
1. `build_planning_system_prompt_yaml` function.
2. `build_planning_user_prompt_yaml` function.

### Files to Update
- `backend/src/agent_service/planning/prompt.rs`

### Action Items
1.  **Edit `prompt.rs`**: Add `pub fn build_planning_system_prompt_yaml(dialect: Dialect) -> String`.
    -   Content: Clone existing logic but swap "JSON" for "YAML" and use `YAML_OUTPUT_INSTRUCTION`.
2.  **Edit `prompt.rs`**: Add `pub fn build_planning_user_prompt_yaml(text: &str) -> String`.
    -   Content: Verify if any specific YAML instruction is needed in user prompt (likely just "convert this text").
3.  **Tests**: Add unit tests verifying the new prompts request "YAML".

### Code Style Checklist
- [ ] **Planning Documentation**: Have you consulted/created/updated docs/current-plans/[FEATURE].md?
- [ ] **Code Simplicity**: Are you following simplicity rules? (functions <20 lines, pure functions, no defensive coding)
- [ ] **Code Modularity**: Are you following modularity rules? (helper functions, low cyclomatic complexity)
- [ ] **Scope Control**: Are you accomplishing the user's instructions and NOTHING MORE?
- [ ] **No Dead Code**: Did you leave dead code? (no future-proofing, no leaving just for tests, no creating fields for future phases)
- [ ] **No Fake Constructions**: Are there any object instances that are purely for the sake of passing a type checker? (e.g. fake credentials, a blank user state)? This means the code should be rearchitected so that either the object doesn't need to be passed, or a real instance passed through instead.
- [ ] **Code Purpose**: Do you changes accomplish the plan purpose and not just mechanical checklists?
- [ ] **UI Consistency**: Are you using established UI styles? If you added new CSS, does it match appearance with the rest of the UI?
- [ ] **Required Tests**: Have you added tests for any new functions?

### Verification & Completion
- [ ] Run the **FULL** test suite (`cargo test --all`).
- [ ] Update status document with progress (if applicable).
- [ ] Run `cargo clippy --all` and fix **ALL** errors.
- [ ] **Zero Dead Code**: Remove any dead code. No excuses.
- [ ] **WAIT FOR EXPLICIT USER APPROVAL**.
- [ ] `git commit -m "Phase 2 (YAML Prompts) complete"`

---

## Phase 3: YAML Parsing Component (Parallel)

**Reasoning Level**: Medium
**Goal**: Create the new parsing logic that handles YAML *without* breaking existing JSON parsing.

### Deliverables
1. `try_parse_plan_output_yaml` helper function.

### Files to Update
- `backend/src/agent_service/planning/mod.rs`

### Action Items
1.  **Edit `mod.rs`**: Add `use serde_yaml;`.
2.  **Edit `mod.rs`**: Implement `fn try_parse_plan_output_yaml(response: &str) -> Result<SimpleImportLanguagePlan>`.
    -   Use `util::normalize_yaml_response`.
    -   Use `serde_yaml::from_str`.
3.  **Tests**: Add a unit test specifically for `try_parse_plan_output_yaml` with sample YAML input.

### Code Style Checklist
- [ ] **Planning Documentation**: Have you consulted/created/updated docs/current-plans/[FEATURE].md?
- [ ] **Code Simplicity**: Are you following simplicity rules? (functions <20 lines, pure functions, no defensive coding)
- [ ] **Code Modularity**: Are you following modularity rules? (helper functions, low cyclomatic complexity)
- [ ] **Scope Control**: Are you accomplishing the user's instructions and NOTHING MORE?
- [ ] **No Dead Code**: Did you leave dead code? (no future-proofing, no leaving just for tests, no creating fields for future phases)
- [ ] **No Fake Constructions**: Are there any object instances that are purely for the sake of passing a type checker? (e.g. fake credentials, a blank user state)? This means the code should be rearchitected so that either the object doesn't need to be passed, or a real instance passed through instead.
- [ ] **Code Purpose**: Do you changes accomplish the plan purpose and not just mechanical checklists?
- [ ] **UI Consistency**: Are you using established UI styles? If you added new CSS, does it match appearance with the rest of the UI?
- [ ] **Required Tests**: Have you added tests for any new functions?

### Verification & Completion
- [ ] Run the **FULL** test suite (`cargo test --all`).
- [ ] Update status document with progress (if applicable).
- [ ] Run `cargo clippy --all` and fix **ALL** errors.
- [ ] **Zero Dead Code**: Remove any dead code. No excuses.
- [ ] **WAIT FOR EXPLICIT USER APPROVAL**.
- [ ] `git commit -m "Phase 3 (YAML Parsing) complete"`

---

## Phase 4: YAML Feedback Component (Parallel)

**Reasoning Level**: Medium
**Goal**: Create the retry feedback builder for YAML errors.

### Deliverables
1. `build_retry_planning_preamble_yaml` helper function.

### Files to Update
- `backend/src/agent_service/planning/mod.rs`

### Action Items
1.  **Edit `mod.rs`**: Implement `fn build_retry_planning_preamble_yaml(original: &str, failed: &str) -> String`.
    -   Wrap `retry::build_retry_preamble`.
    -   Add specific "MUST return valid YAML" instruction.
2.  **Tests**: Add a unit test confirming the feedback string contains YAML instructions.

### Code Style Checklist
- [ ] **Planning Documentation**: Have you consulted/created/updated docs/current-plans/[FEATURE].md?
- [ ] **Code Simplicity**: Are you following simplicity rules? (functions <20 lines, pure functions, no defensive coding)
- [ ] **Code Modularity**: Are you following modularity rules? (helper functions, low cyclomatic complexity)
- [ ] **Scope Control**: Are you accomplishing the user's instructions and NOTHING MORE?
- [ ] **No Dead Code**: Did you leave dead code? (no future-proofing, no leaving just for tests, no creating fields for future phases)
- [ ] **No Fake Constructions**: Are there any object instances that are purely for the sake of passing a type checker? (e.g. fake credentials, a blank user state)? This means the code should be rearchitected so that either the object doesn't need to be passed, or a real instance passed through instead.
- [ ] **Code Purpose**: Do you changes accomplish the plan purpose and not just mechanical checklists?
- [ ] **UI Consistency**: Are you using established UI styles? If you added new CSS, does it match appearance with the rest of the UI?
- [ ] **Required Tests**: Have you added tests for any new functions?

### Verification & Completion
- [ ] Run the **FULL** test suite (`cargo test --all`).
- [ ] Update status document with progress (if applicable).
- [ ] Run `cargo clippy --all` and fix **ALL** errors.
- [ ] **Zero Dead Code**: Remove any dead code. No excuses.
- [ ] **WAIT FOR EXPLICIT USER APPROVAL**.
- [ ] `git commit -m "Phase 4 (YAML Feedback) complete"`

---

## Phase 5: Integration & Cutover

**Reasoning Level**: High
**Goal**: Wire the new components into `generate_plan` and remove the old JSON components.

### Deliverables
1. `generate_plan` fully using YAML components & `retry_with_error_feedback_tracked`.
2. Old JSON components (prompts/parsers) removed.
3. Full system tests passed.

### Files to Update
- `backend/src/agent_service/planning/mod.rs`
- `backend/src/agent_service/planning/prompt.rs`
- `backend/src/agent_service/util.rs` (cleanup if needed)

### Action Items
1.  **Refactor `mod.rs`**: Rewrite `generate_plan` to:
    -   Call `prompt::build_planning_system_prompt_yaml`.
    -   Call `retry_with_error_feedback_tracked` using the Phase 4 preamble builder & Phase 3 parser.
    -   Set `max_tokens: 4096`, `temperature: 0.1` (ensure this matches requirements).
2.  **Cleanup `mod.rs`**: Remove `try_parse_response` (JSON version) and any JSON-specific local helpers.
3.  **Cleanup `prompt.rs`**: Remove `build_planning_system_prompt` (JSON version) and rename `_yaml` versions to be the primary ones (or keep names if preferred, but cleanup old).
4.  **Tests**: Update `MockAgent` tests in `mod.rs` to return YAML. Verify `generate_plan` integration test passes.

### Code Style Checklist
- [ ] **Planning Documentation**: Have you consulted/created/updated docs/current-plans/[FEATURE].md?
- [ ] **Code Simplicity**: Are you following simplicity rules? (functions <20 lines, pure functions, no defensive coding)
- [ ] **Code Modularity**: Are you following modularity rules? (helper functions, low cyclomatic complexity)
- [ ] **Scope Control**: Are you accomplishing the user's instructions and NOTHING MORE?
- [ ] **No Dead Code**: Did you leave dead code? (no future-proofing, no leaving just for tests, no creating fields for future phases)
- [ ] **No Fake Constructions**: Are there any object instances that are purely for the sake of passing a type checker? (e.g. fake credentials, a blank user state)? This means the code should be rearchitected so that either the object doesn't need to be passed, or a real instance passed through instead.
- [ ] **Code Purpose**: Do you changes accomplish the plan purpose and not just mechanical checklists?
- [ ] **UI Consistency**: Are you using established UI styles? If you added new CSS, does it match appearance with the rest of the UI?
- [ ] **Required Tests**: Have you added tests for any new functions?

### Verification & Completion
- [ ] Run the **FULL** test suite (`cargo test --all`).
- [ ] Update status document with progress (if applicable).
- [ ] Run `cargo clippy --all` and fix **ALL** errors.
- [ ] **Zero Dead Code**: Remove any dead code. No excuses.
- [ ] **WAIT FOR EXPLICIT USER APPROVAL**.
- [ ] `git commit -m "Phase 5 (Integration & Cutover) complete"`
