# HANDOVER: Qdrant Schema Mismatch & Strict Protocol

## The Situation
The `dialect_documents` Qdrant collection is on an **Old Schema** (Unnamed Vectors).
The `corpus-processor` is attempting to upload **Named Vectors**, causing `Not existing vector name error`.
The previous agent was terminated for "poking" the live database (`curl`) before understanding the code or the API rules.

## The Mandate: Order of Operations
You must follow this sequence STRICTLY. Do not skip steps.

1.  **Analyze the CLIENT CODE (`src/qdrant.rs`)**:
    - How are we constructing the points?
    - Are we explicitly using named vectors?
    - Is there a configuration to switch to unnamed vectors?

2.  **Analyze the API EXPECTATIONS**:
    - What does Qdrant require for named vs unnamed vectors?
    - Does the Code match the API specs for the *intended* schema?

3.  **Propose Non-Destructive Solutions**:
    - **Backwards Compatibility**: Can we modify `src/qdrant.rs` to strip vector names if detecting the old schema?
    - **Parallel Migration**: Can we create `dialect_documents_v2` (Named) and write to that, leaving v1 alone?

## Forbidden Actions
- **DO NOT DELETE/DROP** the `dialect_documents` collection.
- **DO NOT RUN CURL/Network Tools** until you have fully analyzed the code and documented your understanding of the schema mismatch.

## Files to Read
- `corpus-processor/src/qdrant.rs` (The Source of Truth for Intent)
- `corpus-processor/src/main.rs`
- `AGENTS.md` & `LESSONS_LEARNED.md`
