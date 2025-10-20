# Qdrant API Cassettes

This directory contains recorded HTTP responses from Qdrant API calls for deterministic testing.

## Files

### `collections_list.json`
Response from `GET /collections` - Lists available collections in the Qdrant instance.

### `collection_info_dialect_documents.json`
Response from `GET /collections/dialect_documents` - Information about the dialect_documents collection.

### `count_points_dialect_documents_nofilter.json`
Response from `POST /collections/dialect_documents/points/count` with no filter - Count of points in collection.

## Format

Each file contains only the JSON response body, with no HTTP headers or authentication data.

Example format:
```json
{
  "result": {
    "vectors_count": 1500,
    "points_count": 1500,
    "status": "green"
  },
  "status": "ok",
  "time": 0.001456
}
```

## Regenerating Cassettes

To update these files with fresh data from a live Qdrant instance:

```bash
# Set environment variables
export QDRANT_URL=https://your-instance.cloud.qdrant.io:6334
export QDRANT_API_KEY=your_qdrant_key
export QDRANT_TEST_COLLECTION=dialect_documents

# Run the recorder (not executed in CI)
cargo run --bin record_qdrant_cassettes
```

## Security

- **No secrets**: These files contain only response bodies with no authentication headers
- **No sensitive data**: Only collection metadata, not actual document content
- **Safe to commit**: All files in this directory should be committed to git for CI determinism

## Testing

Tests use these cassettes via a mock HTTP server to provide deterministic, offline behavior:

```bash
# Run tests offline (no network calls)
cargo test -p corpus-processor

# Tests work without QDRANT_* environment variables
unset QDRANT_URL QDRANT_API_KEY
cargo test -p corpus-processor  # Still passes
```