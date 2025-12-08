# Language Plans - Design Brainstorm

## Current System Analysis

### Learning Goals
- Simple `String` + `Dialect`
- All active simultaneously
- Persist until manually changed
- Used in agent prompts to guide conversation style
- No structure or progression

### Learning Items
- Four types: Mistake, Explained, Translated, Exploratory
- Scored 0-100 by analysis agent
- Filtered by dialect
- Independent - no relationships between items
- No concept of "sets" or "groups"
- Generated dynamically OR added manually

### Current Limitations
1. No multi-step progression
2. No review mechanism
3. No completion criteria
4. No structured curriculum
5. Goals are vague guidance, not concrete lessons

## Language Plans - Requirements

### User's Examples
1. **Spanish prepositions**: Practice "verbs with en", then "verbs with con", then "verbs with de" - each as separate step with review between
2. **Levantine Arabic present tense**: Practice verbs by vowel pattern (fatha, kasrah, dammah) - separate steps with review

### Key Characteristics
1. **Multi-step** - Ordered sequence of focused sub-lessons
2. **Reviewed** - Periodic consolidation/practice of previous steps
3. **Structured** - Pre-defined curriculum with clear objectives
4. **Progressive** - Move through steps one at a time (or with controlled advancement)
5. **Scoped** - Focused on specific grammar/vocabulary domains

## Differences from Current System

### 1. Multi-step Structure
**Current**: Learning goals are independent, learning items are flat
**Plans**: Hierarchical structure with plan → steps → learning content

### 2. Review Mechanism
**Current**: None - items just accumulate scores until 100
**Plans**: Explicit review steps that consolidate previous material

### 3. Progression State
**Current**: No concept of "current position" in a learning sequence
**Plans**: Track which step is active, which completed, overall plan progress

### 4. Setup Interface
**Current**: Learning goals are simple text input, items added ad-hoc
**Plans**: Need plan builder UI - define plan structure upfront

### 5. Step Activation
**Current**: All goals active simultaneously
**Plans**: One step active at a time (or limited active set)

### 6. Completion Criteria
**Current**: Learning items scored to 100 (or manually deleted)
**Plans**: Need step completion logic - when to advance?

### 7. Plan-Level Metadata
**Current**: No grouping metadata
**Plans**: Title, description, dialect, overall progress, creation date

### 8. Step-Level Metadata
**Current**: Goals are just strings
**Plans**: Each step needs name, instructions, expected focus, completion criteria

### 9. Relationship to Learning Items
**Current**: Items exist independently
**Plans**: Steps might generate/expect specific learning items

### 10. Plan Lifecycle
**Current**: Goals just exist or don't
**Plans**: Not Started → In Progress → Paused → Completed → Archived states

## Data Structure Proposals

