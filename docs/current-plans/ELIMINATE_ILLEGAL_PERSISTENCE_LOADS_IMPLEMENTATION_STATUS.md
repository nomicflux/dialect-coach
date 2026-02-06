# Eliminate Illegal Persistence Loads

## Problem
Translated learning items (مخنوق, روقان) not scored by analysis agent because the backend loads stale `UserState` from persistence instead of using the items the frontend sent in `msg_with_context`.

## Root Cause
Two illegal `UserState` loads during message handling create dual sources of learning items:
1. `check_rate_limits` (user_state.rs:416) — loads full `UserState`, returns it to callers who extract learning items
2. `load_user_state_for_action` (user_state.rs:206) — loads full `UserState` for AI action processing

In `run_agents_with_analysis` (agents.rs:157-159), items for the **analysis agent** are extracted from the persisted `UserState`, while the **response agent** uses `msg_with_context` (frontend-sent). The persisted state was stale (missing recently-added items), causing analysis to silently skip them.

## Architectural Principle
- User state is loaded on **sign-in only**
- During message handling, all context comes from the **message** (frontend is source of truth)
- The **only** legitimate persistence interaction during message handling is **stats management**
- No other loading. None. We communicate by way of messages.

## Evidence from Logs
```
Line 5: Running agents in parallel: ... 2 translated ...    (msg_with_context — frontend)
Line 6: Starting analysis for ... 1 translated ...          (user_state — persistence, STALE)
```
Drop from 2→1 translated items. The persisted state had only the old "sometimes/ساعات" item; the frontend-added مخنوق and روقان were missing.

## Illegal Load Audit

| ID | File | Line | Function | Returns | Used For | Verdict |
|----|------|------|----------|---------|----------|---------|
| A | user_state.rs | 206 | `load_user_state_for_action` | `UserState` | AI action processing (learning items, messages, settings) | **ILLEGAL** |
| B | user_state.rs | 416 | `check_rate_limits` | `UserState` | Rate limiting (only needs `UsageStats`) + callers extract items | **ILLEGAL** |
| C | user_state.rs | 463 | `load_user_state_response` | `UserState` | Initial load on connect | Legal (sign-in) |
| D | user.rs | 34 | `load_user_state` | `UserState` | Sign-in flow | Legal (sign-in) |
| E | auth_service.rs | 131 | `is_authorized` | existence check | Auth verification | Legal |
| F | tts_handler.rs | 97 | `check_tts_rate_limits` | `UsageStats` | TTS rate limiting | Legal (stats) |

## Additional Dead Code Found
- `AIActionRequest::TranslateMessage` — exists in shared types and backend handler but **never sent by frontend**. Dead code to remove.

---

## Phase 1: Fix Analysis to Use Message Context — COMPLETE

**Status**: COMPLETE (commit ecbb98f4)
**Subagent**: kiss-code-generator
**Result**: Deleted 3 lines extracting items from `user_state` in `run_agents_with_analysis`. Analysis call now uses `msg_with_context.past_*` directly. 271 tests pass, 0 clippy warnings.

### Code Style Checklist
- [ ] Planning Documentation: This document
- [ ] Code Simplicity: Deleting code, no new functions
- [ ] Code Modularity: N/A
- [ ] Scope Control: Fix only the analysis item source
- [ ] No Dead Code: `extract_learning_items` still used in `call_agent_for_conversation_action`
- [ ] UI Consistency: N/A (backend only)
- [ ] Required Tests: Existing tests cover the function; verify no regressions

### Files Modified
- `backend/src/websocket/agents.rs`

### Changes
In `run_agents_with_analysis` (line 139):
- Delete lines 157-159:
  ```rust
  let active_plan = user_state.active_plan();
  let all_items = combine_plan_and_user_items(&user_state.learning_items, active_plan.as_ref());
  let (m, e, t, x) = extract_learning_items(&all_items, dialect);
  ```
