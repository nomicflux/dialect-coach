# Post-Mortem: Docker Build Diagnosis Failure

**Date:** 2025-12-19
**Agent:** Antigravity
**Incident:** Failed diagnosis of "No space left on device" during `docker buildx bake`.

## Failure Analysis

The agent failed to correctly identify the root cause of the Docker build failure and erroneously suggested "hacks" and ineffective maintenance commands.

### 1. Unjustified Assumptions & "Solutionering"
The agent assumed that compiling `trunk` from source was "too heavy" and constituted a "wrong" configuration, labeling it effectively as a problem to be "fixed" by switching to pre-built binaries. This directly contradicted the user's statement that this configuration had worked previously.

**Lesson:** Valid, standard Rust build practices (compiling from source) are not "bugs" to be optimized away unless performance is the *specific* complaint. Space exhaustion is an environmental constraint, not necessarily a code defect.

### 2. Lack of Verification
The agent hypothesized that the `docker buildx` cache was full and separate from `docker system prune`, leading to the error. While theoretically possible, the agent proposed `docker buildx prune --all` as the solution without first verifying if such cache actually existed or was significant. The command returned `0B`, proving the hypothesis instantly false and damaging user trust.

**Lesson:** Before prescribing destructive or maintenance commands (like pruning), verify the state. If the agent had checked `docker system df` or similar, it would have seen the cache was empty.

### 3. Persistent Refusal to Listen (The "Phantom" Error)
The user explicitly stated that **disk space is not actually constrained** and that previous versions with *larger* context (3GB+) worked fine.
**Critical Failure:** The agent persistently treated the "No space left on device" error as a *resource management* problem (needs pruning, needs serialization, needs smaller binaries) rather than a *configuration* problem causing a false positive. The agent failed to internalize that the error was a "phantom" result of a bad setup, not physical reality.

## Corrective Actions (For Future Agents)

1.  **Assume Abundance:** Proceed with the axiom that **disk space is abundant**. Any error saying otherwise is a lie told by the configuration.
2.  **Find the Config Bug:** The goal is to find the specific Docker/Buildx configuration (e.g., driver options, limits, mount settings) that is artificially constraining the build or misreporting the status.
3.  **Do Not Optimize:** Do not try to save space. Do not prune. Do not serialize. Fix the broken configuration that thinks it's full.
