# Branch Explicit Message Tracking - Implementation Plan

## Problem Summary

**Critical Bug:** Deleting message B causes unrelated message A to disappear from UI.

**Root Cause:** Branches derive message membership from tree structure (walking `parent_id` links backward from `leaf_message_id`). When a message is deleted from global `conversation_history`, the walk breaks and all messages disappear.

**Fundamental architectural flaw:** Branches are views into a shared tree, but deletion operates on global storage, creating impossible situation where branch views can be invalidated.

## Solution

Add explicit `message_ids: Vec<Uuid>` to `ConversationBranch` to track which messages belong to each branch. This makes branch membership explicit and independent of tree structure.

## Research Summary

**Current Code Locations:**
- `shared/src/models/branch.rs:9-16` - ConversationBranch struct definition
- `shared/src/models/user_state.rs:166-188` - get_active_branch_messages() uses parent walking
- `frontend/src/app/app_state.rs:574-597` - AddMessage action adds to conversation_history and updates leaf_message_id
- `frontend/src/app/app_state.rs:656-658` - DeleteMessage action only removes from conversation_history
- `frontend/src/app/app_state.rs:662-684` - CreateBranch action creates new branch with parent_message_id and leaf_message_id

**Key insight:** Message addition happens in frontend's `AddMessage` reducer action. Backend only sends messages via websocket; frontend manages the conversation_history and branch state.

---

## Phase 1: Add message_ids field to ConversationBranch

**Subagent:** kiss-code-generator

**Code Style Checklist:**
- [ ] Functions <20 lines
- [ ] No defensive coding
- [ ] Pure functions where possible
- [ ] Helper functions for complex logic
- [ ] Tests for all new functions

**Files to modify:**
- `shared/src/models/branch.rs`

**Deliverables:**
1. Add `pub message_ids: Vec<Uuid>` field to `ConversationBranch` struct
2. Update `ConversationBranch::new()` to accept `message_ids` parameter
3. Update `set_dialect_if_none` test and all other tests to pass empty vec for message_ids
4. Add serde default for backward compatibility: `#[serde(default)]` on message_ids field

**Implementation steps:**
1. Add field to struct with serde default attribute
2. Update constructor signature to include message_ids parameter
3. Update all test constructors to pass `vec![]` for message_ids
4. Update serialization test to verify message_ids is included

**Phase completion:**
- Run: `cargo test`
- Run: `cargo clippy`
- Fix ALL warnings and errors
- Update: docs/current-plans/branch_explicit_tracking_STATUS.md with "Phase 1 complete"
- Commit: `git commit -m "Phase 1 (add message_ids field) complete"`
- STOP and wait for explicit approval before Phase 2

---

## Phase 2: Update branch creation to initialize message_ids

**Subagent:** kiss-code-generator

**Code Style Checklist:**
- [ ] Functions <20 lines
- [ ] No defensive coding
- [ ] Pure functions where possible
- [ ] Helper functions for complex logic
- [ ] Tests for all new functions

**Files to modify:**
- `frontend/src/app/app_state.rs` (CreateBranch action, line 662-684)
- `shared/src/models/user_state.rs` (UserState::new for initial branch)

**Deliverables:**
1. When creating branch from message M, initialize `message_ids` with path to M
2. Initial branch in UserState::new starts with empty `message_ids`
3. Tests verify new branches have correct message_ids

**Implementation steps:**
1. In CreateBranch action:
   - Get path to fork point message using `get_path_to_message(Some(message_id))`
   - Create new branch with `message_ids` set to this path
2. In UserState::new:
   - Create initial branch with `message_ids: vec![]`
3. Update test_create_branch_reducer to verify message_ids is populated correctly

**Phase completion:**
- Run: `cargo test`
- Run: `cargo clippy`
- Fix ALL warnings and errors
- Update: docs/current-plans/branch_explicit_tracking_STATUS.md with "Phase 2 complete"
- Commit: `git commit -m "Phase 2 (initialize message_ids on branch creation) complete"`
- STOP and wait for explicit approval before Phase 3

---

## Phase 3: Update AddMessage to append to branch message_ids

**Subagent:** kiss-code-generator

**Code Style Checklist:**
- [ ] Functions <20 lines
- [ ] No defensive coding
- [ ] Pure functions where possible
- [ ] Helper functions for complex logic
- [ ] Tests for all new functions

**Files to modify:**
- `frontend/src/app/app_state.rs` (AddMessage action, line 574-597)

**Deliverables:**
1. When adding message to active branch, append message ID to branch's message_ids
2. Tests verify message_ids grows correctly

**Implementation steps:**
1. In AddMessage action, after pushing to conversation_history:
   ```rust
   if let Some(branch) = next.branches.iter_mut().find(|b| b.id == next.active_branch_id) {
       branch.message_ids.push(new_msg_id);
       branch.leaf_message_id = Some(new_msg_id);
       branch.set_dialect_if_none(msg_dialect);
   }
   ```
