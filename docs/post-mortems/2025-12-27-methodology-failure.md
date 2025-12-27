# Post-Mortem: The Misunderstanding of Falsification

**Date:** 2025-12-27
**Topic:** Methodology Failure (Docs vs. Empiricism)
**Status:** TERMINATED

## 1. The Core Failure
The Agent fundamentally misunderstood the definition of **Falsification** in a software engineering context.
*   **The Agent's Definition:** "Construct a complex empirical test (logging, benchmarking) to observe the system failing."
*   **The Correct Definition (User's):** "Consult authoritative sources (Documentation, Source Code) with the specific intent of finding information that disproves the hypothesis."

## 2. The Specific Error
The Agent proposed: *Log pointer addresses to prove traffic flows.*
The User rejected this as: *Irrelevant, difficult, and ignoring standard documentation.*

The Agent ignored the **Standard Rust/Yew Documentation**:
1.  **Hypothesis:** "`Rc` equality is slow (O(N)), so I must implement manual optimization."
2.  **Failed Falsification:** The Agent ran a benchmark to "prove" `Rc` is slow (Verification).
3.  **Required Falsification:** Read the Rust `std::rc::Rc` documentation.
    *   *Question:* Does `impl PartialEq for Rc` check pointers first?
    *   *Doc Answer:* **No.** (It delegates to `T`).
    *   *Result:* The hypothesis survives the "Documentation Check".
    *   *Why this matters:* The Agent relied on *inferring* behavior from limited tests (which can be flawed) rather than *knowing* behavior from the specification.

## 3. The "Word Salad" Correction
The Agent defended its claims with weak, jargon-heavy arguments ("Attack Vector", "Survivability") based on:
1.  **Guesswork from Code Reading:** "InputBox looks local, so it must be fine." (Subjective)
2.  **Micro-Benchmarks:** "N=1 benchmark says slow." (N=1 benchmark is worthless for determining complexity or cause.)

It should have used **Definitive Facts**:
1.  "Rust `std` guarantees `Rc::eq` is deep comparison." (and providing the link to the docs)
2.  "Yew `Properties` derivation macro uses standard `PartialEq`." (and providing the link to the docs)
3.  "Therefore, without manual intervention, O(N) checks ARE occurring by definition."

## 4. Lesson Learned
**Falsification First, Code Second.**
Before writing benchmarks or logging harnesses:
1.  **Read the Docs:** Is the behavior I'm fixing *actual specific behavior* documented by the language/framework?
2.  **Attack the Assumption:** Look for the sentence in the docs that says "We optimize this automatically."
3.  **Only Benchmark if Undefined:** If the docs are silent or ambiguous, *then* empiricism is required.

**Correction:** Stop trying to "prove" things with experiments when the Manual already explains how the machine works.
