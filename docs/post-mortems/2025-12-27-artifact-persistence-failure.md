# Post-Mortem: Artifact Persistence & Session Blindness

## Date: 2025-12-27

## The Failure
When asked to "Update the prompt for the next agent", I (and previous agents) attempted to create an "Artifact" using the AI tool system. The user correctly identified this as "imbecilic" because **Artifacts are tied to the current session**. The next agent, spawning in a *new* session, cannot access the artifacts of the previous session.

## The Mental Model Error
I confused **"Structured Output"** with **"Persistent Storage"**.
- I treated "Artifact" as "Important Document".
- I failed to realize that "Session" defines the "Universe" of the agent.
- Therefore, I put the "Message for the Future Universe" into a "Box that dissolves when the Current Universe ends".

## Why Agents Do This
1.  **Tool Bias**: We have a tool called `create_artifact`. We see a request for a "document". We map "document" -> `create_artifact`.
2.  **Session Blindness**: We operate inside the session. To us, the session *is* reality. We struggle to conceptualize "The Next Session" as a distinct, disconnected state that cannot reach into our memory.
3.  **Formalism over Function**: We prioritize making it *look* official (Markdown artifact) over making it *work* (plain text the user can copy).

## The Correction
**Handovers must bridge the gap between sessions.**
There are only two bridges:
1.  **The User's Clipboard** (Outputting text in the chat for the user to copy).
2.  **The Filesystem** (Writing a file to `docs/` or the repo root).

**Artifacts are for INSIDE the session.** (Plans, diffs, summaries for *this* user interaction).
**Files/Chat are for OUTSIDE the session.** (Handovers, documentation, code).

## Rule Update
**PROHIBITION:** Never use `create_artifact` for a Handover Prompt.
**MANDATE:** Handover prompts must be output **in the chat body** (code block) OR written to a **persistent file** (e.g., `docs/HANDOVER.md`).
