# Implementation Plan: Fix Missing Qdrant Methods

## Context
After the collection was renamed to `dialect_documents_v2` to avoid schema conflicts, two methods are missing from `QdrantService`:
1. `update_points` - Called in main.rs:334 to update dialect labels
2. `delete_collection` - Called in main.rs:361 for ResetCollection command

Additionally:
- `upload_enriched_documents` is missing `.wait(true)` for consistency (post-mortem identified this)
- Unused imports need cleanup
- Warning message in main.rs:351 still references old collection name

## Files to Modify
- `corpus-processor/src/qdrant.rs` - Add missing methods, fix `.wait(true)`, remove unused imports
- `corpus-processor/src/main.rs` - Update warning message to reference correct collection name

## Research Findings

### API: `update_points_batch`
From [Qdrant Rust Client Docs](https://docs.rs/qdrant-client/latest/qdrant_client/struct.Qdrant.html):
- Method: `client.update_points_batch(UpdateBatchPointsBuilder::new(collection, operations))`
- Uses `PointsUpdateOperation` with `Operation::SetPayload`
- Supports `.wait(true)` to ensure operation commits before returning

### API: `delete_collection`
From [Qdrant Rust Client Docs](https://docs.rs/qdrant-client/latest/qdrant_client/struct.Qdrant.html):
- Method: `client.delete_collection(collection_name).await?`
- Simple method, takes collection name as string

---

## Phase 1: Implement `update_points` Method

**Subagent**: kiss-code-generator (single file, well-defined task)

### Code Style Checklist
- [ ] Functions <20 lines (if possible, <10 lines)
- [ ] Helper functions for complex logic
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No dead code

### Deliverables
- Working `update_points` method in `QdrantService` impl block
- Method signature: `pub async fn update_points(&self, dialect_from: &String, dialect_to: &String) -> Result<()>`
- Uses `update_points_batch` internally with `SetPayload` operation

### Implementation Steps
1. Add method before the closing `}` of the impl block (before line 352)
2. Filter points by `dialect == dialect_from` using `scroll` (similar to `delete_points` pattern)
3. Build `SetPayload` operation with new dialect value
4. Call `client.update_points_batch()` with `.wait(true)`
5. Print progress messages (finding points, updating, success)

### Expected Code Structure
```rust
pub async fn update_points(&self, dialect_from: &String, dialect_to: &String) -> Result<()> {
    // 1. Print start message
    // 2. Scroll to find points with filter on dialect_from
    // 3. If no points found, print warning and return Ok
    // 4. Build SetPayload with new dialect value
    // 5. Create PointsUpdateOperation
    // 6. Call update_points_batch with .wait(true)
    // 7. Print success message
    Ok(())
}
```

### Phase End Checklist
- [ ] Run `cargo check` - must pass
- [ ] Run `cargo clippy` - must pass with zero warnings
- [ ] No dead code
- [ ] Git add and commit: `git commit -m "Phase 1 (add update_points method) complete"`

---

## Phase 2: Implement `delete_collection` Method

**Subagent**: kiss-code-generator (single file, simple method)

### Code Style Checklist
- [ ] Functions <20 lines (if possible, <10 lines)
- [ ] Helper functions for complex logic
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No dead code

### Deliverables
- Working `delete_collection` method in `QdrantService` impl block
- Method signature: `pub async fn delete_collection(&self) -> Result<()>`
- Uses `client.delete_collection()` API

### Implementation Steps
1. Add method before the closing `}` of the impl block (after `update_points`)
2. Print message indicating deletion
3. Call `client.delete_collection(COLLECTION_NAME).await?`
4. Print success message

### Expected Code Structure
```rust
pub async fn delete_collection(&self) -> Result<()> {
    // 1. Print deletion message with COLLECTION_NAME
    // 2. Call client.delete_collection(COLLECTION_NAME).await?
    // 3. Print success message
    Ok(())
}
```

### Phase End Checklist
- [ ] Run `cargo check` - must pass
- [ ] Run `cargo clippy` - must pass with zero warnings
- [ ] No dead code
- [ ] Git add and commit: `git commit -m "Phase 2 (add delete_collection method) complete"`

---

## Phase 3: Add `.wait(true)` and Fix Warning Message

**Subagent**: kiss-code-generator (two small fixes in separate files)

### Code Style Checklist
- [ ] Functions <20 lines
- [ ] No dead code

### Deliverables
- `upload_enriched_documents` has `.wait(true)` on line 165
- Warning message in main.rs:351 references `dialect_documents_v2` (or uses constant)

### Implementation Steps

#### Fix 1: Add `.wait(true)` to upload_enriched_documents
1. Locate line 165 in qdrant.rs: `.upsert_points(UpsertPointsBuilder::new(COLLECTION_NAME, points))`
2. Change to: `.upsert_points(UpsertPointsBuilder::new(COLLECTION_NAME, points).wait(true))`

#### Fix 2: Update warning message
1. Locate line 351 in main.rs: `print!("⚠️  WARNING: This will DELETE existing collection 'dialect_documents'. Type 'yes' to confirm: ");`
2. Change to: `print!("⚠️  WARNING: This will DELETE existing collection 'dialect_documents_v2'. Type 'yes' to confirm: ");`

### Phase End Checklist
- [ ] Run `cargo check` - must pass
- [ ] Run `cargo clippy` - must pass with zero warnings
- [ ] No dead code
- [ ] Git add and commit: `git commit -m "Phase 3 (add .wait(true) and fix warning) complete"`

---

## Phase 4: Remove Unused Imports

**Subagent**: kiss-code-generator (cleanup task)

### Code Style Checklist
- [ ] No dead code
- [ ] No unused imports

### Deliverables
- All unused imports removed from qdrant.rs
- `cargo clippy` produces zero warnings

### Implementation Steps
1. Run `cargo clippy --fix --lib -p corpus-processor --allow-dirty` to auto-fix unused imports
2. Verify the auto-fixes are correct
3. If any imports are still unused, remove them manually

### Current Unused Imports (from compilation output)
- Line 1: `anyhow` (from `use anyhow::{anyhow, Context, Result}`)
- Line 5: `Operation`, `SetPayload` (should NOT be unused after Phase 1)
- Line 6: `qdrant_client::qdrant::value::Kind`
- Line 9-10: `PointsSelector`, `PointsUpdateOperation`, `UpdateBatchPointsBuilder`, `Value`
  (Should NOT be unused after Phase 1)

**IMPORTANT**: After Phase 1 and 2, `Operation`, `SetPayload`, `PointsSelector`, `PointsUpdateOperation`, and `UpdateBatchPointsBuilder` will be USED. Only remove imports that are ACTUALLY unused after implementation.

### Phase End Checklist
- [ ] Run `cargo check` - must pass
- [ ] Run `cargo clippy` - must pass with ZERO warnings
- [ ] No dead code
- [ ] Git add and commit: `git commit -m "Phase 4 (remove unused imports) complete"`

---

## Phase 5: Final Verification

**Orchestrator responsibility** (not delegated to subagent)

### Deliverables
- All code compiles without errors or warnings
- All functionality is implemented and working
- Documentation updated

### Verification Steps
1. Run `cargo check` - must pass
2. Run `cargo test` - must pass 100%
3. Run `cargo clippy` - must produce ZERO warnings
4. Verify no dead code exists
5. Test compilation of all workspace crates: `cd .. && cargo check`

### Phase End
- Update this status document with completion timestamp
- NO git commit (this is verification only)

---

## Sources
- [Qdrant Rust Client - Batch Update Points](https://api.qdrant.tech/v-1-13-x/api-reference/points/batch-update)
- [Qdrant Rust Client Docs](https://docs.rs/qdrant-client/latest/qdrant_client/struct.Qdrant.html)
- [Delete Collection API](https://api.qdrant.tech/api-reference/collections/delete-collection)
