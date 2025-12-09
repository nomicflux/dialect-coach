# Gamification Implementation Plan

## Goal
Implement a gamification system to reward mistakes, encourage exploration, and track daily progress, adhering to the "fail forward" ethos of the application.

## Principles
1.  **Strict State Separation**: Gamification data lives in `UserGamificationState`, distinct from `UsageStats` (rate limiting only).
2.  **No New Latency**: Gamification logic runs asynchronously or piggybacks on existing cycles (Agent Analysis).
3.  **Frontend-Driven Quests**: "Daily Focus" quests are derived from existing Learning Items state, minimizing backend complexity.

---

## Phase 1: Backend Data Model & Core Logic

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

### Deliverables
- `UserGamificationState` struct in shared models.
- Core logic for calculating XP updates and Streak progression.
- Unit tests verifying scoring rules.

### Files to Update
- [MODIFY] `shared/src/models/user_state.rs`: Add `gamification` field.
- [NEW] `shared/src/models/gamification.rs`: Define `UserGamificationState`, `StreakStats`, `Quest`, `Achievement`.
- [NEW] `backend/src/gamification/mod.rs`: Logic for updating state (e.g., `calculate_xp_gain`, `update_streak`).

### Action Plan
1.  **Create Models**: New file `shared/src/models/gamification.rs` with:
    -   `UserGamificationState`: `xp`, `streak`, `quests`, `achievements`.
    -   `StreakStats`: `current`, `longest`, `last_active_date`.
    -   `Quest`: `id`, `description`, `target_item_id` (optional), `status`.
    -   `Achievement`: `id`, `kind`, `unlocked_at`.
2.  **Integrate Models**: In `shared/src/models/user_state.rs`, add `pub gamification: UserGamificationState`.
3.  **Implement Logic**: New file `backend/src/gamification/mod.rs` with:
    -   `process_analysis_update(current_state: &mut UserGamificationState, analysis: &AgentAnalysis)` -> Returns `GamificationEvent`.
    -   `check_streak_update(current_state: &mut UserGamificationState, now: i64)`.

### Phase End Procedure
- [ ] **Run Full Test Suite**: `cargo test --all`.
- [ ] **Fix All Errors**: If tests fail, fix them immediately.
- [ ] **Run Clippy**: `cargo clippy --all`.
- [ ] **Clean Dead Code**: Remove any code flagged as unused.
- [ ] **Update Status**: Update `docs/status.md` (or equivalent) if applicable.
- [ ] **Commit**: `git commit -m "Phase 1 (Backend Models) complete"`.
- [ ] **WAIT**: Stop and wait for user approval before proceeding.

---

## Phase 2: Backend Integration

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
- Analysis completion triggers gamification updates.
- Updated user state is persisted to database.
- WebSocket notification sends new gamification state to frontend.

### Files to Update
- [MODIFY] `backend/src/websocket/agents.rs`: Call gamification update logic.
- [MODIFY] `backend/src/websocket/user_state.rs`: Ensure serialization includes gamification.

### Action Plan
1.  **Hook Analysis**: In `backend/src/websocket/agents.rs`, import `backend::gamification`.
2.  **Update Handler**: In `handle_parallel_agents_success`, after saving usage stats (`update_and_save_usage`), call `gamification::process_analysis_update`.
3.  **Persistence**: Save the modified `UserState` back to the persistence layer.
4.  **Serialization**: Verify `backend/src/websocket/user_state.rs` correctly serializes the full `UserState` so the frontend receives the XP/Streak updates.

### Phase End Procedure
- [ ] **Run Full Test Suite**: `cargo test --all`.
- [ ] **Fix All Errors**: If tests fail, fix them immediately.
- [ ] **Run Clippy**: `cargo clippy --all`.
- [ ] **Clean Dead Code**: Remove any code flagged as unused.
- [ ] **Update Status**: Update `docs/status.md`.
- [ ] **Commit**: `git commit -m "Phase 2 (Backend Integration) complete"`.
- [ ] **WAIT**: Stop and wait for user approval before proceeding.

---

## Phase 3: Frontend Components (UI Library)

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
- [ ] **Run Full Test Suite**: `cargo test --all`.
- [ ] **Fix All Errors**: If tests fail, fix them immediately.
- [ ] **Run Clippy**: `cargo clippy --all`.
- [ ] **Clean Dead Code**: Remove any code flagged as unused.
- [ ] **Update Status**: Update `docs/status.md`.
- [ ] **Commit**: `git commit -m "Phase 3 (Frontend Components) complete"`.
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
    -   Trigger a backend save.

### Phase End Procedure
- [ ] **Run Full Test Suite**: `cargo test --all`.
- [ ] **Fix All Errors**: If tests fail, fix them immediately.
- [ ] **Run Clippy**: `cargo clippy --all`.
- [ ] **Clean Dead Code**: Remove any code flagged as unused.
- [ ] **Update Status**: Update `docs/status.md`.
- [ ] **Commit**: `git commit -m "Phase 4 (UI Integration) complete"`.
- [ ] **WAIT**: Stop and wait for user approval before proceeding.
