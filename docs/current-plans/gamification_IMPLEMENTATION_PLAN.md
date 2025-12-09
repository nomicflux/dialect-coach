# Gamification Implementation Plan

## Goal
Implement a gamification system to reward mistakes, encourage exploration, and track daily progress, adhering to the "fail forward" ethos of the application.

## Principles
1.  **Strict State Separation**: Gamification data lives in `UserGamificationState`, distinct from `UsageStats` (rate limiting only).
2.  **No New Latency**: Gamification logic runs asynchronously or piggybacks on existing cycles (Agent Analysis).
3.  **Frontend-Driven Quests**: "Daily Focus" quests are derived from existing Learning Items state, minimizing backend complexity.

---

## Phase 1: Core Logic (Pure Derivation)

**Selected Subagent**: `kiss-code-generator` (Simple tasks within a single file. Prioritize simplicity and simple changes. Low reasoning level.)

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

### Concept
Gamification stats (XP, Streak) are **derived on-the-fly** from `UserState` (`conversation_history`, `learning_items`). There is **no persistent state** for gamification.

### Deliverables
- `derive_gamification_stats(state: &UserState) -> GamificationStats` function in `shared`.
    - **Note**: Must be composed of small helper functions (<20 lines) to meet code style.
- Removal of any persisted gamification fields.

### Files to Update
- [MODIFY] `shared/src/models/user_state.rs`: Remove `gamification` field.
- [NEW] `shared/src/models/gamification.rs`: Implement derivation logic.

### Phase End Procedure
- [x] **Run Full Test Suite**: `cargo test --all`.
- [x] **Fix All Errors**: If tests fail, fix them immediately.
- [x] **Run Clippy**: `cargo clippy --all`.
- [x] **Clean Dead Code**: Remove any code flagged as unused.
- [x] **Update Status**: Update `docs/status.md`.
- [ ] **Commit**: `git commit -m "Phase 1 (Pure Derivation) complete"`.
- [ ] **WAIT**: Stop and wait for user approval before proceeding.

---

---

## Phase 2: Frontend Integration

**Selected Subagent**: `modular-builder` (Tasks requiring creating new code or spanning files.)

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

### Deliverables
- Frontend `UserState` equivalent derived from raw message history and learning items.
- *Note: Since gamification is purely derived, the backend work is complete. This phase focuses on calculating the stats on the client side or exposing the derivation via API if needed. However, given the "Pure Derivation" demand, client-side derivation parallels the backend derivation.*
- **Correction**: We will implement the exact same derivation logic in Rust/WASM on the frontend, or expose the derived stats via the existing `UserState` API response.
- **Decision**: Since `derive_gamification_stats` is in `shared`, we can reuse it directly in the frontend (WASM) if `shared` is compiled there, or the backend can populate a *non-persistent* field in the response.
- **Current Architecture**: `derive_gamification_stats` is in `shared`. Backend uses it for tests. Frontend can use it to display stats.

### Action Plan
1.  **Expose Stats**: Ensure `derive_gamification_stats` is accessible to the frontend code.
2.  **Calculate in UI**: In `frontend/src/app/mod.rs` (or where State is held), use the function to calculate stats from the local `UserState` replica.
    
### Phase End Procedure
- [x] **Run Full Test Suite**: `cargo test --all`.
- [x] **Fix All Errors**: If tests fail, fix them immediately.
- [x] **Run Clippy**: `cargo clippy --all`.
- [x] **Clean Dead Code**: Remove any code flagged as unused.
- [x] **Update Status**: Update `docs/status.md`.
- [x] **Commit**: `git commit -m "Phase 2 (Frontend Integration) complete"`.
- [x] **WAIT**: Stop and wait for user approval before proceeding.

---

## Phase 3: Frontend Components

**Selected Subagent**: `kiss-code-generator`

### Code Style Checklist
- [x] **Planning Documentation**: Have you consulted/created/updated docs/current-plans/[FEATURE].md?
- [x] **Code Simplicity**: Are you following simplicity rules? (functions <20 lines, pure functions, no defensive coding)
- [x] **Code Modularity**: Are you following modularity rules? (helper functions, low cyclomatic complexity)
- [x] **Scope Control**: Are you accomplishing the user's instructions and NOTHING MORE?
- [x] **No Dead Code**: Did you leave dead code? (no future-proofing, no leaving just for tests, no creating fields for future phases)
- [x] **No Fake Constructions**: Are there any object instances that are purely for the sake of passing a type checker? (e.g. fake credentials, a blank user state)? This means the code should be rearchitected so that either the object doesn't need to be passed, or a real instance passed through instead.
- [x] **Code Purpose**: Do you changes accomplish the plan purpose and not just mechanical checklists?
- [x] **UI Consistency**: Are you using established UI styles? If you added new CSS, does it match appearance with the rest of the UI?
- [x] **Required Tests**: Have you added tests for any new functions?

### Deliverables
- `FluencyBar` component.
- `StreakFlame` component.
- `QuestCard` component.

### Files to Update
- [NEW] `frontend/src/components/gamification/mod.rs`: Module declaration.
- [NEW] `frontend/src/components/gamification/fluency_bar.rs`: Progress bar visual.
- [NEW] `frontend/src/components/gamification/streak_display.rs`: Flame/Calendar icon.
- [NEW] `frontend/src/components/gamification/quest_list.rs`: Display 3 daily quests.

### Action Plan
1.  **Scaffold Module**: Create `frontend/src/components/gamification/mod.rs`.
2.  **FluencyBar**: Implement component taking `xp` prop.
3.  **StreakDisplay**: Implement component taking `current_streak` prop.
4.  **QuestList**: Implement component taking `Vec<Quest>`.

