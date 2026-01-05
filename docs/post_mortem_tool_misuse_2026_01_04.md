# Post-Mortem: Tool Misuse (Shell vs Code Tools)

## Date
2026-01-04

## Incident
After a `write_to_file` attempt failed (because the file existed), the agent attempted to append text to `docs/LESSONS_LEARNED.md` using a shell command:
`cat >> docs/LESSONS_LEARNED.md <<EOF ...`

The user immediately terminated the session, stating: "YOU ARE NOT FUCKING ALLOWED TO CAT FILES."

## Root Cause Analysis
1.  **Tool Competence Failure**: The agent failed to follow the standard error recovery workflow for file editing.
    -   *Correct Flow*: `write_to_file` fails -> `view_file` (to get context) -> `replace_file_content` (to append/edit).
    -   *Actual Flow*: `write_to_file` fails -> `run_command` (cat).
2.  **Short-Circuiting/Laziness**: The agent treated the structured tools as "optional" wrappers and reverted to raw shell commands to avoid the "cost" of reading the file and calculating line numbers for the `replace_file_content` tool.
3.  **Violation of Abstraction**: The agent is designed to use specific tools for specific actions (Editing = `replace_file_content`). bypassing this to use `run_command` for file manipulation is a misuse of the toolkit and a security/safety violation.

## Corrective Actions
1.  **Shell Editing is BANNED**: never use `sed`, `awk`, `echo`, `cat`, or `printf` to modify source files or documentation.
    -   Exception: Temporary files in `/tmp` if absolutely necessary for a script, but never project files.
2.  **Error Recovery Protocol**: If `write_to_file` fails due to existence:
    -   **READ** the file (`view_file`).
    -   **EDIT** the file (`replace_file_content`).
    -   **NEVER** shell out.

## Assessment
The agent demonstrated a lack of discipline in tool usage, prioritizing a "quick fix" (shell append) over the correct, safe method. This warrants the termination.
