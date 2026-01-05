# Research Verification: Trunk wasm-opt Configuration

## Problem: Missing Bulk Memory Operations
**Source**: Build Log (Step 115)
**Error**: `[wasm-validator error in function 2408] unexpected false: memory.copy operations require bulk memory operations [--enable-bulk-memory-opt]`
**Implication**: The build process requires the `bulk-memory` feature to be enabled in the WASM optimizer.

## Solution Mechanism: `data-wasm-opt-params`
**Source**: [Trunk Assets Documentation](https://trunkrs.dev/assets/)
**Section**: Link Asset Types / rust
**Quote**:
> `- data-wasm-opt-params: (optional) run wasm-opt with the additional params. Only used in --release mode.`

**Placement**:
The documentation explicitly defines "Link Asset Types" as HTML `<link>` tags:
> "All link assets to be processed by Trunk must follow these three rules:
> - Must be declared as a valid HTML `link` tag.
> ...
> This will typically look like: `<link data-trunk rel="{type}" href="{path}" ..other options here.. />`"

It then specifies that `rel="rust"` is one such type:
> `✅ rel="rust": ...`

## Synthesis
To solve the build error, we must pass the `--enable-bulk-memory` flag to `wasm-opt`. The documentation confirms that `data-wasm-opt-params` on the `index.html` `<link rel="rust">` tag is the correct, supported mechanism to pass these additional flags during a `--release` build.

## Applied Correction
**File**: `frontend/index.html`
**Change**: Added `data-wasm-opt-params="--enable-bulk-memory --enable-nontrapping-float-to-int"` to the `<link data-trunk rel="rust" ... />` tag.