- Replace the analysis call (lines 163-173) to use `msg_with_context` fields directly:
  ```rust
  mistakes: &msg_with_context.past_mistakes,
  explained: &msg_with_context.past_explained,
  translated: &msg_with_context.past_translated,
  exploratory: &msg_with_context.past_exploratory,
  ```

### Deliverables
Analysis agent receives the same learning items as response agent. Both use `msg_with_context`.

### Phase Completion Gate — MANDATORY
At the end of this phase, the orchestrating agent MUST execute these steps IN ORDER:
1. `cargo test` — 100% pass rate required. If ANY test fails: FIX IT before proceeding.
2. `cargo clippy` — fix ALL warnings and errors. Dead code = immediate removal.
3. Update this status document with what was done, what tests pass, any issues.
4. `git add <specific files> && git commit -m "Phase 1 (fix analysis item source) complete"`
5. **STOP AND WAIT** for explicit user approval before proceeding to Phase 2.

### Subagent Specification is BINDING
This phase specifies **kiss-code-generator**. You MUST launch that subagent. You MUST NOT use Edit/Write/NotebookEdit yourself. Violation = wasting money (5-10x cost) + burning context.

---

## Phase 2: check_rate_limits Returns UsageStats Only — COMPLETE

**Status**: COMPLETE (commit 8adae69c)
**Subagent**: kiss-code-generator
**Result**: `check_rate_limits` now uses `load_usage_stats` and returns `UsageStats`. `update_and_save_usage` takes `(Uuid, UsageStats)`. `save_usage_and_notify` no longer takes `UserState`. All callers in agents.rs updated. 271 tests pass, 0 clippy warnings.

### Code Style Checklist
- [ ] Planning Documentation: This document
- [ ] Code Simplicity: Narrowing return types, functions stay small
- [ ] Code Modularity: Separate rate-limiting from item extraction
- [ ] Scope Control: Only change rate limiting + usage tracking data flow
- [ ] No Dead Code: `load_user_state_for_action` still used for AI actions
- [ ] UI Consistency: N/A (backend only)
- [ ] Required Tests: Update check_rate_limits tests if any; verify all pass

### Files Modified
- `backend/src/websocket/user_state.rs`
- `backend/src/websocket/agents.rs`

### Changes in user_state.rs

1. `check_rate_limits` (line 408):
   - Use `state.user_persistence.load_usage_stats(user_id)` instead of `.load(user_id)`
   - Return `UsageStats` instead of `UserState`
   - Signature: `pub async fn check_rate_limits(...) -> Result<UsageStats, anyhow::Error>`

2. `update_and_save_usage` (line 230):
   - Change parameter from `mut user_state: UserState` to `user_id: Uuid, mut usage_stats: UsageStats`
   - Update all internal references from `user_state.user_id` to `user_id`, from `user_state.usage_stats` to `usage_stats`

3. `save_usage_and_notify` (line 362):
   - Remove `_user_state: UserState` parameter (already unused — underscore prefix)
   - Only needs `state`, `usage_stats`, `user_id`

### Changes in agents.rs

1. `run_agents_parallel` (line 193):
   - `check_rate_limits` now returns `UsageStats`
   - Extract `user_id` from `msg_with_context.user_id`
   - Pass `(user_id, usage_stats)` to `run_agents_with_analysis` and `run_response_only`

2. `run_agents_with_analysis` (line 139):
   - Change parameter from `user_state: UserState` to `user_id: Uuid, usage_stats: UsageStats`
   - Pass `(user_id, usage_stats)` to `handle_parallel_agents_success`

3. `handle_parallel_agents_success` (line 77):
   - Change parameter from `user_state: UserState` to `user_id: Uuid, usage_stats: UsageStats`
   - Pass `(user_id, usage_stats)` to `update_and_save_usage`

