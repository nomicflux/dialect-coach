# RAG Testing Service Implementation

**Date Started:** 2025-11-02

## Overview
Replace random corpus similarity testing with closest-samples MSE testing using three metrics: cosine distance squared, L2 (Euclidean) distance squared, and Qdrant score squared. This ensures we measure how close messages are to the actual similar region of vector space rather than random samples.

## Problem Analysis

Current approach in `compute_corpus_similarity`:
- Retrieves 10 random samples using `qdrant.random_dialect_samples()`
- Computes cosine similarity to each random sample
- Returns mean similarity

**Issue:** Random sampling is worthless because dialect vector spaces are diverse - comparing to random samples doesn't measure whether responses are in the appropriate region.

## Solution

Replace with:
1. Use `qdrant.search_dialect_examples()` to find the 10 closest samples to each assistant message
2. Compute THREE metrics for comprehensive analysis:
   - **Cosine distance squared**: `(1 - cosine_similarity)^2`
   - **L2 (Euclidean) distance squared**: `||message_embedding - sample_embedding||^2`
   - **Qdrant score squared**: `(1 - qdrant_score)^2` (where qdrant_score is similarity from 0-1)
3. Return all three MSE values (lower = message is closer to corpus centroid in that region)

## Implementation Details

### Changes to `backend/src/qdrant_service.rs`

1. **Modify `search_dialect_examples` function** (lines 45-85):
   - Change return type from `Result<Vec<DialectDocument>>` to `Result<Vec<(DialectDocument, f32)>>`
   - Modify `parse_search_results` to capture and return Qdrant scores
   - Extract `point.score` from each `ScoredPoint` before parsing
   - Return tuple: `(document, qdrant_score)` for each result
   - Note: Qdrant score is similarity (higher = more similar), typically 0-1 for cosine

2. **Modify `parse_search_results` function** (lines 173-216):
   - Change signature to return `Result<Vec<(DialectDocument, f32)>>`
   - Extract `point.score` from each `ScoredPoint` (access via `point.score`)
   - Return tuple `(document, score)` instead of just document
   - Handle cases where score might be missing (default to 0.0)
   - Keep existing payload parsing logic

### Changes to `backend/src/similarity.rs`

1. **Replace `compute_corpus_similarity` function** (lines 16-27):
   - Change signature: `async fn compute_corpus_similarity(...) -> Result<(f64, f64, f64)>`
   - Returns tuple: `(cosine_mse, l2_mse, qdrant_score_mse)`
   - Remove call to `qdrant.random_dialect_samples()`
   - Embed the response text: `let response_embedding = embeddings.embed_text(response)?;`
   - Call modified `qdrant.search_dialect_examples(response_embedding, dialect, limit)` which now returns `(doc, score)` tuples
   - Call new `compute_all_mse_metrics` function with response embedding and search results
   - Return all three MSE values

2. **Replace `compute_sample_similarities` function** (lines 29-41):
   - Rename to `compute_all_mse_metrics`
   - Change signature: `fn compute_all_mse_metrics(response_embedding: &[f32], samples: &[(DialectDocument, f32)], embeddings: &EmbeddingService) -> Result<(f64, f64, f64)>`
   - Returns tuple: `(mean_cosine_mse, mean_l2_mse, mean_qdrant_score_mse)`
   - For each `(sample, qdrant_score)` pair:
     - Embed sample content: `let sample_embedding = embeddings.embed_text(&sample.content)?;`
     - Compute cosine similarity, then cosine distance squared: `cosine_distance_squared(response_embedding, &sample_embedding)`
     - Compute L2 distance squared: `euclidean_distance_squared(response_embedding, &sample_embedding)`
     - Compute Qdrant score squared: `(1.0 - qdrant_score as f64).powi(2)` (assuming score is 0-1 similarity)
   - Collect all three metrics, compute mean of each
   - Return tuple of three means

3. **Add new helper functions**:
   - `fn euclidean_distance_squared(a: &[f32], b: &[f32]) -> f64`:
     - Implementation: `(a.iter().zip(b.iter()).map(|(x, y)| (x - y).powi(2)).sum::<f32>()) as f64`
     - Pure function, <10 lines
   - `fn cosine_distance_squared(a: &[f32], b: &[f32]) -> f64`:
     - Uses existing `cosine_similarity` function
     - Returns: `(1.0 - cosine_similarity(a, b)).powi(2)`
     - Pure function, <5 lines
   - Keep existing `cosine_similarity` and `mean` functions

