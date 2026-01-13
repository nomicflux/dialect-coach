# Phase 1: Add active_plan_id to ConversationBranch and Make Dialect Non-Optional

## Status: Phase 3 COMPLETE

## Objectives
1. Make `dialect` field required (non-optional) in `ConversationBranch`
2. Add new `active_plan_id` field as optional
3. Update constructor to match new signature
4. Delete `set_dialect_if_none` method (no longer needed)
5. Update all tests to use required dialect
6. Add new test for active_plan_id serialization

## Implementation Details

### File: `shared/src/models/branch.rs`

**Changes to ConversationBranch struct:**
- Change `pub dialect: Option<Dialect>` → `pub dialect: Dialect`
- Add `#[serde(default)] pub active_plan_id: Option<Uuid>`

**Constructor Changes:**
- Dialect parameter becomes required: `dialect: Dialect`
- Add new parameter: `active_plan_id: Option<Uuid>`
- New signature: `new(parent_message_id: Option<Uuid>, name: Option<String>, leaf_message_id: Option<Uuid>, dialect: Dialect, message_ids: Vec<Uuid>, active_plan_id: Option<Uuid>)`

**Methods to Delete:**
- `set_dialect_if_none()` - no longer applicable

**Tests to Update:**
- `test_new_branch_with_parent` - pass required dialect, None for active_plan_id
- `test_new_branch_with_name` - pass required dialect, None for active_plan_id
- `test_new_branch_root` - pass required dialect (e.g., Dialect::SpanishMexican), None for active_plan_id
- `test_branch_serialization` - pass required dialect, None for active_plan_id
- DELETE: `test_set_dialect_if_none_sets_when_none`
- DELETE: `test_set_dialect_if_none_does_not_override`

**New Test:**
- `test_branch_with_active_plan_id()` - verify serialization/deserialization with active_plan_id

## Code Style Checklist
- [ ] All functions <20 lines
- [ ] No defensive coding
- [ ] No dead code
- [ ] Modify in place (no parallel implementations)
- [ ] All tests pass

## Testing Command
```bash
cargo test -p dialect-coach-shared branch::tests
```

## Git Commit Message
```
Phase 1 (ConversationBranch updates) complete

- Make dialect field required (non-optional)
- Add active_plan_id field with serde(default)
- Update constructor to accept both changes
- Delete set_dialect_if_none method
- Update all tests to match new signature
- Add test for active_plan_id serialization
```
