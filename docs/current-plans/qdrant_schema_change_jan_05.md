# Qdrant Schema Change: Collection Name and Vector Structure

## Collection Name Change
- **Old**: `dialect_documents`
- **New**: `dialect_documents_v2`

## Schema Change: Vector Structure

### Old Schema (dialect_documents)
**Single unnamed vector:**
- One vector per point (no name)
- Vector contains the content embedding

### New Schema (dialect_documents_v2)
**Named vectors with multiple embeddings:**
- `content` - The main content embedding (required)
- `context` - Context/trigger embedding (optional)
- `keyword` - Keyword embedding (optional)

## Technical Details

### Collection Creation Code
The new collection is created with `VectorParamsMap` containing three named vector configurations:

```rust
let mut vectors_config = VectorParamsMap::new();
vectors_config.insert(
    "content".to_string(),
    VectorParamsBuilder::new(vector_size, Distance::Cosine).build(),
);
vectors_config.insert(
    "context".to_string(),
    VectorParamsBuilder::new(vector_size, Distance::Cosine).build(),
);
vectors_config.insert(
    "keyword".to_string(),
    VectorParamsBuilder::new(vector_size, Distance::Cosine).build(),
);
```

### Point Structure
Each point in `dialect_documents_v2` has:
- **Payload fields**: `content`, `dialect`, `formality`, `intent`, `emotion`, `topics`, `context_triggers`, `keywords`
- **Named vectors**: `content` (always present), `context` (if available), `keyword` (if available)

### Data State
- **Old collection (`dialect_documents`)**: Contains ~131k records with unnamed vectors - **UNCHANGED**
- **New collection (`dialect_documents_v2`)**: Empty or populated with new uploads using named vectors

## Constant Definition
```rust
const COLLECTION_NAME: &str = "dialect_documents_v2";
```
This is defined in `corpus-processor/src/qdrant.rs` line 17.
