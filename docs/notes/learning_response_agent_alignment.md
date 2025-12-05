# Learning Agent and Response Agent Alignment

## Problem Statement

In Corrective mode, we have two agents analyzing the user's message:
1. Learning agent - identifies mistakes and creates learning items
2. Response agent - provides corrections in conversation

These two agents don't always agree, leading to:
- No new learning item from the learning agent
- But the response agent gives a correction
- User sees correction but it's not tracked

## Brainstormed Solutions

### Option 1: Pre-Analysis (Flow Reversal)
**Change the flow: Learning Agent → Response Agent**

**Current Flow:**
1. User message → Response Agent (generates response)
2. User message + Response → Learning Agent (extracts learning items)

**Proposed Flow:**
1. User message → Learning Agent (identifies mistakes/learning opportunities)
2. User message + Learning items → Response Agent (corrects based on identified mistakes)

**Pros:**
- Single source of truth for what's a mistake
- Response agent knows exactly what to correct
- Clean alignment by design

**Cons:**
- Significant architectural change
- Response agent loses ability to make spontaneous observations
- Adds latency (sequential instead of allowing parallel extraction)

**Implementation:**
- Move learning agent call before response generation in `response.rs`
- Pass identified mistakes to response agent's context
- Update response agent prompt: "These specific mistakes were identified: [list]. Incorporate gentle corrections for these in your response."

---

### Option 2: Correction Extraction (Learning Agent Enhancement)
**Enhance learning agent to parse corrections from response text**

Keep current flow, but improve learning agent prompt to explicitly extract corrections that the response agent made.

**Pros:**
- No architectural changes
- Captures what the user actually sees
- Maintains response agent's natural conversational ability

**Cons:**
- Relies on parsing natural language (corrections might be implicit)
- Response agent's corrections might not be in parseable format
- Could extract false positives if response agent is just explaining something

**Implementation:**
- Update learning agent prompt: "Analyze the assistant's response for any corrections provided. If the assistant corrected the user (e.g., 'it should be X' or uses correct form after user's error), extract this as a mistake."
- May need to format response agent corrections more explicitly (e.g., require "X → Y" format in Corrective mode)

---

### Option 3: Response Agent Constraint
**Restrict response agent to only correct identified mistakes**

Make the response agent less autonomous - it should only provide corrections for mistakes that are already in the `past_mistakes` list.

**Pros:**
- Simple to implement (prompt change only)
- Perfect alignment - learning agent is authoritative
- No parsing needed

**Cons:**
- Might make response agent feel unnatural or robotic
- Response agent can't spontaneously notice and correct issues
- Could miss valuable teaching moments

**Implementation:**
- Update response agent prompt in Corrective mode: "Only provide corrections for mistakes that are listed in the 'Mistakes to watch' section above. Do not introduce new corrections."
- Past mistakes become more prominent in prompt

---

### Option 4: Unified Agent (Architectural Change)
**Single agent call generates both learning items AND response**

Combine both functions into one agent call that outputs:
```json
{
  "response": "...",
  "new_mistakes": [...],
  "new_explained": [...],
  // etc
}
```

**Pros:**
- Perfect alignment by design (same context, same decision)
- Single LLM call (saves latency and cost)
- No possibility of disagreement

**Cons:**
- More complex JSON output format
- Harder to get good results at both tasks simultaneously
- Less modularity (can't tune learning vs response separately)
- Bigger refactor

---

### Option 5: Criteria Alignment (Prompt Tuning)
**Align the mistake detection criteria in both prompts**

Keep current architecture, but make both agents use very similar language and criteria for what constitutes a mistake worth correcting.

**Pros:**
- Minimal changes
- Preserves flexibility
- Can iterate on prompts independently

**Cons:**
- Doesn't guarantee alignment (different models/temperatures)
- Implicit coordination is fragile
- Might still have edge case disagreements

**Implementation:**
- Extract "what is a mistake" criteria into shared text
- Include in both learning and response agent prompts
- Make learning agent more aggressive OR response agent more conservative

---

### Option 6: Two-Pass Learning Analysis
**Learning agent runs twice: pre and post response**

1. Learning agent makes initial analysis of user message
2. Response agent generates response (can see initial analysis)
3. Learning agent does second pass on user + response to finalize

**Pros:**
- Response agent can incorporate pre-identified mistakes
- Learning agent can still extract additional corrections from response
- Flexible

**Cons:**
- More complex flow
- Two learning agent calls (cost/latency)
- Overkill?

---

## Recommendation (Pre-Bug Discovery)

**Option 2 (Correction Extraction)** seemed most pragmatic:
- Keep current flow (minimal disruption)
- Update learning agent prompt to explicitly look for corrections in response
- Optionally: In Corrective mode, have response agent format corrections more explicitly

If parsing proves unreliable, fall back to **Option 1 (Pre-Analysis)** which gives clean alignment at the cost of flow reversal.

---

## Actual Bug Discovered

**Root Cause:** Learning and analysis agents are receiving ALL learning items across all dialects and branches, when they MUST receive ONLY the learning items for the current dialect and branch.

**Impact:**
- Agents are analyzing against irrelevant learning items from other contexts
- Causes misalignment when learning items from other branches/dialects don't match current conversation
- Explains why agents might disagree - they're working with wrong context

**Solution:** Filter learning items by current dialect and branch before passing to agents.

**Files to investigate:**
- Where learning items are collected before being passed to agents
- Ensure filtering happens at the right layer (probably in the handlers that call the agent service)

---

## Bug Fixed (2025-12-05)

**Location:**
- `frontend/src/app/helpers.rs` - `extract_learning_items()` function
- `frontend/src/app/callbacks.rs` - Call site in `on_send_message()`
- `backend/src/websocket/agents.rs` - Removed redundant backend filtering

**Changes Made:**

1. **Frontend filtering** (where UserMessageWithContext is built):
   - Modified `extract_learning_items()` to accept `dialect` parameter
   - Use `state.get_learning_items_for_dialect(dialect)` to filter by current dialect
   - Pass `state.selected_dialect` from call site
   - Frontend now filters before sending to backend

2. **Backend simplification**:
   - Removed `filter_learning_items()` function and `FilteredLearningItems` struct
   - Backend now trusts frontend-filtered items in `msg_with_context`
   - Analysis agent uses same items as response/learning agents
   - Single source of truth: frontend-provided context

**Result:**
- All three agents (response, learning, analysis) now receive identical, correctly filtered learning items
- No more agent disagreement due to mismatched contexts
- Frontend is authoritative for what learning items are relevant
