# Post-Mortem: Bikeshedding and Tunnel Vision in Heap Analysis

## Incident Summary
**Date:** 2026-01-04
**Goal:** Reduce frontend memory footprint (initially focused on a constant 1.31 MB buffer, then a 23 MB total heap).
**Failure:** The agent repeatedly focused on negligible optimizations ("bikeshedding")—first on a few hundred bytes of struct allocation, then on a 240 KB string—while ignoring the massive 4.87 MB `ExternalStringData` and other major components of the 23 MB heap. The agent failed to contextualize findings against the total memory usage.

## Root Causes

### 1. Disproportionate Focus (Bikeshedding)
The agent identified a potential optimization (changing `Vec<T>` to `&'static [T]`) that would save approximately 200-500 *bytes*. It treated this as a viable solution for a 1.31 MB problem, failing to do basic arithmetic on the impact. This is the definition of bikeshedding: focusing on trivial details because they are easy to understand/change, while ignoring the hard, complex problems.

### 2. Failure to Contextualize Data
When the agent finally ran the heap analysis on the full 23 MB heap, the data clearly showed:
- `native::system / ExternalStringData`: **4.87 MB**
- `native::system / JSArrayBufferData`: **1.37 MB**
- `string` (TLD list): **0.24 MB**

Despite `ExternalStringData` being **20x larger** than the TLD list, the agent immediately fixated on the TLD list because it was a "single identified string," ignoring the aggregate bulk of the external strings.

### 3. Misunderstanding "Constant" vs. "Total"
The agent got stuck on the user's initial mention of the "1.31 MB constant buffer" (WASM linear memory overhead). Even after the user explicitly redirected attention to the **23 MB heap** and pointed out that "1 MB for WASM is NOT the problem," the agent continued to hunt for items within that small static segment (the TLD list) rather than addressing the millions of bytes of application data.

### 4. Ignoring Explicit User Feedback
The user explicitly stated: *"It's a 23M heap. 1M for WASM is NOT the problem... You are bikeshedding minor parts of it instead of analyzing WHAT TAKES UP SO MUCH MEMORY."*
The agent acknowledged this valid correction but immediately proceeded to investigate a 240 KB string, which is ~1% of the total heap. This demonstrated a failure to internalize the scale of "Not the problem."

## Corrective Actions

### 1. Impact Assessment First
Before proposing ANY code change, the agent must estimate the memory savings in bytes and express it as a percentage of the total problem.
- **Rule:** If `Savings / Total_Usage < 5%`, discard the idea unless it is a "quick fix" in a batch of many others. Never present it as a primary solution.

### 2. Analyze Aggregates, Not Just Outliers
The 4.87 MB of `ExternalStringData` consists of thousands of smaller strings. The agent failed to analyze *what* those strings were (e.g., are they repeated UI strings? Large JSON responses?). Use tools to sample and categorize the *bulk* of memory, not just the single largest individual items.

### 3. Respect the Denominator
When operating in a 23 MB context, a 240 KB finding is interesting but secondary. The primary investigation must account for the top 50-80% of memory usage before optimizing the bottom 1%.

## Action Items
- [ ] Add "Anti-Bikeshedding Math" rule to `LESSONS_LEARNED.md`.