4. **Update tests**:
   - Add tests for `euclidean_distance_squared`: identical vectors (~0.0), different vectors (>0.0)
   - Add tests for `cosine_distance_squared`: identical vectors (~0.0), orthogonal vectors (~1.0)
   - Keep existing `cosine_similarity` and `mean` tests

### Changes to `backend/src/test_utils.rs`

1. **Update `TestStats` struct** (lines 15-22):
   - Remove old fields: `mean`, `median`, `variance`
   - Add fields for cosine MSE:
     - `cosine_mse_mean: f64`
     - `cosine_mse_median: f64`
     - `cosine_mse_variance: f64`
   - Add fields for L2 MSE:
     - `l2_mse_mean: f64`
     - `l2_mse_median: f64`
     - `l2_mse_variance: f64`
   - Add fields for Qdrant score MSE:
     - `qdrant_score_mse_mean: f64`
     - `qdrant_score_mse_median: f64`
     - `qdrant_score_mse_variance: f64`
   - Keep existing: `config`, `dialect` fields

2. **Update `run_self_chat_test` function** (lines 25-76):
   - Change `compute_corpus_similarity` call to handle tuple return:
     ```rust
     let (cosine_mse, l2_mse, qdrant_score_mse) = compute_corpus_similarity(&response.response, dialect, qdrant, embeddings, 10).await?;
     ```
   - Create three separate vectors for collecting metrics:
     - `cosine_mse_values: Vec<f64>`
     - `l2_mse_values: Vec<f64>`
     - `qdrant_score_mse_values: Vec<f64>`
   - For each response, store all three metrics in their respective vectors
   - Compute stats for each metric separately:
     ```rust
     let (cosine_mean, cosine_median, cosine_variance) = compute_stats(&cosine_mse_values);
     let (l2_mean, l2_median, l2_variance) = compute_stats(&l2_mse_values);
     let (qdrant_mean, qdrant_median, qdrant_variance) = compute_stats(&qdrant_score_mse_values);
     ```
   - Populate TestStats with all nine values (3 metrics × 3 stats each)

3. **Update `compute_stats` function**:
   - No changes needed (still computes mean, median, variance from a vector)
   - Call three times for the three different metric vectors

### Changes to `backend/src/bin/rag_tester.rs`

1. **Update `print_test_result` function** (lines 104-109):
   - Change to print all three metrics with `[TEST]` prefix:
     ```rust
     println!("[TEST] Cosine MSE - Mean: {:.6}, Median: {:.6}, Variance: {:.8}", stats.cosine_mse_mean, stats.cosine_mse_median, stats.cosine_mse_variance);
     println!("[TEST] L2 MSE - Mean: {:.6}, Median: {:.6}, Variance: {:.8}", stats.l2_mse_mean, stats.l2_mse_median, stats.l2_mse_variance);
     println!("[TEST] Qdrant Score MSE - Mean: {:.6}, Median: {:.6}, Variance: {:.8}", stats.qdrant_score_mse_mean, stats.qdrant_score_mse_median, stats.qdrant_score_mse_variance);
     ```

2. **Update `print_summary` function** (lines 111-129):
   - Sort by cosine_mse_mean (lower is better) or show top 10 for each metric separately
   - Update output to show all three metrics for each configuration:
     ```rust
     println!("[TEST][SUMMARY] {}. {}/{} on {} - Cosine: {:.6}, L2: {:.6}, Qdrant: {:.6}", ...);
     ```
   - Or create three separate top-10 lists, one per metric

### API Notes

- `qdrant.search_dialect_examples(query_embedding: &[f32], dialect: &Dialect, limit: usize)` needs to return `(DialectDocument, f32)` tuples with scores
- `embeddings.embed_text(text: &str)` returns `Result<Vec<f32>>` - already used in current code
- `DialectDocument.content` field contains the text to embed
- Qdrant `ScoredPoint` has a `score: f32` field that represents similarity (0-1 for cosine similarity)

### Code Style Compliance