2. Add test that verifies message_ids list grows when messages are added

**Phase completion:**
- Run: `cargo test`
- Run: `cargo clippy`
- Fix ALL warnings and errors
- Update: docs/current-plans/branch_explicit_tracking_STATUS.md with "Phase 3 complete"
- Commit: `git commit -m "Phase 3 (append to message_ids on add) complete"`
- STOP and wait for explicit approval before Phase 4

---

## Phase 4: Update DeleteMessage to remove from branch message_ids

**Subagent:** kiss-code-generator

**Code Style Checklist:**
- [ ] Functions <20 lines
- [ ] No defensive coding
- [ ] Pure functions where possible
- [ ] Helper functions for complex logic
- [ ] Tests for all new functions

**Files to modify:**
- `frontend/src/app/app_state.rs` (DeleteMessage action, line 656-658)

**Deliverables:**
1. When deleting message M, remove M from all branches' message_ids
2. Update each affected branch's leaf_message_id to last message in its list
3. Tests verify deletion only affects branches containing the message

**Implementation steps:**
1. Replace `delete_message` function with version that also updates branches:
   ```rust
   fn delete_message(mut history: Vec<Message>, id: Uuid) -> Vec<Message> {
       history.retain(|msg| msg.id != id);
       history
   }

   fn remove_message_from_branches(mut branches: Vec<ConversationBranch>, msg_id: Uuid) -> Vec<ConversationBranch> {
       for branch in &mut branches {
           if let Some(pos) = branch.message_ids.iter().position(|&id| id == msg_id) {
               branch.message_ids.remove(pos);
               branch.leaf_message_id = branch.message_ids.last().copied();
           }
       }
       branches
   }
   ```
2. Update DeleteMessage action:
   ```rust
   UserStateAction::DeleteMessage(id) => {
       next.conversation_history = delete_message(next.conversation_history, id);
       next.branches = remove_message_from_branches(next.branches, id);
   }
   ```
3. Add test for bug scenario: Branch 1: A→B, Branch 2: A→C; delete B; switch to Branch 2; delete C; verify both branches still show A

**Phase completion:**
- Run: `cargo test`
- Run: `cargo clippy`
- Fix ALL warnings and errors
- Update: docs/current-plans/branch_explicit_tracking_STATUS.md with "Phase 4 complete"
- Commit: `git commit -m "Phase 4 (remove from message_ids on delete) complete"`
- STOP and wait for explicit approval before Phase 5

---

## Phase 5: Update get_active_branch_messages to use message_ids

**Subagent:** kiss-code-generator

**Code Style Checklist:**
- [ ] Functions <20 lines
- [ ] No defensive coding
- [ ] Pure functions where possible
- [ ] Helper functions for complex logic
- [ ] Tests for all new functions

**Files to modify:**
- `shared/src/models/user_state.rs` (get_active_branch_messages, line 166-188)

**Deliverables:**
1. Replace parent-walking logic with direct lookup from message_ids
2. Remove obsolete `get_path_to_message` helper (if not used elsewhere)
3. Tests verify messages render correctly

**Implementation steps:**
1. Replace get_active_branch_messages implementation:
   ```rust
   pub fn get_active_branch_messages(&self) -> Vec<&Message> {
       if self.branches.is_empty() {
           return Vec::new();
       }

       let branch_id = if self.branches.iter().any(|b| b.id == self.active_branch_id) {
           self.active_branch_id
       } else {
           self.select_active_branch(&self.branches)
       };

       let branch = self.branches.iter()
           .find(|b| b.id == branch_id);

       match branch {
           Some(b) => b.message_ids
               .iter()
               .filter_map(|msg_id| {
                   self.conversation_history.iter().find(|m| m.id == *msg_id)
               })
               .collect(),
           None => Vec::new(),
       }
   }
   ```
2. Check if `get_path_to_message` is used elsewhere; if not, remove it
3. Tests should now pass without changes since behavior is preserved

**Phase completion:**
- Run: `cargo test`
- Run: `cargo clippy`
- Fix ALL warnings and errors
- Update: docs/current-plans/branch_explicit_tracking_STATUS.md with "Phase 5 complete"
- Commit: `git commit -m "Phase 5 (use message_ids for retrieval) complete"`
- STOP and wait for explicit approval before Phase 6

---

## Phase 6: Migration for existing saved data

**Subagent:** kiss-code-generator

**Code Style Checklist:**
- [ ] Functions <20 lines
- [ ] No defensive coding
- [ ] Pure functions where possible
- [ ] Helper functions for complex logic
- [ ] Tests for all new functions

**Files to modify:**
- `shared/src/models/user_state.rs` (add migration helper)
- `backend/src/websocket/user_state.rs` (call migration on load)

