# Auto-Create Branch on Language Switch - Analysis & Plan

## User Issue

**Date: 2025-11-23**

User reports: "If I start a conversation in Argentinian Spanish, then switch to Levantine Arabic, there is only one branch - the Argentitian Spanish one."

## Current Behavior

1. User starts app → Initial branch with dialect: None
2. User sends message in Argentinian Spanish → Branch gets dialect: SpanishArgentinian
3. User clicks dialect cycle button → Changes selected_dialect to ArabicLevantine
4. User sends message in Arabic → **Message goes to same SpanishArgentinian branch**
5. Result: Only one branch exists, contains mixed languages

## Root Cause Analysis

When user cycles dialect to a different language:
- `selected_dialect` changes
- Active branch stays the same
- Next message uses new `selected_dialect` in metadata
- But **branch dialect doesn't update** (only set from first message)
- Branch keeps original dialect even though new messages are different language

## What Should Happen

When switching to a different **language** (not just dialect):
1. Create a new empty branch (dialect: None)
2. Switch to that new branch
3. First message in new language sets new branch's dialect

## Implementation Plan

### Research Phase: Understand the Flow

Need to trace:
1. How does dialect cycling work in the UI?
2. What action gets dispatched?
3. When does the new dialect take effect?
4. Where should we detect language change?

**Key Files:**
- `frontend/src/app/user_state/callbacks.rs` - `on_dialect_cycle()`
- `frontend/src/app/app_state.rs` - UserStateAction handlers
- Need to understand: ChangeDialect vs CycleDialect

### Solution Approach

**Option A: Detect in ChangeDialect action**
- When ChangeDialect is dispatched
- Check if old_language != new_language
- If different, create new branch

**Option B: Detect in on_dialect_cycle callback**
- Before dispatching ChangeDialect
- Check if language will change
- Dispatch both ChangeDialect AND CreateBranch

**Option C: Create new action**
- New action: SwitchLanguage
- Handles both dialect change and branch creation
- More explicit

**Decision:** Option A - simplest, handles all dialect changes uniformly

### Detailed Implementation

**Step 1: Verify ChangeDialect is correct action**
- Confirm `on_dialect_cycle` dispatches `ChangeDialect(dialect)`
- Trace from UI button click to action dispatch

**Step 2: Implement branch creation in ChangeDialect**
```rust
UserStateAction::ChangeDialect(dialect) => {
    let old_language = next.selected_dialect.language();
    next.selected_dialect = dialect;
    let new_language = dialect.language();

    // If language changed, create a new branch for the new language
    if old_language != new_language {
        let new_branch = ConversationBranch::new(None, None, None, None);
        let new_branch_id = new_branch.id;
        next.branches.push(new_branch);
        next.active_branch_id = new_branch_id;
    }
}
```

**Step 3: Test the flow**
- Start app → "New conversation" branch
- Send Spanish message → Branch becomes "Argentinian Spanish"
- Cycle to Arabic → New "New conversation" branch created
- Send Arabic message → New branch becomes "Levantine Arabic"
- Verify: Two branches exist

### Potential Issues

**Issue 1: Are there other ways to change dialect?**
- Settings panel direct selection?
- Keyboard shortcuts?
- Need to check all dialect change paths

**Issue 2: What about CycleDialect action?**
- Is it used? When?
- Need to update both ChangeDialect and CycleDialect?

**Issue 3: Branch switching behavior**
- When user manually switches back to Spanish branch
- Should we prevent them from sending Arabic there?
- Or allow and create another branch?

**Issue 4: What if cycling within same language?**
- Spanish Mexican → Spanish Argentinian
- Should NOT create new branch (same language)
- Current logic handles this (old_language == new_language)

## Testing Checklist

Manual testing needed:
- [ ] Start app, verify single empty branch
- [ ] Send Spanish message, verify branch becomes Spanish
- [ ] Cycle to Arabic, verify new branch created
- [ ] Send Arabic message, verify new branch becomes Arabic
- [ ] Verify both branches visible in sidebar
- [ ] Switch back to Spanish branch, verify it still works
- [ ] Cycle Spanish Mexican → Spanish Cuban, verify NO new branch created

## Questions to Answer

1. What dispatches ChangeDialect?
   - Answer: `on_dialect_cycle()` in callbacks.rs

2. Is CycleDialect action ever used?
   - Need to grep for CycleDialect dispatch calls

3. Are there other dialect change mechanisms?
   - Settings panel?
   - Direct selection?

4. Should we also handle ChangeLanguage action?
   - Yes - it also changes language fundamentally

## Status

- [x] Analyzed user issue
- [x] Identified root cause
- [ ] Verified all dialect change paths
- [ ] Implemented solution
- [ ] Tested manually
- [ ] All tests pass
- [ ] Clippy clean