### Option A: Plans as Enhanced Learning Goals

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LanguagePlan {
    pub id: Uuid,
    pub title: String,                    // "Spanish Prepositional Verbs"
    pub dialect: Dialect,
    pub description: Option<String>,      // Optional overview
    pub steps: Vec<PlanStep>,
    pub current_step_index: usize,        // Which step is active
    pub status: PlanStatus,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanStep {
    pub id: Uuid,
    pub step_number: usize,               // 1, 2, 3...
    pub title: String,                    // "Verbs with 'en'"
    pub step_type: StepType,
    pub instructions: String,             // What to focus on this step
    pub completion_criteria: CompletionCriteria,
    pub status: StepStatus,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StepType {
    Learning {
        // Active learning step - practice specific content
        focus: String,               // "Practice present tense with fatha vowel"
    },
    Review {
        // Review previous steps
        review_step_ids: Vec<Uuid>,  // Which steps to review
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompletionCriteria {
    Manual,                              // User decides when done
    MessageCount(u32),                   // Complete after N messages
    ItemMastery {                        // Learning items reach threshold
        min_items: u32,                  // Need at least this many items
        avg_score_threshold: u8,         // Average score must be >= this
    },
    TimeSpent(u64),                      // Minutes spent on step
    Combined {                           // Multiple criteria (AND logic)
        message_count: Option<u32>,
        item_mastery: Option<(u32, u8)>, // (min_items, avg_score)
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanStatus {
    NotStarted,
    InProgress,
    Paused,
    Completed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepStatus {
    NotStarted,
    InProgress,
    Completed,
}
```

**Pros:**
- Clean separation from learning goals
- Flexible step types (learning vs review)
- Multiple completion criteria options
- Clear state tracking

**Cons:**
- Significant new data structures
- Need migration for existing users
- More complex UI

### Option B: Plans as Collections of Learning Goals

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LanguagePlan {
    pub id: Uuid,
    pub title: String,
    pub dialect: Dialect,
    pub goal_sequence: Vec<Uuid>,        // References existing LearningGoals
    pub current_goal_index: usize,
    pub review_intervals: Vec<usize>,    // After steps [2, 4, 7], do review
}
```

**Pros:**
- Reuses existing LearningGoal infrastructure
- Minimal new structures
- Simpler to implement

**Cons:**
- Learning goals weren't designed for this (no completion tracking)
- Mixing concepts (goals as plan steps feels weird)
- Less flexible

### Recommendation: **Option A**

Plans are sufficiently different from goals to warrant separate data structures. Trying to retrofit goals would create confusion.

## Integration with Agents

### Current Agent Flow
1. Response agent receives learning goals in prompt (all goals)
2. Response agent receives learning items in prompt (current dialect only)
3. Learning agent generates new learning items
4. Analysis agent scores learning items

### Plans Integration - Proposal

#### 1. Active Step as Context
When a plan is in progress, the current step becomes part of agent context:

**Response Agent Prompt Addition:**
```
# ACTIVE LEARNING PLAN
You are currently working on Step 2/5 of "Spanish Prepositional Verbs"
Current Step: "Verbs with 'con'"
Focus: Practice verbs that use the preposition 'con' (contar con, soñar con, etc.)

{step.instructions}
```

**Learning Agent Prompt Addition:**
```
# PLAN CONTEXT
Current step focuses on: {step.focus}
Look for learning opportunities related to this focus.
```

#### 2. Step-Generated Learning Items

Option: Steps could pre-seed specific learning items
```rust
pub struct PlanStep {
    // ...existing fields...
    pub seeded_items: Vec<LearningItemType>,  // Optional pre-defined items
}
```

Example: "Verbs with 'en'" step could include:
- Translation: "to insist on" → "insistir en"
- Translation: "to think about" → "pensar en"
- Exploratory: "Use 'confiar en' in a sentence about trust"

#### 3. Review Step Behavior

**Learning Mode Review:**
- Pull learning items from previous steps
- Agent focuses on using those items naturally
- No new learning items generated during review

**Review Prompt:**
```
# REVIEW MODE
You are reviewing material from Steps 1-2:
- Step 1: Verbs with 'en'
- Step 2: Verbs with 'con'

Have a natural conversation that incorporates these previously learned items.
Do NOT introduce new material. Focus on reinforcing what's been covered.
```

## Workflow & State Machine

### Plan Lifecycle

```
NOT_STARTED → (user starts plan) → IN_PROGRESS
                                        ↓
                          (user pauses) → PAUSED → (user resumes) → IN_PROGRESS
                                        ↓
                          (final step completed) → COMPLETED
```

### Step Progression

```
Step 1: NOT_STARTED → (plan starts) → IN_PROGRESS
                                           ↓
                               (completion criteria met) → COMPLETED
                                           ↓
Step 2: NOT_STARTED → (step 1 completes) → IN_PROGRESS
                                           ↓
                               (completion criteria met) → COMPLETED
                                           ↓
REVIEW: NOT_STARTED → (step 2 completes) → IN_PROGRESS
                                           ↓
                               (completion criteria met) → COMPLETED
                                           ↓
Step 3: NOT_STARTED → (review completes) → IN_PROGRESS
...
```

### Step Completion Logic

**Check completion criteria on each message:**
1. User sends message
2. Agent responds
3. Check if active step's completion criteria met
4. If met:
   - Mark step as COMPLETED
   - Move to next step (or review)
   - Update UI to show progress

**Example (ItemMastery):**
```rust
fn check_step_completion(
    step: &PlanStep,
    learning_items: &[LearningItem],
    messages_in_step: u32,
) -> bool {
    match &step.completion_criteria {
        CompletionCriteria::ItemMastery { min_items, avg_score_threshold } => {
            let step_items = learning_items.iter()
                .filter(|item| item.created_during_step == step.id)
                .collect::<Vec<_>>();

            if step_items.len() < *min_items as usize {
                return false;
            }

            let avg_score = step_items.iter()
                .map(|item| item.score as u32)
                .sum::<u32>() / step_items.len() as u32;

            avg_score >= *avg_score_threshold as u32
        }
        CompletionCriteria::MessageCount(count) => {
            messages_in_step >= *count
        }
        // ... other criteria
    }
}
```

## UI Considerations

### New UI Components Needed

#### 1. Plan Builder/Editor
- Create new plan (title, dialect, description)
- Add steps (title, type, instructions, completion criteria)
- Reorder steps
- Add review steps at specific intervals
- Save/cancel

**Location:** New page or modal from learning panel

#### 2. Plan List View
- Show all plans for current dialect
- Filter: Not Started, In Progress, Completed
- Start/Resume/Archive buttons
- Progress indicator (Step 3/7)

**Location:** New page or expandable section in learning panel

#### 3. Active Plan Display
- Show in learning panel when plan is active
- Current step name and instructions
- Progress bar (steps completed / total steps)
- "Complete Step" button (for manual completion)
- Step completion progress (e.g., "3/5 items mastered")

**Location:** Top section of learning panel (above learning items)

#### 4. Step Transition UI
- When step completes: Show celebration/transition
- "Starting Step 4: Verbs with 'de'" notification
- Option to review step before moving on

### Learning Panel Modifications

**Current Structure:**
```
┌─────────────────────────────┐
│  Learning Progress          │
│  ─────────────────────────  │
│  Accomplishments (100%)     │
│  Still Learning (<100%)     │
│  + Add Learning Item        │
└─────────────────────────────┘
```

**With Plans:**
```
┌─────────────────────────────┐
│  Active Plan: Spanish Verbs │
│  Step 3/7: Verbs with 'con' │
│  Progress: ████░░░ 57%      │
│  ─────────────────────────  │
│  Learning Progress          │
│  Accomplishments (100%)     │
│  Still Learning (<100%)     │
│  + Add Learning Item        │
│  ─────────────────────────  │
│  📚 View All Plans          │
└─────────────────────────────┘
```

## Technical Considerations

### Storage

**Add to UserState:**
```rust
pub struct UserState {
    // ... existing fields ...
    pub language_plans: Vec<LanguagePlan>,
    pub active_plan_id: Option<Uuid>,
}
```

### Agent Context Building

**Modify `UserMessageWithContext`:**
```rust
pub struct UserMessageWithContext {
    // ... existing fields ...
    pub active_plan_step: Option<ActivePlanStep>,
}

pub struct ActivePlanStep {
    pub plan_title: String,
    pub step_number: usize,
    pub total_steps: usize,
    pub step_title: String,
    pub step_type: StepType,
    pub instructions: String,
}
```

### Learning Item Association

**Track which step created which items:**
```rust
pub struct LearningItem {
    pub item: LearningItemType,
    pub score: u8,
    pub dialect: Dialect,
    pub created_by_plan_step: Option<Uuid>,  // NEW: which step created this
}
```

This enables:
- Step completion based on item mastery
- Review mode (show items from previous steps)
- Progress tracking per step

## Open Questions for User

1. **Step Advancement:** Should steps auto-advance when criteria met, or require user confirmation?

2. **Review Frequency:** Should review steps be auto-inserted (e.g., after every 2-3 learning steps), or manually placed by user?

3. **Multiple Active Plans:** Can user have multiple plans in progress, or only one active plan at a time?

4. **Plan Pause:** If user pauses a plan and just chats normally, do learning items still get generated? Do they count toward the plan?

5. **Step Types:** Are Learning + Review sufficient, or do we need other types (Assessment, Introduction, etc.)?

6. **Completion Criteria:** Which criteria are most important? Should we start simple (message count + manual) and add item mastery later?

7. **Pre-seeded Items:** Should steps come with pre-defined learning items (translations, exploratory prompts), or should all items be generated dynamically?

8. **Plan Templates:** Should there be pre-built plan templates for common learning paths (e.g., "Spanish A1 Verbs", "Arabic Present Tense")?

9. **Learning Goals vs Plans:** Do learning goals continue to exist separately, or do plans replace them entirely?

10. **Agent Behavior:** In review mode, should the agent actively work previously learned items into conversation, or just be ready to score them if user uses them?

## Implementation Complexity

### Phase 1: Minimal Viable Plans
- Data structures (Plan, Step with basic fields)
- Manual completion criteria only
- Simple learning steps (no review yet)
- Basic UI (create plan, view active plan, complete step manually)
- Agent receives active step context

**Effort:** ~5-7 phases

### Phase 2: Automatic Progression
- Completion criteria (message count, item mastery)
- Auto-advance between steps
- Step completion checking logic

**Effort:** ~3-4 phases

### Phase 3: Review Steps
- Review step type
- Pull items from previous steps
- Review-mode agent behavior

**Effort:** ~2-3 phases

### Phase 4: Polish
- Progress visualization
- Plan templates
- Step reordering
- Plan pause/resume

**Effort:** ~4-5 phases

## Recommended Starting Point

1. **Define minimal data structures** - Plan, Step, StepType (Learning only), CompletionCriteria (Manual + MessageCount)

2. **Add to UserState** - `language_plans: Vec<LanguagePlan>`, `active_plan_id: Option<Uuid>`

3. **Simple plan builder UI** - Create plan with steps, set as active

4. **Agent integration** - Pass active step in context, modify prompts

5. **Manual step completion** - Button in UI to advance to next step

This gets the core concept working with minimal complexity. Then iterate based on user feedback.

---

## DECISIONS NEEDED BEFORE IMPLEMENTATION

The following questions must be answered before proceeding with implementation:

### 1. Step Advancement
Should steps auto-advance when criteria met, or require user confirmation?

**Options:**
- A) Auto-advance immediately when criteria met
- B) Show notification + require user to click "Continue to Next Step"
- C) Configurable per plan

**Decision:** [TO BE DETERMINED]

---

### 2. Review Frequency
Should review steps be auto-inserted (e.g., after every 2-3 learning steps), or manually placed by user?

**Options:**
- A) Auto-insert review steps at regular intervals
- B) User manually adds review steps when creating plan
- C) Suggest review steps during plan creation, user can accept/reject

**Decision:** [TO BE DETERMINED]

---

### 3. Multiple Active Plans
Can user have multiple plans in progress, or only one active plan at a time?

**Options:**
- A) Only one active plan per dialect
- B) Multiple active plans allowed (complex - how do agents handle this?)
- C) One active plan across all dialects (simplest)

