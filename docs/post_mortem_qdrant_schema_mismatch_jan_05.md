# Post Mortem: Silent Qdrant Upload Failure (Schema Mismatch)

## Incident Description
The `corpus-processor` reported successful upload of 4070 records ("✅ Upload complete"), but no records appeared in Qdrant. The root cause was a **schema mismatch**:
- **Existing Collection:** Created with single unnamed vector schema.
- **New Data:** Upserted with named vectors (`content`, `context`, `keyword`).

## Root Cause
1. **No Schema Validation:** `init_collection` checked if the collection existed but **did not validate its schema**.
   ```rust
   if exists {
       println!("Collection '{}' already exists...", COLLECTION_NAME);
       // ❌ Failed to check if existing collection supports named vectors
   }
   ```
2. **Silent Failure in Upsert:** The Qdrant client (or server) likely rejected the malformed points (named vectors into unnamed collection) but returned a standard `200 OK` or the error was swallowed/misinterpreted by the client library, leading the CLI to report success.

## Failure to Anticipate
When the user asked about "new fields (tags, new embeddings)", I focused on **idempotency** (can we re-upload?) but missed the **migration** aspect (can we upload *new schema* to *old collection*?).

**Critical Oversight:** I authorized a workflow that mixed schema versions without a migration strategy, leading to silent data loss (failure to persist).

## Corrective Actions
1. **Implement Schema Validation:** `init_collection` must verify that the existing collection supports the required named vectors. If not, it must **FAIL FAST** or prompt for migration.
2. **Add `reset-collection`:** Explicit command to drop and recreate the collection with the correct schema.
3. **Verify Upsert Response:** Investigate why `upsert_points` returned `Ok` despite the operation failing to store data.

## Lesson Learned
**"Idempotency" implies schema compatibility.** You cannot be idempotent if the underlying data structure has changed. Always validate the target schema before attempting to sync data, especially when adding new vector fields.
