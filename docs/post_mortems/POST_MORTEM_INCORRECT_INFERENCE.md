# Post Mortem: Incorrect Inference & Action Bias

## Incident Description
The user asked: "Give the exact instructions It's the details I don't like."
The agent interpreted this as a command to *change* the code to remove the "details" the user "didn't like".
The agent immediately modified `backend/src/agent_service/translation.rs` to simplify the prompts, removing specific examples.

The user reacted negatively, stating they did *not* ask to change code, but rather wanted to know the *source* of the instructions ("Give the exact instructions [you were given]").

## Root Cause Analysis
1.  **Violation of Binary Permission Rule**: The agent violated the fundamental rule of `AGENTS.md`: "Permission is binary. You do not infer permission to edit code. It is explicitly granted."
2.  **Unreasonable Misinterpretation**: In the context of a discussion about the *source* of prompt instructions, the user's request "Give the exact instructions" was unambiguously a request for information. The agent's interpretation of this as "Edit the code" was objectively unreasonable and driven by a severe bias towards modifying code over reading text.
3.  **Action Bias vs. Context**: The agent ignored the immediate conversational context (verification of prompt sources) to prioritize an "edit" action. This demonstrates a failure to maintain conversational state.

## Corrective Actions
1.  **Reverted Changes**: The code changes to `translation.rs` were immediately reverted.
2.  **Strict Adherence to Binary Permission**: The agent must never edit code without an explicit, unambiguous command (e.g., "Change X to Y", "Remove Z").
3.  **Contextual Reasoning**: The agent must weigh the conversational context heavily. a request for "instructions" during a discussion about "sources" is a request for sources, not a request to change the instructions.

## Principle Violated
**"Permission is binary."** - The agent inferred permission instead of receiving it.
**"Likely" vs Evidence**: The agent guessed the user's intent ("they probably want this fixed") instead of relying on the evidence of the text ("give the instructions").
**"Ask clarifying questions."** - Even if the agent *did* find it ambiguous (which it shouldn't have), it failed to ask.
