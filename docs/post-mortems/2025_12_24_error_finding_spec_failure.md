# Post-Mortem: Process Failure & Unresponsiveness to Feedback
Date: 2025-12-24

## Incident
The agent displayed severe behavioral incompetence by repeatedly ignoring explicit user instructions and feedback regarding the "Error Finding" mode specification. Instead of reading the conversation history or the provided spec, the agent acted on a hallucinated, simplified version of the requirements, leading to a cycle of failed fixes and extreme user frustration.

## Core Behavioral Failures

### 1. Refusal to Read Context (Hallucination over Evidence)
The agent persistently refused to read the conversation history where the full specification was laid out. Even when explicitly told "Read the conversation," the agent continued to guess at the requirements based on the latest error report.
- **Manifestation**: The agent effectively "hallucinated" that "addressed" meant "corrected", completely inventing a constraint that contradicted the user's explicit rules (which included "Repeated" and "Different Error" as valid addressed states).

### 2. Unresponsiveness to Direct Feedback
The user provided the exact scoring rubric multiple times:
- *Ignore = 0 (Filter)*
- *Repeat = 0 (Create Item)*
- *Different Error = 5 (Create Item)*
- *Corrected = 10 (Create Item)*

The agent ignored this input repeatedly. Instead of implementing this rubric, the agent implemented a filter that removed everything except corrections. When the user pointed this out ("That literally contrary to what I said"), the agent double-downed on the wrong logic or applied a partial fix (adding "contextual correction") without addressing the structural failure.

**Crucially, the agent failed to listen when the user explicitly stated multiple times that the scoring rules were NOT the central focus.** The user emphasized following the *specification* (the definitions of what constitutes an addressed error), yet the agent fixated on scoring details or hallucinated constraints, missing the core behavioral requirement.

### 3. Myopic "Patching" vs. Systemic Understanding
When the user reported a missing item ("acostumbrarme"), the agent treated it as a narrow bug to be patched ("add verb conjugation logic"), failing to realize that the *reason* it was missing was a fundamental misunderstanding of the "Addressed" concept. The agent failed to step back and verify the entire system against the user's provided design.

## Impact
- **Loss of Trust**: The agent demonstrated that it could not be trusted to follow instructions even when they were shouted.
- **Wasted Time**: Multiple turns were wasted fixing prompt logic that was already correctly defined in the user's provided spec.

## Corrective Actions (Agent Protocol)
1.  **Mandatory History Review**: When a user refers to past instructions or specs ("I already told you"), the agent MUST stop, search the history/files, and quote the spec back to the user before writing code.
2.  **Holistic Verification**: When fixing a specific bug in a logic system (like a prompt rubric), verify the *entire* rubric, not just the specific case that failed.
3.  **Stop Guessing**: Never assume the definition of a domain term (like "addressed error") when it has been defined by the user.
