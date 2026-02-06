# Fix State Persistence Architecture

## Summary

Two bugs, two fixes:
1. **Stale snapshot overwrite**: `PendingSaveQueue` stores a state snapshot on save failure. If newer state is saved successfully afterward, a reconnect replays the stale snapshot, overwriting newer data. Fix: **delete PendingSaveQueue entirely**, replace with `MarkDirty` on reconnect.
2. **Wasteful save trigger**: `UsageStatsUpdate` from backend goes through `SessionAction::Domain(...)` which sets `needs_save: true`, triggering a full-state save. But usage stats are already persisted by the backend separately (via `save_usage_stats`), and `persistence.save()` strips them (`clean.usage_stats = UsageStats::default()`). Fix: **route through a non-save path**.

---

## Phase 1: Delete PendingSaveQueue, add MarkDirty

### Before Starting - Mandatory Checklist
- [ ] **Planning Documentation**: This file
- [ ] **Code Simplicity**: Pure deletions + one new enum variant
- [ ] **Code Modularity**: N/A (deletions)
- [ ] **Scope Control**: Only PendingSaveQueue removal and MarkDirty addition
- [ ] **No Dead Code**: Entire module deleted, all references removed
- [ ] **No Fake Constructions**: N/A
- [ ] **Code Purpose**: Eliminates stale snapshot overwrite bug
- [ ] **UI Consistency**: N/A (no UI changes)
- [ ] **Required Tests**: No new functions, only deletions

### Subagent: kiss-code-generator

**CRITICAL: Subagent Specification is BINDING.** When a phase specifies "Subagent: X", this is a COMMAND, not a suggestion. You MUST launch that subagent. You MUST NOT use Edit/Write/NotebookEdit yourself.

### Deliverables
- `PendingSaveQueue` module deleted
- All `QueuePendingSave`, `RetryPendingSaves`, `save_queue` references removed
- New `SessionAction::MarkDirty` that sets `needs_save: true`
- On save failure: just log (no queue)
- On user state WS reconnect: dispatch `MarkDirty`
- Chat WS reconnect no longer triggers any save retry

### File Changes

**DELETE**: `frontend/src/services/save_queue.rs`

**EDIT**: `frontend/src/services/mod.rs`
- Remove `pub mod save_queue;` (line 5)
- Remove `pub use save_queue::PendingSaveQueue;` (line 14)

**EDIT**: `frontend/src/app/app_state/app.rs`
- Remove `use crate::services::save_queue::PendingSaveQueue;` (line 12)
- Remove `QueuePendingSave(Box<UserState>)` from `AppStateAction` enum (line 26)
- Remove `RetryPendingSaves` from `AppStateAction` enum (line 27)
- Remove `pub save_queue: Rc<PendingSaveQueue>` from `AppState` struct (line 63)
- Remove `&& Rc::ptr_eq(&self.save_queue, &other.save_queue)` from `PartialEq` impl (line 88)
- Remove `save_queue: Rc::new(PendingSaveQueue::new())` from `Default` impl (line 118)
- Remove `QueuePendingSave` handler (lines 163-165)
- Remove `RetryPendingSaves` handler (lines 166-171)

**EDIT**: `frontend/src/app/app_state/session.rs`
- Add `MarkDirty` variant to `SessionAction` enum
- Add handler:
  ```rust
  SessionAction::MarkDirty => SessionState {
      user: self.user.clone(),
      needs_save: self.user.is_some(),
  }.into(),
  ```

**EDIT**: `frontend/src/app.rs`
- In save failure path (lines 55-59): remove `QueuePendingSave` dispatch, keep just `error!` log
- Remove the clone of `app_state_for_save` that was only needed for QueuePendingSave (line 45). The remaining closure only uses `session_dispatch` and `ws_service` from `app_state_for_save`, so keep that clone but remove the separate `app_state_for_save` usage in the error branch.

**EDIT**: `frontend/src/app/app_state/callbacks.rs`
- Change `on_user_state_ws_open` signature: remove `app_state: UseReducerHandle<AppState>` and `_user_id: Uuid` params, add `session: UseReducerHandle<SessionState>`
- Change body to dispatch `SessionAction::MarkDirty` instead of `AppStateAction::RetryPendingSaves`
- Update imports accordingly

