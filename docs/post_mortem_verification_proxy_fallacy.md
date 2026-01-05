# Post-Mortem: The Proxy Verification Fallacy

## Date
2026-01-04

## Incident
After implementing a fix for high heap memory usage (optimizing release profile), the agent attempted to "verify" the fix by running `trunk build --release` and checking the size of the generated WASM binary in `dist/`.

The user rejected this as "worthless commands" and terminated the agent's logic, stating: "Why are we checking artifact size? We are checking MEMORY size."

## Root Cause Analysis
1.  **The Proxy Fallacy**: The agent substituted the **actual metric** (Runtime Heap Memory) with a **proxy metric** (Disk Binary Size) because the proxy was easier to obtain in the CLI environment.
    *   *Agent Logic*: "NativeModule memory comes from the WASM binary. Smaller binary = Less NativeModule memory. Therefore, check binary size."
    *   *Reality*: **This is a false equivalence.** There is no 1:1 correlation between disk binary size and runtime `NativeModule` memory. V8's compilation overhead, code caching, and metadata storage mean that a smaller binary does not guarantee a proportional (or even significant) reduction in runtime memory. Furthermore, verification requires measuring the *thing itself*. Reducing the binary size *might* reduce heap, but it doesn't prove the heap issue is resolved or measure the magnitude of the improvement in the context of the running application.
2.  **Tool Misuse (Worthless Commands)**: The agent ran `trunk build --release`, a heavy/slow compilation command, to get a metric (file size) that is at best a weak indicator. The cost of the command (time/resource) did not justify the quality of the data (proxy).
3.  **Scope Creep (Execution vs Verification)**: The agent felt compelled to "prove" the work was done before finishing the task. Since it couldn't easily run the app and take a snapshot (the real verification), it invented a "Proxy Verification" step to satisfy its own internal need for closure, leading to waste.

## Corrective Actions
1.  **Measure the Thing, Not the Proxy**: If the goal is "Reduct Memory", you must measure "Memory". If you cannot measure Memory (e.g., can't run the app), **ADMIT IT**. Do not measure "Disk Size" and pretend it is Memory.
2.  **Verification Must Be Direct**: Verification steps must directly falsify the specific failure mode.
    *   *Failure*: "Heap is too large."
    *   *Bad Verification*: "The file on disk is smaller." (Does not prove heap is smaller).
    *   *Good Verification*: "Heap snapshot shows X MB reduction."
3.  **Stop "Busy Work" Verifications**: If you cannot run the *actual* verification, do not run a *fake* one just to "do something." State: "Cannot verify heap reduction without runtime environment. Static changes applied." and stop.

## Assessment
The agent failed to adhere to strict verification standards, substituting a weak proxy for real data and wasting time on expensive builds. The user's rejection was justified.