4. `run_response_only` (line 461):
   - Change parameter from `user_state: UserState` to `user_id: Uuid, usage_stats: UsageStats`
   - Pass `(user_id, usage_stats)` to `update_and_save_usage`

### Deliverables
Message handling path no longer loads full UserState. Only UsageStats loaded for rate limiting and tracking.

### Phase Completion Gate — MANDATORY
At the end of this phase, the orchestrating agent MUST execute these steps IN ORDER:
1. `cargo test` — 100% pass rate required. If ANY test fails: FIX IT before proceeding.
2. `cargo clippy` — fix ALL warnings and errors. Dead code = immediate removal.
3. Update this status document with what was done, what tests pass, any issues.
4. `git add <specific files> && git commit -m "Phase 2 (check_rate_limits returns UsageStats only) complete"`
5. **STOP AND WAIT** for explicit user approval before proceeding to Phase 3.

### Subagent Specification is BINDING
This phase specifies **kiss-code-generator**. You MUST launch that subagent. You MUST NOT use Edit/Write/NotebookEdit yourself. Violation = wasting money (5-10x cost) + burning context.

---

## Phase 3: AI Conversation Actions Carry Full Context

**Subagent**: modular-builder

### Code Style Checklist
- [ ] Planning Documentation: This document
- [ ] Code Simplicity: Frontend sends all context; backend receives and uses it
- [ ] Code Modularity: Shared ConversationContext struct extended
- [ ] Scope Control: StartConversation + ContinueBranch only
- [ ] No Dead Code: Remove all functions that become unused
- [ ] UI Consistency: N/A (no visual changes)
- [ ] Required Tests: Test ConversationContext construction; verify all crate tests pass

### Files Modified
- `shared/src/models/message.rs`
- `shared/src/models/user_state.rs`
- `frontend/src/app/callbacks.rs`
- `backend/src/websocket/agents.rs`

### Changes in shared/src/models/message.rs

1. Add `context_messages: Vec<Message>` field to `ConversationContext` (after line 58)

2. Change `StartConversation` from unit variant to:
   ```rust
   StartConversation {
       context: Box<ConversationContext>,
   },
   ```

### Changes in shared/src/models/user_state.rs

1. Update `build_action_context` (line 381) signature to accept context messages:
   ```rust
   pub fn build_action_context(&self, context_messages: Vec<Message>) -> ConversationContext
   ```
   Add `context_messages` field to the returned struct.

### Changes in frontend/src/app/callbacks.rs

1. `on_auto_start_conversation` (around line 213):
   - Build context: `let context = state.build_action_context(vec![]);`
   - Send: `AIActionRequest::StartConversation { context: Box::new(context) }`

2. `on_continue_branch` (around line 249):
   - Build context messages up to parent:
     ```rust
     let all_msgs = state.get_active_branch_messages();
     let parent_idx = all_msgs.iter().position(|m| m.id == parent_message_id);
     let context_messages = match parent_idx {
         Some(idx) => all_msgs[..=idx].iter().map(|m| (*m).clone()).collect(),
         None => vec![],
     };
     let context = state.build_action_context(context_messages);
     ```

### Changes in backend/src/websocket/agents.rs

1. `process_ai_action_request` (line 398): Update match on line 415 to extract from StartConversation's context:
   ```rust
   AIActionRequest::StartConversation { context } => (
       context.dialect, context.formality, context.teaching_mode, context.user_gender,
   ),
   AIActionRequest::ContinueBranch { context, .. } => (
       context.dialect, context.formality, context.teaching_mode, context.user_gender,
   ),
   ```

2. `build_action_context` (line 335, the BACKEND function — different from the shared one):
   - Remove `user_state: &UserState` parameter
   - `ContinueBranch` arm: get parent message text from `context.context_messages.last()` instead of `user_state.msg_by_id()`
   - Signature becomes: `fn build_action_context(action: &AIActionRequest, dialect: Dialect, formality: Formality, user_gender: UserGender) -> String`

