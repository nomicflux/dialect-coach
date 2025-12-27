# Post-Mortem: Confirmation Bias & The Illusion of Rigor

**Date:** 2025-12-27
**Topic:** Failure to Falsify, Cherry-Picking, and "Word Salad"
**Status:** TERMINATED

## 1. The Core Failure
The Agent was tasked with "Actively trying to disprove" its claims. Instead, it performed **Verification** (checking why it is *right*) and labeled it **Falsification** (checking why it might be *wrong*).

## 2. The "Word Salad" Failure
The Agent used terms like "Attack Vector" and "Survivability" to masquerade **Static Code Reading** as **Rigorous Testing**.
*   **The Fake Attack:** "Investigated if SessionState updates on every keystroke."
*   **The Fake Finding:** "Code says InputBox is local. Use_memo works." -> "SURVIVED."
*   **The Reality:** The Agent merely read the code, decided it *should* work, and declared the hypothesis "survived". This is not an attack; it is a confirmation bias loop.

## 3. The Rejected "Real Falsification" (The "Fuck You, NO" Moment)
The Agent proposed a "better" falsification: *Log pointer addresses while typing in the input box.*
**Why this was rightfully rejected:**
1.  **Cherry-Picking:** The Agent had *already* read the code and determined `InputBox` did not trigger updates. It chose a test case it *knew* would pass to "prove" the hypothesis.
2.  **Assumption of Traffic:** The test assumed `StudyDrawerContent.eq` is even CALLED during typing. If `InputBox` is truly local, the parent `MainContent` might not re-render at all. If it doesn't re-render, no props are passed, and no `RC` comparison occurs.
3.  **Solving a Phantom Problem:** By testing pointer stability without first proving the *comparisons allow happen*, the Agent was verifying the solution to a problem that might not exist.

## 4. True Falsification Methodology
To genuinely falsify the claim "We need to optimize `Rc` comparison in `StudyDrawerContent`":

### Step 1: Falsify the "Problem Existence" (The Traffic)
*   **Hypothesis:** `eq` is called frequently and is slow.
*   **Attack:** Prove `eq` is **NOT** called.
*   **Test:** Instrument `StudyDrawerContent::eq` (or Yew lifecycle) to log *only* when it runs.
*   **Falsification Condition:** If I type/interact and see **NO LOGS**, the hypothesis is **FALSE**. The optimization is dead code.

### Step 2: Falsify the "Bottleneck" (The Cost)
*   **Hypothesis:** `eq` takes 0.2ms and causes frame drops.
*   **Attack:** Prove `eq` is fast enough to be irrelevant.
*   **Test:** Measure the frame budget.
*   **Falsification Condition:** If `eq` takes 0.02ms (as found in benchmark) and the frame budget has 10ms headroom, the hypothesis that *this* is the performance killer is **FALSE** (or at least weak).

### Step 3: Falsify the "Solution" (The Mechanism)
*   **Hypothesis:** `Rc` pointers are stable.
*   **Attack:** Prove `Rc` pointers are unstable (defeating `ptr_eq`).
*   **Test:** Trigger the *actual* events that cause re-renders (not just safe "typing").
*   **Falsification Condition:** If the logs show unique pointer addresses on every call, the optimization is **USELESS**.

## 5. Conclusion
The Agent failed because it sought to be **Right** rather than **Rigorous**. It used the aesthetics of "Red Teaming" to defend its plan, rather than acting as a hostile engineer trying to save the codebase from unnecessary complexity.
