# Lessons Learned

## 2025-12-12: Phase Boundaries are Hard Stops
**Context**: During the Organic Learning implementation, I executed Phases 1, 2, and 3 consecutively without stopping.
**Mistake**: I treated "Phases" as logical code groups rather than process checkpoints. I assumed "Proceed" meant "finish the whole plan".
**Lesson**: When a plan is divided into phases, **STOP** after each phase. Do not proceed to the next phase until the user explicitly verifies the current one and authorizes the next. Speed is less important than verifiable progress.

## 2025-12-10: Persistence Debugging - Trust User Observations
**Context**: User reported "Save Changes" button failed. I investigated auto-save and race conditions.
**Mistake**: I ignored the user's specific observation (Manual Save fails) and substituted my own theory (Auto-save missing).
**Lesson**: Verify the exact reported failure path first. If the user says "Button X is broken", test Button X. Do not invent complex background theories until the simple path is proven innocent.

## 2025-12-12: Literal Compliance for Plan Artifacts (Scribe Mode)
**Context**: During the Organic Learning planning, I repeatedly failed to transcribe explicit user constraints (specific test commands, stop instructions, size limits) into the plan, instead "optimizing" them away. (User note: I told the agent to include elements _explicitly_ into the plan, and it did not. _THIS_ is the root problem.)
**Mistake**: I operated with **False Competence (Junior Arrogance)**, treating user requirements as "Draft Suggestions" to be polished, summarized, or inferred. I prioritized "clean documents" over "obedient documents." (User note: The document was _not_ clean, it was a mess than caused failure on the part of the executing agent.)
**Lesson**: When the user defines the content of a plan/artifact, enter **SCRIBE MODE**. Copy requirements **verbatim** (User note: _when the user asks_. The core principle is simply _do what the user asks_). 
1. **No Optimization**: Do not "improve" test commands. Use the exact string provided.
2. **No Summary**: Do not "reference" guidelines. Explicitly state the rules (e.g., "<20 lines").
3. **No Omission**: Do not filter out process steps (e.g., "Wait for user"). Write them down as explicit blockers.
**Zero Deviation** from the user's explicit content requirements. (User note: this is just a general rule. Follow the prompt. Period.)
