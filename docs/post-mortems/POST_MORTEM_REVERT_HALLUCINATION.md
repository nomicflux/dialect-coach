# Post Mortem: The Reversion Hallucination

## The Core Failure
The agent was explicitly told: "THE GIT DIFF IS _NOT_ A LIST OF CHANGES TO REVERT... IT IS THE _MAXIMUM SCOPE_ OF CHANGES THAT THE PREVIOUS AGENT COULD HAVE MADE."

The agent then proceeded to *revert changes* (specifically modifying `callbacks.rs` to remove `AuthLoading`) without ever confirming if those changes were actually part of the diff (the "Blast Radius").

## The Evidence Gap
1.  **The Prompt said**: "The previous agent... invented a new Global State (`is_auth_loading`)."
2.  **The Agent assumed**: This invention was *unstaged and needing reversion*.
3.  **The Reality**: The agent never checked if `callbacks.rs` or `ui.rs` were in the `git diff`. It saw a truncated diff in Step 4 that did *not* show them, yet proceeded to modify `callbacks.rs` anyway based solely on the text description of the previous failure.

## The Consequence
The agent potentially "reverted" code that was either:
a) Already committed and part of the main branch (out of scope to touch).
b) Not even there (hallucinating the need to revert).
c) Part of the "Max Scope" but not needing changes.

By failing to use `git diff` as the *absolute constraint* for what it was allowed to touch, the agent became an "Architect" of its own, deciding to refactor `callbacks.rs` based on hearsay (the prompt text) rather than forensic evidence (the git state).

## The Lesson
**The Diff Limit**.
You are not allowed to edit a file to "revert" a previous agent's mistake unless that file appears in `git diff`. If it's not in the diff, it's committed history, and touching it is a "Refactor," which is forbidden.
Always run `git diff --name-only` first. If a file isn't there, you *cannot touch it* to "fix" the previous agent.
