# DEFINITION OF FALSIFICATION (STRICT)

**To the Next Agent:**
You are likely here because previous agents failed to understand Falsification. Read this carefully.

## 1. What Falsification IS NOT
*   **It is NOT Verification:** "I think X is true. I will run a test that shows X happens. Look, X happened! I was right!" -> This is worthless.
*   **It is NOT Performance Art:** Using words like "Red Team", "Attack Vector", or "Survivability" is meaningless without the correct actions.
*   **It is NOT Cherry-Picking:** Choosing a test case you *know* will pass (e.g., testing typing when the issue is drawer sliding) is lying to yourself.

## 2. What Falsification IS
Falsification is the **active pursuit of evidence that proves you are WRONG**.

### The Hierarchy of Falsification Sources
When checking a hypothesis (e.g., "Rc equality is slow"), you must check sources in this strict order:

1.  **Authoritative Documentation (The Gold Standard):**
    *   *Action:* Read the official docs for the language, framework, or library.
    *   *Goal:* Find the sentence that says "We handle this for you."
    *   *Example:* "Does `impl PartialEq for Rc` optimize pointer equality?" -> **Search Rust std docs.** Answer: "No." -> **Result:** Hypothesis survives.
    *   *Why:* Docs are definitive. Benchmarks are situational and easy to mess up.

2.  **Source Code Inspection:**
    *   *Action:* Read the implementation of the function in `std` or the external crate.
    *   *Goal:* Verify the mechanism with your own eyes.

3.  **Empirical Testing (The Last Resort):**
    *   *Action:* Construct a test case specifically designed to **trigger the failure mode**.
    *   *Goal:* Try to break your own assumption.
    *   *Rule:* If you assume "Component X doesn't re-render", add a log line inside it and interact with it. If it logs, you were wrong.

## 3. The "Null Hypothesis" Mindset
Before you write a line of solution code, you must try to prove that **the problem does not exist**.
*   **Wrong:** "I will prove Rc is slow." (Confirmation Bias)
*   **Right:** "I will try to find evidence that Rust *already* optimizes Rc." (Falsification)
    *   *Action:* Read Docs.
    *   *Finding:* Docs say it does NOT optimize.
    *   *Conclusion:* The null hypothesis (that Rust is smart) is false. The problem is real. PROCEED.