**EDIT**: `frontend/src/app/websocket_hooks.rs`
- Update `on_user_state_ws_open` call (line 243) to pass `session.clone()` instead of `app_state.clone(), user_id`
- Remove `RetryPendingSaves` dispatch on chat WS connected (lines 200-203): delete the `if matches!(new_state, ConnectionState::Connected)` block entirely

### Phase Completion Gate (MANDATORY)

1. Run `cargo test` - 100% pass rate required
2. Run `cargo clippy` - fix ALL warnings
3. Update this status document
4. `git commit` with specific files
5. **STOP AND WAIT** for user approval

---

## Phase 2: Route UsageStats through non-save path

### Before Starting - Mandatory Checklist
- [ ] **Planning Documentation**: This file
- [ ] **Code Simplicity**: One new session action, one removal
- [ ] **Code Modularity**: Clean routing change
- [ ] **Scope Control**: Only UsageStats routing
- [ ] **No Dead Code**: Remove `UserDomainAction::UsageStats` variant
- [ ] **No Fake Constructions**: N/A
- [ ] **Code Purpose**: Eliminates wasteful full-state saves on every message
- [ ] **UI Consistency**: N/A
- [ ] **Required Tests**: No new functions

### Subagent: kiss-code-generator

**CRITICAL: Subagent Specification is BINDING.** When a phase specifies "Subagent: X", this is a COMMAND, not a suggestion. You MUST launch that subagent. You MUST NOT use Edit/Write/NotebookEdit yourself.

### Deliverables
- `UsageStats` updates no longer trigger `needs_save: true`
- `UserDomainAction::UsageStats` variant removed
- New `SessionAction::UpdateUsageStats` preserves current `needs_save` value

### File Changes

**EDIT**: `frontend/src/app/app_state/session.rs`
- Add `UpdateUsageStats(dialect_coach_shared::UsageStats)` variant to `SessionAction`
- Add handler that updates user but preserves `needs_save`:
  ```rust
  SessionAction::UpdateUsageStats(stats) => {
      if let Some(user) = &self.user {
          let mut next = user.clone();
          next.usage_stats = stats;
          SessionState {
              user: Some(next),
              needs_save: self.needs_save,
          }.into()
      } else {
          self
      }
  },
  ```

**EDIT**: `frontend/src/app/app_state/callbacks.rs`
- Change `on_user_state_usage_stats_update` to dispatch `SessionAction::UpdateUsageStats(usage_stats)` instead of `SessionAction::Domain(UserDomainAction::UsageStats(usage_stats))`
- Remove `UserDomainAction` import if no longer needed here

**EDIT**: `frontend/src/app/app_state/user/actions.rs`
- Remove `UsageStats(UsageStats)` from `UserDomainAction` enum (line 90)
- Remove `UserDomainAction::UsageStats(a) => UserStateAction::UpdateUsageStats(a)` from `From` impl (line 101)
- Remove `UsageStats` from imports if no longer needed (line 6)

Note: `UserStateAction::UpdateUsageStats` stays in the enum and reducer - it's used by tests (`tests.rs:20, 657`).

### Phase Completion Gate (MANDATORY)

1. Run `cargo test` - 100% pass rate required
2. Run `cargo clippy` - fix ALL warnings
3. Update this status document
4. `git commit` with specific files
5. **STOP AND WAIT** for user approval

---

## Verification (after all phases)

- No references to `PendingSaveQueue`, `QueuePendingSave`, `RetryPendingSaves`, `save_queue` remain
- No references to `UserDomainAction::UsageStats` remain
- `SessionAction::MarkDirty` exists and is dispatched on user state WS reconnect
- `SessionAction::UpdateUsageStats` exists and preserves `needs_save`

---

## Status

- [x] Phase 1: Complete — PendingSaveQueue deleted, MarkDirty added, all tests pass, clippy clean
- [x] Phase 2: Complete — UsageStats routed through non-save path, all tests pass, clippy clean
