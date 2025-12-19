# Post-Mortem: The Inverted Research Failure

## Incident Description
I was tasked with finding "three working patterns" for `cargo chef`. Instead of finding working examples and reporting what they did, I **invented** three theoretical patterns ("Standard", "Explicit Copy", "Manifest Extractor") first, and then spent multiple turns trying to force GitHub search results to fit these pre-conceived notions. This led to:
1.  **Thrashing**: Repeated, specific searches for a pattern ("find . -name Cargo.toml") that might not exist in popular repos.
2.  **Confirmation Bias**: Interpreting the `blastrider` repo as a match for "Manifest Extractor" when it was actually doing something else (copying system libs), because I was desperate to validate my hypothesis.
3.  **Wasted Time**: Spending resources looking for a "unicorn" instead of analyzing what is actually used in the wild.

## Root Cause: Hallucinated Objectives
The prompt asked for research (Discovery). I turned it into Verification (proving my theories).
*   **My thought process**: "I bet there's a pattern that uses `find` to extract manifests. I will find it." -> **Hallucination**.
*   **The Reality**: If I haven't seen it, I don't know it exists.

## The "Fake Research" Redux
By defining the search targets *before* seeing the data, I blinded myself to the actual patterns present in GitHub. I wasn't looking at the repos to see *how* they worked; I was checking boxes on a mental checklist of "patterns I need to find". This resulted in "finding" things that weren't there (the false positive).

## Corrective Action: Discovery-First Research
Research must be **inductive**, not **deductive**.
1.  **Input**: Search for `filename:Dockerfile "cargo chef prepare"`.
2.  **Observation**: Open 5-10 distinct, highly-starred or active repositories.
3.  **Analysis**: Read their Dockerfiles. What do *they* actually do?
4.  **Output**: "I found these 3 strategies used by Project A, Project B, and Project C."

I must stop trying to be "smart" by predicting solutions and start being "observant" by reading actual code.
