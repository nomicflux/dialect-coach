# Post-Mortem: Plan Generator JSON Parsing Failure (500 Error)

## Incident Summary
**Date:** 2025-12-13
**Component:** Backend (`PlanGenerator`)
**Impact:** Plan generation functionality completely broken for users due to 500 Internal Server Errors.
**Error:** `Failed to parse generated plan JSON`
**Root Cause:** "Happy Path" implementation of LLM generation logic. The `PlanGenerator` blindly trusted the LLM to output valid JSON on the first try (with simple network retries) and failed to utilize the existing, robust `retry_with_error_feedback_tracked` mechanism present in the codebase.

## Technical Analysis

### The Failure Pattern
The `PlanGenerator` implementation used the low-level `retry_completion_call`:
```rust
// backend/src/agent_service/planning/mod.rs
let (result, _) = retry::retry_completion_call(self.agent.as_ref(), &request, 3).await;
let normalized = normalize_json_response(&response);
let plan: SimpleImportLanguagePlan = serde_json::from_str(&normalized)?;
```
`retry_completion_call` **only** retries on:
1. Network failures
2. Empty responses

It **does not** retry on:
1. Malformed JSON
2. Hallucinated markdown (beyond simple normalization)
3. Schema violations

When the LLM returned invalid JSON (which is statistically inevitable), the `serde_json::from_str` failed immediately, returning a 500 error to the user.

### The Missed Solution
The codebase already contained a robust solution in `backend/src/agent_service/retry.rs`:
```rust
pub async fn retry_with_error_feedback_tracked<T, F>(...)
```
This method:
1. Attempts to parse the response.
2. If parsing fails, constructs a **new prompt** containing the error message (e.g., "Your JSON was missing a brace") and the previous failed response.
3. Feeds this back to the LLM to self-correct.
4. Repeats for up to 3 attempts.

This pattern is successfully used in `backend/src/agent_service/response/generation.rs` but was ignored in the new `PlanGenerator` implementation.

## Lessons Learned
1. **Research Before Implementation:** NEVER implement a new feature (especially a complex one like an agent) without first reading and understanding the implementation of what you are building with.
2. **Respect Architecture:** If an architecture exists (like `RetryContext`), use it. Do not invent "simpler" (fragile) versions of solved problems.
3. **"Blind Usage" is Dangerous:** Copying *imports* without copying the *robustness patterns* leads to fragile code.

## Action Items
1. **Refactor `PlanGenerator`**: Completely rewrite the generation logic to use `RetryContext::retry_with_error_feedback_tracked`, mirroring the robust pattern in `enrichment.rs`.
2. **Update `LESSONS_LEARNED.md`**: Document the necessity of architectural research.