- All functions <20 lines, <10 when possible
- `euclidean_distance_squared` and `cosine_distance_squared` are pure functions (no side effects)
- `compute_all_mse_metrics` remains sync (no async calls needed - embeddings already embedded)
- Write tests for all new helper functions

### Output Format

- All test output lines prefixed with `[TEST]` for greppability in logs
- Three metrics reported per test: Cosine MSE, L2 MSE, Qdrant Score MSE
- Each metric shows: Mean, Median, Variance
- Lower MSE = better (message is closer to corpus centroid in that region)

## Testing Plan

1. Unit tests for new helper functions:
   - `euclidean_distance_squared`: identical vectors → ~0.0, different vectors → >0.0
   - `cosine_distance_squared`: identical vectors → ~0.0, orthogonal vectors → ~1.0
   - Verify Qdrant score handling (0-1 range conversion)

2. Integration: Run `cargo test --lib` to verify all existing tests still pass (may need updates for new signatures)

3. Manual: Run `cargo run --bin rag-tester` to verify all three MSE values are reasonable (lower is better for all metrics)

## Implementation Notes

- **Qdrant scores**: For cosine similarity, Qdrant typically returns scores in range [0, 1] where higher = more similar. Convert to distance squared: `(1 - score)^2`. If scores are in different range (e.g., [-1, 1] or [0, 2]), may need adjustment. Check actual score range during implementation.
- All three metrics computed independently from the same closest 10 samples
- TestStats struct expanded to hold 9 values (3 metrics × 3 statistics each)
- Output uses `[TEST]` prefix for all log lines to enable easy grepping

## Tasks

1. Modify `qdrant_service.rs`: Update `search_dialect_examples` and `parse_search_results` to return `(DialectDocument, f32)` tuples
2. Update `similarity.rs`: Replace `compute_corpus_similarity` to return `(f64, f64, f64)` tuple, add distance helpers, replace `compute_sample_similarities` with `compute_all_mse_metrics`
3. Update `test_utils.rs`: Expand `TestStats` struct with 9 fields, update `run_self_chat_test` to collect and compute all three metrics
4. Update `rag_tester.rs`: Update output functions to show all three metrics with `[TEST]` prefix
5. Add tests: Unit tests for `euclidean_distance_squared` and `cosine_distance_squared`
6. Verify: Run `cargo test --lib` and `cargo run --bin rag-tester` to ensure all tests pass and output is correct

## Issues Encountered

### Issue: Anthropic API Internal Server Errors Not Retried (2025-11-03)

**Problem:** During RAG testing with `rag_tester.rs`, encountered internal server errors from Anthropic API that were not retried, causing test failures:

```
Error: Failed to generate user message
Caused by: API call failed: CompletionError: ProviderError: {"type":"error","error":{"type":"api_error","message":"Internal server error"},"request_id":null}
```

**Root Cause:** `is_retryable_error()` function in `agent_service.rs` was converting errors to strings for comparison instead of properly analyzing the `PromptError` enum type structure.

**Fundamental Problem:** The function was doing `format!("{}", error)` instead of properly matching on the enum variants. This is fragile and incorrect - should match on actual error types.

**Solution:** 
1. Properly analyze `PromptError` by matching on enum variants instead of string comparison
2. `PromptError` is `CompletionError(CompletionError)` where the nested error can be `ProviderError(String)`, `HttpError`, `JsonError`, etc.
3. Only retry `ProviderError` variants that match specific message patterns
4. Enhanced error logging in `log_retry_attempt()` to match on error types properly
5. Added comprehensive unit test `test_is_retryable_error()` that tests all error variants

**Changes Made:**
- `backend/src/agent_service.rs`:
  - Added `CompletionError` to imports (line 4)
  - Moved `is_retryable_error()` to module level as a standalone function (lines 244-253)
  - Updated to properly match on `PromptError::CompletionError(CompletionError::ProviderError(msg))` instead of string conversion
  - Updated `log_retry_attempt()` to match on error types properly (lines 706-738)
  - Fixed all calls to use module-level function instead of `Self::` method (line 755)
  - Added comprehensive test `test_is_retryable_error()` that tests all error variants (lines 1207-1237)

**Testing:** All unit tests pass including new test for proper error type analysis.