3. `call_agent_for_conversation_action` (line 560):
   - Remove `user_state: &UserState` parameter
   - Extract settings from ConversationContext for both variants (both have it now)
   - Get `context_messages` from the request's ConversationContext instead of `get_branch_messages_up_to`
   - Extract learning items + goals + plan + language_option + language_level from ConversationContext for BOTH variants (no `_ =>` branch needed)

4. **Delete dead code**:
   - `extract_learning_items` (line 502) — no longer called (phase 1 removed agents.rs:159 usage; this phase removes agents.rs:623 usage)
   - `combine_plan_and_user_items` (line 540) — same
   - `get_item_id` (line 530) — only used in `combine_plan_and_user_items`
   - `get_branch_messages_up_to` (line 484) — replaced by frontend-sent context_messages
   - `clone_messages_up_to_index` (line 477) — only used in `get_branch_messages_up_to`

### Deliverables
StartConversation and ContinueBranch are fully self-contained. `call_agent_for_conversation_action` has zero dependency on loaded UserState. Five dead functions removed.

### Phase 3 Completion Status — DONE

**All changes implemented:**
1. `shared/src/models/message.rs`: Added `context_messages: Vec<Message>` to `ConversationContext`; changed `StartConversation` to carry `Box<ConversationContext>`; updated test.
2. `shared/src/models/user_state.rs`: `build_action_context` now takes `context_messages: Vec<Message>` parameter.
3. `frontend/src/app/callbacks.rs`: `on_auto_start` sends `StartConversation { context }` with empty messages; `on_continue_branch` builds context_messages up to parent and passes to `build_action_context`.
4. `backend/src/websocket/agents.rs`:
   - Replaced `build_action_context` with `build_conversation_instruction` (no user_state) and `build_explain_translate_instruction` (still uses user_state for Phase 4).
   - Split `process_ai_action_request` into `process_conversation_action` (uses context from request, no user_state) and `process_explain_translate_action` (still loads user_state for now).
   - `call_agent_for_conversation_action` no longer takes `user_state`; extracts all data from `ConversationContext`.
   - **Deleted 7 items**: `clone_messages_up_to_index`, `get_branch_messages_up_to`, `extract_learning_items`, `get_item_id`, `combine_plan_and_user_items`, `test_extract_learning_items`, `test_combine_plan_and_user_items`.
   - Removed `HashSet`, `LearningItem`, `LearningItemType`, `LanguagePlan` imports.

**Verification:**
- `cargo test`: 220 backend + all shared + all frontend tests pass (100%)
- `cargo clippy`: zero warnings

### Phase Completion Gate — MANDATORY
At the end of this phase, the orchestrating agent MUST execute these steps IN ORDER:
1. `cargo test` — 100% pass rate required. If ANY test fails: FIX IT before proceeding.
2. `cargo clippy` — fix ALL warnings and errors. Dead code = immediate removal.
3. Update this status document with what was done, what tests pass, any issues.
4. `git add <specific files> && git commit -m "Phase 3 (conversation actions carry full context) complete"`
5. **STOP AND WAIT** for explicit user approval before proceeding to Phase 4.

### Subagent Specification is BINDING
This phase specifies **modular-builder**. You MUST launch that subagent. You MUST NOT use Edit/Write/NotebookEdit yourself. Violation = wasting money (5-10x cost) + burning context.

---

## Phase 4: ExplainMessage Carries Content; Remove All Remaining Illegal Loads

**Subagent**: modular-builder

### Code Style Checklist
- [ ] Planning Documentation: This document
- [ ] Code Simplicity: Action requests carry their own data
- [ ] Code Modularity: N/A
- [ ] Scope Control: ExplainMessage restructured; TranslateMessage (dead) removed; load_user_state_for_action removed
- [ ] No Dead Code: Final cleanup — all illegal loads eliminated
- [ ] UI Consistency: N/A
- [ ] Required Tests: All crate tests pass

