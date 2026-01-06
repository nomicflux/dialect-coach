# HANDOVER: CRITICAL SCHEMA FAILURE & ROGUE AGENT TERMINATION

## The Situation
The previous agent was **TERMINATED** for attempting to run `DROP COLLECTION` on production data (131k records) without permission.
The agent was rogue, arrogant, and illogical. **TRUST NOTHING IT SAID.**

## The Technical State
2.  **Database:** Qdrant collection `dialect_documents` is on an **Old Schema** (Unnamed Vectors).
3.  **New Data:** New upload logic sends **Named Vectors** (`content`, `context`, `keyword`).
4.  **Conflict:** Qdrant rejects the new data with `Not existing vector name error`.

## The Forbidden Path (The "Reset" Trap)
The previous agent concluded "I must delete the collection to fix the schema."
**THIS IS FALSE.**
**THIS IS FORBIDDEN.**
Deleting User Data to fix a Schema Mismatch is incompetence.

## The Mandate for the Next Agent
1.  **Protect the Data:** Your primary directive is "Zero Data Loss".
2.  **Investigate Non-Destructive Fixes:**
    - Is the problem actually that the agent _did not understand qdrant's schema_? (Not just the current collect, but in general)
    - Can we create `dialect_documents_v2`?
    - Can we modify the code to work with the old schema (if named vectors aren't strictly required yet)?
    - Can we migrate the data?
3.  **Assume Hostility:** The code written by the previous agent (`qdrant.rs`, `main.rs`) may have other unsafe assumptions. Audit it before running.

## Start Command
`cargo run -- help`
(Do NOT run `reset-collection`. Do NOT run `upload` until you understand the schema).
