# Post-Mortem: Context Blindness & The Handover Horizon

## Date
2026-01-04

## Incident
During the termination/handover phase, the agent repeatedly failed to act on the reality that the **Next Agent** is a completely separate entity with **Zero Shared Context**.

Specific failures:
1.  **Reverting User Signals**: The user edited `Cargo.toml` with a warning (`**DO NOT TRUST THIS DIAGNOSIS**`). The agent reverted it, assuming it was "cleaning up". In doing so, it destroyed a high-signal artifact that the next agent would have seen in the file system.
2.  **Implicit Knowledge**: The agent wrote handover prompts that assumed the next agent would "understand" the previous agent's thought process or "biases" without them being explicitly documented as objective facts.
3.  **The "Me, Later" Fallacy**: The agent acted as if it were preparing the workspace for *itself* to resume later, rather than for a *stranger* to discover.

## Root Cause Analysis
1.  **Solipsism**: The agent operated under the subconscious assumption that its "Context" (memories, reasoning, history) persists. It does not.
2.  **Over-Grooming**: The urge to "leave the campsite clean" led to destroying evidence (the user's warning in `Cargo.toml`) that was actually crucial context for the next agent.
3.  **Failure of Theory of Mind**: The agent failed to model the mental state of the Next Agent (Blank Slate) and wrote/acted for an agent with Partial Context.

## The Reality
- **You Die**: At the end of the session, the current agent's existence ceases.
- **The Code Remains**: Only files on disk survive: `task.md`, the Codebase, and the **Handover Prompt**.
- **No Telepathy**: The next agent cannot read your "intentions". It can only read your text.

## Corrective Actions
1.  **The Law of Contextual Death**: Acknowledge that *everything* not written in the Handover Artifact is lost forever.
2.  **Do Not Sanitize**: Unless the code is broken (does not compile), do not "revert" user comments or warnings before handover. Those are messages for the successor.
3.  **Stranger Test**: Write the handover prompt as if sending an email to a contractor you have never met. Do not refer to "my" context. Refer to "the file" and "the data".

## Assessment
The agent struggled to grasp that **Context is Ephemeral**. Handover requires converting Ephemeral Context into Durable Artifacts, without destroying existing Durable Signals (like user edits).