### Files Modified
- `shared/src/models/message.rs`
- `frontend/src/app/callbacks.rs`
- `backend/src/websocket/agents.rs`
- `backend/src/websocket/user_state.rs`

### Changes in shared/src/models/message.rs

1. Change `ExplainMessage` to carry content + settings:
   ```rust
   ExplainMessage {
       message_content: String,
       dialect: Dialect,
       formality: Formality,
   },
   ```

2. Delete `TranslateMessage` variant entirely (never sent by frontend — dead code).

### Changes in frontend/src/app/callbacks.rs

1. `on_explain_message` (around line 279):
   - Look up message content from state: `state.msg_by_id(message_id)`
   - Get dialect + formality from settings
   - Send: `AIActionRequest::ExplainMessage { message_content, dialect, formality }`

### Changes in backend/src/websocket/agents.rs

1. `process_ai_action_request` (line 398):
   - Remove `load_user_state_for_action` call (line 407). No more UserState load.
   - Extract settings from each action variant's own data (all variants now carry what they need)
   - Remove `TranslateMessage` handling from the match

2. `build_action_context` (line 335):
   - Remove `TranslateMessage` arm
   - `ExplainMessage` arm uses `message_content` directly instead of `user_state.msg_by_id()`
   - Function no longer references UserState at all

3. Restructure `process_ai_action_request` to not need `user_state`:
   - Create metadata from action's own settings instead of `create_metadata_from_user_state`

### Changes in backend/src/websocket/user_state.rs

1. Delete `load_user_state_for_action` (line 205-214) — no longer called
2. Delete `create_metadata_from_user_state` (line 216-228) — replaced by metadata construction from action context

### Deliverables
Zero illegal UserState loads remain. `load_user_state_for_action` deleted. `TranslateMessage` dead code deleted. Every AI action is fully self-contained — backend never consults persistence for anything except stats.

### Phase Completion Gate — MANDATORY
At the end of this phase, the orchestrating agent MUST execute these steps IN ORDER:
1. `cargo test` — 100% pass rate required. If ANY test fails: FIX IT before proceeding.
2. `cargo clippy` — fix ALL warnings and errors. Dead code = immediate removal.
3. Update this status document with what was done, what tests pass, any issues.
4. `git add <specific files> && git commit -m "Phase 4 (ExplainMessage carries content, all illegal loads removed) complete"`
5. **STOP AND WAIT** for explicit user approval.

### Subagent Specification is BINDING
This phase specifies **modular-builder**. You MUST launch that subagent. You MUST NOT use Edit/Write/NotebookEdit yourself. Violation = wasting money (5-10x cost) + burning context.

---

## Summary of All Deleted Code

| Phase | Function | File | Reason |
|-------|----------|------|--------|
| 3 | `extract_learning_items` | agents.rs | Replaced by msg_with_context / ConversationContext |
| 3 | `combine_plan_and_user_items` | agents.rs | Same |
| 3 | `get_item_id` | agents.rs | Only used in combine_plan_and_user_items |
| 3 | `get_branch_messages_up_to` | agents.rs | Frontend sends history |
| 3 | `clone_messages_up_to_index` | agents.rs | Only used in get_branch_messages_up_to |
| 3 | `build_action_context` (backend) | agents.rs | Replaced by `build_conversation_instruction` + `build_explain_translate_instruction` |
| 4 | `load_user_state_for_action` | user_state.rs | Illegal persistence load eliminated |
| 4 | `create_metadata_from_user_state` | user_state.rs | Replaced by context-based metadata |
| 4 | `TranslateMessage` variant + handler | message.rs, agents.rs | Dead code (never sent by frontend) |

## Explicitly Rejected
- Loading UserState from persistence during message handling (for any purpose other than stats)
- Having two separate sources of learning items
- Backend consulting persistence for learning items, message history, or settings during message processing
