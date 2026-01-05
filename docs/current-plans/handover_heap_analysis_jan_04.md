# Handover: Heap Analysis & Memory Reduction

## Objective
Reduce the frontend memory footprint using `Heap-20260104T201929.heapsnapshot` (Baseline) and `Heap-2.heapsnapshot` (Current) as data sources.

## System State (What changed between Snapshot 1 and 2)
1.  **Cargo.toml**: Added `[profile.release]` with `opt-level="z"`, `lto=true`.
2.  **frontend/index.html**: Added `data-wasm-opt-params="--enable-bulk-memory"` to `rel="rust"`.
3.  **frontend/Cargo.toml**: Moved `wasm-logger` and `console_error_panic_hook` to `[target.'cfg(debug_assertions)'.dependencies]`.

## The Problem
Despite these changes, `Heap-2.heapsnapshot` shows a **regression** (Total nodes increased from ~220k to ~243k).

## Raw Data (from investigate_heap.py)
**Baseline (Heap-1)**:
- `native::system / JSArrayBufferData`: 1.31 MB
- `hidden::system / Managed<wasm::NativeModule>`: 2.12 MB

**Current (Heap-2)**:
- `native::system / JSArrayBufferData`: 1.31 MB (Constant)
- Top Strings include external contamination (e.g., `Sur la page Options de RoboForm`, `web-client-content-script`).

## The Failure Pattern (to avoid)
The previous attempt failed because it:
1.  **Chased Proxies**: Optimized for "Binary Disk Size" (via `trunk build`) assuming it 1:1 correlated with Heap. It does not.
2.  **Deflected**: Blamed the "RoboForm" strings for the entire regression without proving it. (User: also, the point is to make the APP more performant. We control the app, we do not control extensions.)
3.  **Ignored Core Data**: The 1.31 MB buffer remained constant, yet was ignored in favor of tweaking build flags.

## Your Task
1.  **Ignore Build Config**: The build is fully optimized. Do not tweak `Trunk.toml` or `Cargo.toml` further.
2.  **Analyze the 1.31 MB Buffer**: Identify *what* this buffer is. It is the largest single constant block.
3.  **Filter the Noise**: You will see "RoboForm" strings. Acknowledge them as noise, but **find the Application Memory** underneath.
4.  **Produce Actionable Reductions**: Find a specific code change (e.g., "Reduce struct size", "Drop unused Vec") that targets the constant application memory.