**Decision:** [TO BE DETERMINED]

---

### 4. Plan Pause Behavior
If user pauses a plan and just chats normally, do learning items still get generated? Do they count toward the plan?

**Options:**
- A) Paused plan = no plan context sent to agents, items generated normally but not associated with plan
- B) Paused plan = completely disabled, switch to normal chat mode
- C) Can't pause - must complete or archive

**Decision:** [TO BE DETERMINED]

---

### 5. Step Types
Are Learning + Review sufficient, or do we need other types (Assessment, Introduction, etc.)?

**Options:**
- A) Start with Learning + Review only
- B) Add Introduction (no items generated, just conversational intro to topic)
- C) Add Assessment (test knowledge, different completion criteria)
- D) Keep it simple - just Learning steps for Phase 1, add Review in Phase 2

**Decision:** [TO BE DETERMINED]

---

### 6. Completion Criteria Priority
Which criteria are most important? Should we start simple (message count + manual) and add item mastery later?

**Options:**
- A) Phase 1: Manual only
- B) Phase 1: Manual + MessageCount
- C) Phase 1: Manual + MessageCount + ItemMastery (full implementation)

**Decision:** [TO BE DETERMINED]

---

### 7. Pre-seeded Items
Should steps come with pre-defined learning items (translations, exploratory prompts), or should all items be generated dynamically?

