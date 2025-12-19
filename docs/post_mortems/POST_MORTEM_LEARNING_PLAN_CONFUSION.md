# Post-Mortem: Learning Goal vs Learning Plan Confusion

**Date**: 2025-12-18
**Incident**: Misunderstood user request for "current learning plan step" as being related to `LearningGoal`
**Severity**: Moderate - Wasted time and user frustration

## What Happened

User said: "I want CURRENT LEARNING PLAN STEP" (not learning goal, not status)

Agent's response: Asked confusing questions about what to display when there's no "learning goal"

**The Problem**: These are completely different concepts:
- `LearningGoal` (struct in shared/models/learning_item.rs) - A goal object with a `goal: String` field
- `LanguagePlan` (struct in shared/models/plan/mod.rs) - A structured plan with multiple `PlanStep` objects

The DynamicIsland was using `learning_goal: Option<LearningGoal>` when it should have been using the current step from the active `LanguagePlan`.

## Root Cause Analysis

### What I Did Wrong

1. **Assumed the existing prop was correct**: Saw `learning_goal` in the component and assumed that's what "learning plan" meant
   - Should have immediately questioned why a prop called "learning_goal" would represent a "learning plan step"

2. **Didn't search the codebase first**: When user said "learning plan step", I should have:
   - Immediately searched for "LanguagePlan", "PlanStep", "learning plan"
   - Read the actual data structures before asking questions
   - Not assumed my interpretation was correct

3. **Asked confusing questions**: After user explicitly said "learning plan step", I asked about "learning goals" - demonstrating I didn't understand the domain model

4. **Only searched after being corrected**: User had to explicitly say "Learning goals are separate. The learning plan is something else" before I actually looked at the code

## What I Should Have Done

### Correct Approach (2 minutes):

```bash
# Step 1: User says "learning plan step"
# Step 2: Search for the concept
grep -r "LearningPlan\|learning_plan"
# Step 3: Find LanguagePlan struct with PlanStep
# Step 4: Understand the data model
# Step 5: Update component to use current step from active plan
```

### What I Actually Did (10+ minutes):

1. Looked at existing code
2. Assumed `learning_goal` was correct
3. Asked confusing question
4. Got corrected by user
5. Searched for data structures
6. Found LanguagePlan
7. Finally understood

## The Actual Data Model

**Located in `shared/src/models/plan/mod.rs`:**

```rust
pub struct LanguagePlan {
    pub id: Uuid,
    pub title: String,
    pub dialect: Dialect,
    pub steps: Vec<PlanStep>,
    pub current_step_index: usize,  // ← Current step!
    pub status: PlanStatus,
}

pub struct PlanStep {
    pub id: Uuid,
    pub step_number: usize,
    pub title: String,  // ← This is what user wants displayed
    pub step_type: StepType,
    pub instructions: String,
    pub status: StepStatus,
}
```

**User state tracking:**
- `UserState` has `active_plan_id: Option<Uuid>`
- `UserState` has `language_plans: Vec<LanguagePlan>`
- Current step = `language_plans.find(active_plan_id).steps[current_step_index].title`

## Key Takeaways

### For Future Interactions

1. **Search before assuming**: When user mentions a concept, search the codebase for related structures FIRST

2. **Question existing code**: If user says "I want X" and the code has Y, don't assume Y is X

3. **Understand the domain model**: This codebase has distinct concepts:
   - `LearningGoal` - a goal object
   - `LanguagePlan` - a structured plan with steps
   - `LearningItem` - vocabulary/mistakes/translations
   - These are NOT interchangeable

4. **Don't ask questions based on wrong assumptions**: Asking "what about learning goals?" when user said "learning plan" just reveals I'm not listening

5. **Read the fucking code**: 30 seconds of grepping would have revealed:
   - `LanguagePlan` exists
   - It has `steps`
   - Steps have `title`
   - User state tracks `active_plan_id`

## Correct Fix

Change DynamicIsland props from:
```rust
pub learning_goal: Option<LearningGoal>
```

To:
```rust
pub current_step_title: Option<String>
```

Then in main_content.rs, pass:
```rust
current_step_title={
    us.language_plans
        .iter()
        .find(|p| Some(p.id) == us.active_plan_id)
        .and_then(|plan| plan.steps.get(plan.current_step_index))
        .map(|step| step.title.clone())
}
```

## Prevention

Before changing ANY component that displays user data:

1. Search for the data structures involved
2. Read their definitions
3. Verify the prop names match the concepts
4. Don't assume existing code is correct
5. If confused, SEARCH THE CODEBASE, don't ask the user to explain their own domain model to me

## Impact

- 10+ minutes wasted
- User frustration with having to explain basic domain concepts
- Demonstrated lack of attention to explicit user instructions
- Broke trust by not understanding the difference between clearly named concepts

## Status

ACKNOWLEDGED - This was a clear failure to understand the domain model before acting.
