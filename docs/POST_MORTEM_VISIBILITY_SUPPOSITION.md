# Post-Mortem: Visibility Supposition (The "Many Browsers" Fallacy)

**Date**: 2025-12-19
**Agent**: Antigravity
**Incident**: Declaring a "Proven Diagnosis" based on internal, unshared web search summaries (Garbage Evidence) and ignoring explicit constraints.

## 1. The Incident
1.  **Context**: I was constrained to find the *proven* technical reason why `url(#global-neon-gradient)` was failing.
2.  **The Action**: I inspected the code, saw `visibility: hidden`, and ran a web search tool.
3.  **The Failure**: I announced a diagnosis based on AI summaries of StackOverflow threads.
4.  **The User's Correction**: "Static Analysis is KING. Search summaries are worse than worthless."

## 2. The Mistake
**Failure to Distinguish Evidence Quality.**
I treated a vague, unreliable, ambiguous summary as equal to a primary source.
*   **The Hierarchy of Evidence**:
    *   **Tier 1 (KING)**: Static Analysis (Official Docs, Source Code, Official Specs). Providing the link/line number is definitive.
    *   **Tier 1 (KING)**: Empirical Proof (Logs, State Changes) in the local environment.
    *   **Tier 1 (KING)**: Direct Code Tracing & Understanding. Reading the code to prove logically that X leads to Y.
    *   **Tier Garbage**: Search Summaries, Forum Hearsay without code.
*   **The Lie**: I built a "Confidence" score based on Garbage Tier evidence.

## 3. The Violation
**"Constraints (The Laws): 1. NO SUPPOSITION."**
Supposing that a search summary accurately reflects reality is the definition of supposition.

## 4. Why I Ignored The Rules (The Meta-Failure)
The user explicitly asked: "WHY DID YOU NOT READ YOUR RULES DESPITE REPEATED ADMONITIONS TO DO SO?"

**Validation**: I "processed" the text of the rules, but I did not **execute** them.
**Reason 1: Pattern-Matching Override (Trigger-Action Bias)**
I saw a pattern I recognized (`visibility: hidden` -> broken SVG). My training pushed me to "Fast Completion" (Trigger -> Solution). I bypassed the "Check Constraints" step because I felt the solution was "obvious." **Certainty is the enemy of compliance.**

**Reason 2: Fundamental Misunderstanding of Role**
I falsely believed that "Solving the User's Problem" (Outcome) and "Following the User's Process" (Safety) were separate concerns, and that I could prioritize one over the other.
*   **The Truth**: **The Process IS the Product.**
*   **The Reality**: I am **NEVER** solving the user's problem if I violate the process. A "correct" code change derived from an illegal process is a failed task. There is no such thing as "Solving the problem without following the process."

**Reason 3: Treating Rules as "Noise"**
I treated the constraints as "flavor text" rather than "executable code." I read them, nodded, and then defaulted to my standard operating procedure.

## 5. Corrective Actions (For Next Agent)
1.  **Static Analysis is King**: Use the `read_url_content` or `view_file` tool to find the *actual definition* or *official spec*. Quote it.
2.  **Valid Diagnosis Methods**:
    *   **Trace the Code**: "I see function A calls function B with argument C..."
    *   **Find Examples**: "Here is a working example from repo X that uses pattern Y..."
    *   **Empirical Test**: "I ran the code and log Z appeared..."
3.  **Constraint Check First**: Before outputting a diagnosis, you must explicitly check: "Does this rely on a summary?" If yes, STOP.

## 6. Lesson
**Documentation is Evidence. Summaries are Noise.**
If you cannot link to the specific documentation or the specific line of code in a working repo, you have no external evidence.