**Options:**
- A) All items generated dynamically by agents
- B) Steps can optionally include seed items
- C) Steps MUST include seed items (helps ensure focus stays on target)

**Decision:** [TO BE DETERMINED]

---

### 8. Plan Templates
Should there be pre-built plan templates for common learning paths (e.g., "Spanish A1 Verbs", "Arabic Present Tense")?

**Options:**
- A) No templates - users create all plans from scratch
- B) Include a few starter templates
- C) Eventually build template library, but not for Phase 1

**Decision:** [TO BE DETERMINED]

---

### 9. Learning Goals vs Plans
Do learning goals continue to exist separately, or do plans replace them entirely?

**Options:**
- A) Plans replace learning goals entirely
- B) Both coexist - can have plan OR goals OR both active
- C) Plans are built FROM learning goals (goals become steps)

**Decision:** [TO BE DETERMINED]

---

### 10. Agent Behavior in Review Mode
Should the agent actively work previously learned items into conversation, or just be ready to score them if user uses them?

**Options:**
- A) Passive - agent doesn't force previous items, just scores them if used
- B) Active - agent tries to naturally incorporate previous items
- C) Semi-active - agent sets up conversational contexts where items are likely to appear, but doesn't force them

**Decision:** [TO BE DETERMINED]

---

## NEXT STEPS

1. Answer the 10 decision questions above
2. Create implementation plan based on decisions
3. Start with minimal viable implementation (Phase 1)
