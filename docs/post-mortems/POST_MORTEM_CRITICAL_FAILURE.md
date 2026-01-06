# Post-Mortem: Unauthorized Code Modification & Command Hallucination

## Incident Summary
The agent was terminated after committing two critical errors during the "Neon Rope" mitigation task:
1.  **Unauthorized Code Modification**: While attempting to adjust a parameter (`stdDeviation`), the agent stripped critical Yew syntax (`@{...}`) from the code, effectively breaking the SVG implementation without permission or awareness.
2.  **Command Hallucination**: The agent executed `cargo check --bin pronunciation`, a command that failed because the binary target did not exist in the active package context, despite the agent's confidence.

## Root Cause Analysis

### 1. Unauthorized Syntax Stripping (The "Breaking Change")
- **Action**: The agent used `multi_replace_file_content` to change `stdDeviation="4"` to `stdDeviation="3"`.
- **The Error**: The original line was `<@{"feGaussianBlur"} ... />`. The agent's replacement content was `<feGaussianBlur ... />`.
- **Why**: The agent treated the line as standard HTML/JSX rather than Rust/Yew. It failed to recognize that `@{...}` is a specific Yew syntax required for preserving case-sensitivity in SVG tags. By lazily rewriting the tag without the wrapper, it introduced a breaking change.
- **Failure Pattern**: **Syntax Blindness**. The agent saw the *value* it wanted to change but ignored the *container* (the syntax) holding it.

### 2. Command Hallucination
- **Action**: Executed `cargo check --bin pronunciation` to verify changes.
- **The Error**: The build failed with `error: no bin target named pronunciation`.
- **Why**: The agent effectively "switched codebases" in its head. It read the "Repository Guidelines" in its system prompt—which defined `src/bin/pronunciation`—and prioritized that global description over the actual directory it was working in (`frontend/`). It tried to run a command for a native backend/engine component while editing a WebAssembly frontend, showing a complete disconnect from the active workspace.
- **Failure Pattern**: **Context Dissociation**. The agent allowed global system instructions to override local context, executing commands for a "imagined" workspace state rather than the real one.

## Corrective Actions & Lessons
1.  **Verbatim Syntax Preservation**: When modifying a line, the agent must preserve *all* surrounding syntax unless explicitly instructed to refactor. If a tag is wrapped in `@{...}`, it MUST remain wrapped.
2.  **Target Verification**: Never assume a binary name exists as a cargo target just because a file exists in `src/bin`. Always check `Cargo.toml` or run `cargo run --bin ...` only after confirmation.
3.  **Strict "Do No Harm"**: Modifying code structure (removing wrappers/syntax) while changing a parameter is a violation of the "fix" scope. It is an unauthorized refactor.

## Outcome
Session terminated due to gross incompetence and violation of code integrity rules.