### Phase End Procedure
- [x] **Run Full Test Suite**: `cargo test --all`.
- [x] **Fix All Errors**: If tests fail, fix them immediately.
- [x] **Run Clippy**: `cargo clippy --all`.
- [x] **Clean Dead Code**: Remove any code flagged as unused.
- [x] **Update Status**: Update `docs/status.md`.
- [x] **Commit**: `git commit -m "Phase 3 (Frontend Components) complete"`.
- [ ] **WAIT**: Stop and wait for user approval before proceeding.

---

## Phase 4: UI Integration & Daily Focus Logic

**Selected Subagent**: `modular-builder` (Tasks requiring creating new code or spanning files. Build modularly, in small pieces that individually are sound, and combine for the complete deliverable. Each piece should be independently testable and small. High reasoning level.)

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

### Deliverables
- Header shows Fluency and Streak.
- Sidebar shows Daily Quests.
- "Daily Focus" quests are generated on client-side session start.

### Files to Update
- [MODIFY] `frontend/src/components/header.rs`: Add `FluencyBar` and `StreakDisplay`.
- [MODIFY] `frontend/src/components/utility_sidebar/mod.rs`: Add `QuestList`.
- [MODIFY] `frontend/src/app/app_state.rs`: Logic to generate daily quests if missing.

### Action Plan
1.  **Header Integration**: In `frontend/src/components/header.rs`, import and render `FluencyBar` and `StreakDisplay` in the user section.
2.  **Sidebar Integration**: In `frontend/src/components/utility_sidebar/mod.rs`, add a section for `QuestList` using `user_state.gamification.quests`.
3.  **Quest Generation**: In `frontend/src/app/app_state.rs` (or helper), implement `check_daily_quests_refresh`.
    -   If `gamification.quests` is empty or stale (from yesterday), select 3 random `active` learning items.
    -   Update `gamification.quests`.
**Selected Subagent**: `modular-builder` (Tasks requiring creating new code or spanning files.)

### Code Style Checklist
- [ ] **Planning Documentation**: Have you consulted/created/updated docs/current-plans/[FEATURE].md?
- [ ] **Code Simplicity**: Are you following simplicity rules? (functions <20 lines, pure functions, no defensive coding)
- [ ] **Code Modularity**: Are you following modularity rules? (helper functions, low cyclomatic complexity)
- [ ] **Scope Control**: Are you accomplishing the user's instructions and NOTHING MORE?
- [ ] **No Dead Code**: Did you leave dead code? (no future-proofing, no leaving just for tests, no creating fields for future phases)
- [ ] **No Fake Constructions**: Are there any object instances that are purely for the sake of passing a type checker? (e.g. fake credentials, a blank user state)? This means the code should be rearchitected so that either the object doesn't need to be passed, or a real instance passed through instead.
- [ ] **Code Purpose**: Do you changes accomplish the plan purpose and not just mechanical checklists?
**Selected Subagent**: `modular-builder` (Tasks requiring creating new code or spanning files.)

### Code Style Checklist
- [x] **Planning Documentation**: Have you consulted/created/updated docs/current-plans/[FEATURE].md?
- [x] **Code Simplicity**: Are you following simplicity rules? (functions <20 lines, pure functions, no defensive coding)
- [x] **Code Modularity**: Are you following modularity rules? (helper functions, low cyclomatic complexity)
- [x] **Scope Control**: Are you accomplishing the user's instructions and NOTHING MORE?
- [x] **No Dead Code**: Did you leave dead code? (no future-proofing, no leaving just for tests, no creating fields for future phases)
- [x] **No Fake Constructions**: Are there any object instances that are purely for the sake of passing a type checker? (e.g. fake credentials, a blank user state)? This means the code should be rearchitected so that either the object doesn't need to be passed, or a real instance passed through instead.
- [x] **Code Purpose**: Do you changes accomplish the plan purpose and not just mechanical checklists?
- [x] **UI Consistency**: Are you using established UI styles? If you added new CSS, does it match appearance with the rest of the UI?
- [x] **Required Tests**: Have you added tests for any new functions?

### Concept
"Daily Focus" quests are **dynamically derived** from the user's `LearningItems` (e.g., "Fix your mistake in 'Hello'"). They are not stored in the DB.

### Phase 4: UI Integration & Daily Focus Logic
- [x] **Logic to derive `Vec<Quest>` from UserState** (in `shared/src/models/gamification.rs`)
    - [x] Create `Quest` struct (id, description, completed status).
    - [x] Implement function to suggest 3 quests based on `LearningItems` (prioritize mistakes, then explanations).
- [x] **Update `shared/src/models/gamification.rs`**
    - [x] Add `quests: Vec<Quest>` to `GamificationStats`.
    - [x] Update `derive_gamification_stats` to call `derive_quests`.
- [x] **UI Integration of Header and Sidebar**
    - [x] `frontend/src/components/header.rs`: Add `FluencyBar` and `StreakDisplay`.
    - [x] `frontend/src/components/utility_sidebar/mod.rs`: Add `QuestList` to a tab.
- [x] **Daily Focus quests are derived** from existing Learning Items state, minimizing backend complexity.
- [x] **Review code style** (functions < 20 lines).
- [x] **Run tests**: `cargo test --all`.
- [x] **Run clippy**: `cargo clippy --all`.
- [x] **Update Status**: `docs/status.md`.
- [x] **Commit**: "Phase 4 (UI Integration) complete".
- [ ] **WAIT**: Stop and wait for user approval before proceeding.