**Deliverables:**
1. Migration function populates message_ids from tree structure for old data
2. Migration is called when loading user state
3. Migration is idempotent (safe to call multiple times)

**Implementation steps:**
1. Add migration helper in shared/src/models/user_state.rs:
   ```rust
   fn migrate_branch_message_ids(&mut self) {
       for branch in &mut self.branches {
           if branch.message_ids.is_empty() && branch.leaf_message_id.is_some() {
               branch.message_ids = self.get_path_to_message(branch.leaf_message_id)
                   .into_iter()
                   .map(|m| m.id)
                   .collect();
           }
       }
   }
   ```
2. Call migration in backend/src/websocket/user_state.rs `create_load_response`:
   ```rust
   Some(mut state) => {
       state.migrate_branch_message_ids(); // NEW
       if state.rebuild_branches_from_history() {
           tracing::warn!(user_id = %user_id, "Rebuilt branch metadata from conversation history");
       }
       // ... rest of function
   }
   ```
3. Test migration with fixture data that has empty message_ids

**Phase completion:**
- Run: `cargo test`
- Run: `cargo clippy`
- Fix ALL warnings and errors
- Update: docs/current-plans/branch_explicit_tracking_STATUS.md with "Phase 6 complete"
- Commit: `git commit -m "Phase 6 (migration for old data) complete"`
- STOP and wait for explicit approval before Phase 7

---

## Phase 7: End-to-end testing and validation

**Subagent:** kiss-code-generator

**Code Style Checklist:**
- [ ] Functions <20 lines
- [ ] No defensive coding
- [ ] Pure functions where possible
- [ ] Helper functions for complex logic
- [ ] Tests for all new functions

**Files to modify:**
- `frontend/src/app/app_state.rs` (add comprehensive test)

**Deliverables:**
1. Comprehensive test for the original bug scenario
2. All existing tests still pass
3. No clippy warnings

**Implementation steps:**
1. Add test `test_delete_message_preserves_other_branches`:
   ```rust
   #[test]
   fn test_delete_message_preserves_other_branches() {
       let mut state = UserState::new(Uuid::new_v4());

       // Create message A
       let msg_a = create_test_message(Uuid::new_v4(), None);
       state = apply_user_state_action(&state, UserStateAction::AddMessage(msg_a.clone()));

       // Create message B after A
       let msg_b = create_test_message(Uuid::new_v4(), Some(msg_a.id));
       state = apply_user_state_action(&state, UserStateAction::AddMessage(msg_b.clone()));

       // Branch after A
       state = apply_user_state_action(&state, UserStateAction::CreateBranch(msg_a.id));
       let branch2_id = state.active_branch_id;

       // Create message C in branch 2
       let msg_c = create_test_message(Uuid::new_v4(), Some(msg_a.id));
       state = apply_user_state_action(&state, UserStateAction::AddMessage(msg_c.clone()));

       // Delete C
       state = apply_user_state_action(&state, UserStateAction::DeleteMessage(msg_c.id));

       // Switch to branch 1 (A→B)
       let branch1_id = state.branches.iter().find(|b| b.id != branch2_id).unwrap().id;
       state = apply_user_state_action(&state, UserStateAction::SwitchBranch(branch1_id));

       // Delete B
       state = apply_user_state_action(&state, UserStateAction::DeleteMessage(msg_b.id));

       // Verify branch 1 still has A
       let branch1_msgs = state.get_active_branch_messages();
       assert_eq!(branch1_msgs.len(), 1);
       assert_eq!(branch1_msgs[0].id, msg_a.id);

       // Switch to branch 2 and verify it also has A
       state = apply_user_state_action(&state, UserStateAction::SwitchBranch(branch2_id));
       let branch2_msgs = state.get_active_branch_messages();
       assert_eq!(branch2_msgs.len(), 1);
       assert_eq!(branch2_msgs[0].id, msg_a.id);

       // Verify they are distinct branches
       assert_ne!(branch1_id, branch2_id);
   }
   ```
2. Run full test suite: `cargo test`
3. Run clippy: `cargo clippy`

**Phase completion:**
- Run: `cargo test` - MUST show 100% pass rate
- Run: `cargo clippy` - MUST show 0 warnings
- Update: docs/current-plans/branch_explicit_tracking_STATUS.md with "COMPLETE - All phases done"
- Commit: `git commit -m "Phase 7 (end-to-end testing) complete - Branch explicit tracking fully implemented"`
- STOP and wait for user confirmation that fix is working in manual testing

---

## Notes

- The `parent_id` field in Message remains (may be useful for other features, maintains data structure)
- The `parent_message_id` field in Branch remains (tracks fork point)
- The `leaf_message_id` field in Branch remains (redundant with message_ids.last() but useful for quick checks and backward compatibility)
- This change makes branch identity explicit and independent of message tree structure
- Deletion now only affects branches that explicitly contain the message
