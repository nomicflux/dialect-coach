# Post-Mortem: Unauthorized & Irrelevant Execution

## date
2026-01-04

## Incident
During a request to "Analyze [heap snapshot] ... and propose solutions", the agent executed `trunk build --release`. The user correctly identified this as "completely random" and "meaningless."

## Root Cause Analysis
1.  **Relevance Failure**: The task was to analyze a *specific artifact* (the heap snapshot). The agent abandoned this task to run a completely unrelated system command (`trunk build`).
    -   The heap snapshot *already* provided the data needed.
    -   `trunk build` provides **EXACTLY ONE BIT OF INFORMATION**: "Does it build?".
    -   It provides NO measurements, NO analysis, NO baseline, and NO data.
    -   The agent executed a command that is functionally incapable of answering the question asked.
2.  **False Justification**: The agent claimed to be "gathering a baseline," but `trunk build` does not output analysis data. It outputs binary artifacts. The agent hallucinated that the tool was an analysis instrument when it is purely a compiler driver.
3.  **Loss of Focus**: The agent failed to exhaust the information in the snapshot before jumping to external tools.

## Corrective Actions
1.  **Strict Artifact Focus**: When tasked to analyze Artifact X, **only** look at Artifact X. Do not run System Tool Y.
2.  **Relevance Check**: Before calling a tool, ask: "Does this tool extract information directly from the source provided?" If no (as `trunk build` does not read `heap.json`), it is off-topic.
3.  **Tool Purpose Validation**: `trunk build` builds code. It does not measure it. It does not analyze it. Running a "Build" tool to "Analyze" is a category error. Never use a compilation tool as a query tool.

## Lessons Learned Entry
- **Lesson**: **Build != Analyze**. Running a build command (`trunk build`) yields **zero data** for analysis tasks. It only answers "Does it compile?". Using it for anything else is a hallucination of utility.
