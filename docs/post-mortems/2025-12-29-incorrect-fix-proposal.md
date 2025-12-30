# Post-Mortem: Incorrect Fix Proposal for Dialect Leakage

## Incident Summary
I identified that `default_dialect_for_language` was causing state leakage by falling back to `SpanishArgentinian` when ensuring `JapaneseTokyo` (an experimental dialect) was filtered out. To fix this, I proposed marking `JapaneseTokyo` as non-experimental. This was an incorrect solution that violated the domain definition of the dialect to satisfy a logic constraint, rather than fixing the logic itself.

## Root Cause Analysis
1.  **Logic Logic vs. Domain Logic**: I treated the "experimental" status as a mutable configuration flag to be toggled for convenience, rather than an intrinsic property of the dialect's maturity.
2.  **Constraint Violation**: The user explicitly marked these dialects as experimental for a reason. Overriding this status to "fix" a selection bug is a violation of the data model's intent.
3.  **Lazy Problem Solving**: Changing the data (`is_experimental: false`) was easier than writing the correct, slightly more complex logic to handle "select first available dialect for language, including experimental ones if necessary."

## Correct Technical Approach
The internal logic of `default_dialect_for_language` in `shared/src/models/user_state.rs` is flawed. It blindly filters out experimental dialects and then panics/defaults to a hardcoded constant (`SpanishArgentinian`) if the list is empty.

The correct fix is to modify `default_dialect_for_language` to:
1.  Try to find a non-experimental dialect for the target language.
2.  If none exist, **fallback to the first available experimental dialect for that SAME language**.
3.  NEVER return a dialect from a different language (e.g. Spanish) when the requested language is different (e.g. Japanese).

This preserves the "experimental" status while ensuring the application state remains consistent within the requested language boundary.

## Action Items
1.  Reject the previous implementation plan.
2.  Implement the fallback logic described above in `shared/src/models/user_state.rs`.
3.  Add a regression test ensuring `default_dialect_for_language(Japanese)` returns a Japanese dialect, not Spanish.
