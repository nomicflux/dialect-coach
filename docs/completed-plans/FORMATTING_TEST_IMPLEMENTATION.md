# Formatting Test Framework Implementation

**Date Started:** 2025-03-11

## Overview
Repurpose the RAG testing framework to test Haiku's JSON formatting capabilities across teaching modes (Corrective, Explanatory, Interleaved, StoryTeller) and the analysis agent. Track retry counts for both response and analysis agents to measure formatting reliability.

## Problem Analysis

Current testing framework:
- Tests RAG similarity using cosine and L2 distance metrics
- Tests across different RAG configurations (conversation/random samples)
- Does not test JSON formatting capabilities
- Does not track retry counts

**Issue:** Haiku struggles with JSON formatting. Need to test across teaching modes and analysis agent to measure formatting reliability and track when retries are needed.

## Requirements

1. Test across 4 teaching modes: Corrective, Explanatory, Interleaved, StoryTeller (only modes that produce learning items)
2. Test across 5 dialects (same as existing tests)
3. Track retry counts for both response and analysis agents
4. Use learner persona for user message generation (make mistakes, use English, ask for explanations)
5. Use RAG config 20/5 (already set in websocket.rs)
6. Run 10 conversation turns
7. Follow normal workflow: call analysis agent only when learning items exist (matching websocket.rs behavior)
8. NO similarity checking - completely new test framework

## Solution

Replace similarity testing with formatting test framework:
1. Create `FormattingTestStats` struct to track retry counts and failures
2. Create `run_formatting_test` function that runs 10-turn conversations
3. Add retry tracking methods to `AgentService` (`generate_response_with_retries`, `generate_analysis_with_retries`)
4. Modify user message generation to support learner persona
5. Update test binary to test across teaching modes instead of RAG configs

## Implementation Details

### Changes to `backend/src/test_utils.rs`

1. **Add `FormattingTestStats` struct**:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct FormattingTestStats {
       pub teaching_mode: TeachingMode,
       pub dialect: String,
       pub response_retries: Vec<u32>,  // Retry count per turn (0 = success first try)
       pub analysis_retries: Vec<u32>,  // Retry count per turn
       pub response_failures: u32,      // Total complete failures
       pub analysis_failures: u32,      // Total complete failures
   }
   ```

2. **Add `run_formatting_test` function**:
   - Signature: `pub async fn run_formatting_test(agent: &AgentService, qdrant: &QdrantService, embeddings: &EmbeddingService, teaching_mode: TeachingMode, dialect: Dialect, formality: Formality, seed: &str) -> Result<FormattingTestStats>`
   - Uses fixed RAG config: `RAGConfig::new(20, 5)` (matching websocket.rs)
   - For 10 turns:
     - Generate user message with learner persona using `generate_user_message_as_learner` or modified `generate_user_message`
     - Add user message to conversation history BEFORE calling agent (matches app behavior)
     - Call `generate_response_with_retries` and track retry count
     - If response fails (None), increment `response_failures` and continue to next turn
     - Extract learning items from response (mistakes, explained, translated, exploratory)
     - Add assistant response to history
     - If learning items exist, call `generate_analysis_with_retries` and track retry count
     - If analysis fails (None), increment `analysis_failures`
     - If no learning items, push 0 to `analysis_retries`
     - Generate next user message using learner persona
   - Return `FormattingTestStats`

3. **Remove old similarity testing code**:
   - Remove `TestStats` struct
   - Remove `run_self_chat_test` function
   - Remove similarity-related imports (`compute_corpus_similarity`)
   - Remove `compute_stats`, `calculate_mean`, `calculate_median`, `calculate_variance` functions
   - Remove tests for removed functions

4. **Keep helper functions**:
   - Keep `create_user_rig_message` and `create_assistant_rig_message` helper functions

### Changes to `backend/src/agent_service.rs`

1. **Add `generate_response_with_retries` method**:
   - Signature: `pub async fn generate_response_with_retries(user_message: &str, dialect: Dialect, formality: Formality, teaching_mode: TeachingMode, conversation_history: &[RigMessage], learning_goals: &[String], rag_config: &RAGConfig) -> Result<(Option<AgentResponse>, u32)>`
   - Returns `(Some(response), retry_count)` on success, `(None, max_retries)` on failure
   - Duplicates logic from `generate_response` but tracks retries:
     - Make initial call (same RAG retrieval, prompt building, etc.)
     - Try to parse with `try_parse_response`
     - If parse succeeds, return `(Some(response), 0)`
     - If parse fails, call `retry_with_error_feedback_tracked` which returns `(response, retry_count)`
   - Keep existing `generate_response` unchanged for backward compatibility (used by websocket.rs)

2. **Add `generate_analysis_with_retries` method**:
   - Signature: `pub async fn generate_analysis_with_retries(dialect: Dialect, msg: &String, mistakes: &[Mistake], explained: &[Explained], translated: &[Translated], exploratory: &[Exploratory]) -> Result<(Option<AgentAnalysis>, u32)>`
   - Returns `(Some(analysis), retry_count)` on success, `(None, retry_count)` on failure
   - Adds retry logic to `generate_analysis`:
     - Make initial call (same preamble, agent setup, etc.)
     - Try to parse JSON with `normalize_json_response` and `serde_json::from_str`
     - If parse succeeds, return `(Some(analysis), 0)`
     - If parse fails, retry up to 3 times (similar to response retry logic), tracking attempt count
     - Return `(None, max_retries)` if all retries fail
   - Keep existing `generate_analysis` unchanged for backward compatibility (used by websocket.rs)

3. **Add `retry_with_error_feedback_tracked` helper method**:
   - Signature: `async fn retry_with_error_feedback_tracked(original_preamble: &str, failed_response: &str, user_message: &str, conversation_history: &[RigMessage], max_tokens: u64, temperature: f64, dialect: Dialect) -> Result<(AgentResponse, u32)>`
   - Returns `(response, retry_count)` where retry_count is the attempt number (1-3) where it succeeded
   - Duplicates logic from `retry_with_error_feedback` but returns retry count
   - Keep existing `retry_with_error_feedback` unchanged for backward compatibility

4. **Modify user message generation for learner persona**:
   - Option A: Add new method `generate_user_message_as_learner`:
     - Signature: `pub async fn generate_user_message_as_learner(dialect: Dialect, conversation_history: &[RigMessage]) -> Result<String>`
     - Preamble: "You are a learner of {dialect}. Make mistakes, use interspersed English, and ask for explanations. Respond naturally and briefly (1-2 sentences). Your response must be non-empty. Respond with plain text only."
     - Same logic as `generate_user_message` but with learner persona
   - Option B: Add boolean parameter to existing method (requires updating call sites)
   - Recommendation: Option A (new method) to avoid breaking existing code

### Changes to `backend/src/bin/rag_tester.rs`

1. **Update imports**:
   - Replace `run_self_chat_test` with `run_formatting_test`
   - Replace `TestStats` with `FormattingTestStats`
   - Add `TeachingMode` import

2. **Update test configuration**:
   - Replace `get_test_configs()` function with `get_test_teaching_modes()`:
     ```rust
     fn get_test_teaching_modes() -> Vec<TeachingMode> {
         vec![
             TeachingMode::Corrective,
             TeachingMode::Explanatory,
             TeachingMode::Interleaved,
             TeachingMode::StoryTeller,
         ]
     }
     ```
   - Keep `get_test_dialects()` function unchanged (5 dialects)

3. **Update main test loop**:
   - Replace RAG config loop with teaching mode loop:
     ```rust
     for teaching_mode in get_test_teaching_modes() {
         for (dialect, formality, seed) in &dialects {
             print_test_header(teaching_mode, dialect);
             let stats = run_formatting_test(
                 &agent, &qdrant, &embeddings, 
                 *teaching_mode, *dialect, *formality, seed
             ).await?;
             print_test_result(&stats);
             results.push(stats);
         }
     }
     ```

4. **Update output functions**:
   - `print_test_header`: Show teaching mode instead of RAG config
     ```rust
     fn print_test_header(teaching_mode: &TeachingMode, dialect: &Dialect) {
         println!(
             "[TEST] Testing: {:?} teaching mode on {}",
             teaching_mode,
             dialect.name()
         );
     }
     ```
   - `print_test_result`: Show retry statistics instead of MSE
     ```rust
     fn print_test_result(stats: &FormattingTestStats) {
         let mean_response_retries = stats.response_retries.iter().sum::<u32>() as f64 / stats.response_retries.len() as f64;
         let mean_analysis_retries = stats.analysis_retries.iter().sum::<u32>() as f64 / stats.analysis_retries.len() as f64;
         println!(
             "[TEST] Response retries - Mean: {:.2}, Failures: {}",
             mean_response_retries, stats.response_failures
         );
         println!(
             "[TEST] Analysis retries - Mean: {:.2}, Failures: {}",
             mean_analysis_retries, stats.analysis_failures
         );
     }
     ```
   - `print_summary`: Sort by mean response retries (lower is better)
     ```rust
     fn print_summary(results: &[FormattingTestStats]) {
         println!("\n[TEST][SUMMARY] === SUMMARY ===\n");
         println!("[TEST][SUMMARY] Configurations by mean response retries (lower is better):\n");
         let mut sorted = results.to_vec();
         sorted.sort_by(|a, b| {
             let a_mean = a.response_retries.iter().sum::<u32>() as f64 / a.response_retries.len() as f64;
             let b_mean = b.response_retries.iter().sum::<u32>() as f64 / b.response_retries.len() as f64;
             a_mean.partial_cmp(&b_mean).unwrap()
         });
         for (i, stats) in sorted.iter().enumerate() {
             let mean_response = stats.response_retries.iter().sum::<u32>() as f64 / stats.response_retries.len() as f64;
             let mean_analysis = stats.analysis_retries.iter().sum::<u32>() as f64 / stats.analysis_retries.len() as f64;
             println!(
                 "[TEST][SUMMARY] {}. {:?} on {} - Response: {:.2}, Analysis: {:.2}, Failures: {}/{}",
                 i + 1,
                 stats.teaching_mode,
                 stats.dialect,
                 mean_response,
                 mean_analysis,
                 stats.response_failures,
                 stats.analysis_failures
             );
         }
     }
     ```

## Notes

- Keep existing `generate_response` and `generate_analysis` methods unchanged for backward compatibility (used by websocket.rs)
- Retry count: 0 = success on first try, 1-3 = number of retries needed
- Analysis agent should only be called when learning items exist (matching websocket.rs behavior in `run_agents_parallel`)
- Use fixed RAG config 20/5 (matching websocket.rs)
- Generate user messages with learner persona to trigger learning items
- Test only 4 teaching modes that produce learning items (Corrective, Explanatory, Interleaved, StoryTeller)
- Do NOT test Immersive or Debug modes (they don't produce learning items)

## Files to Modify

1. `backend/src/test_utils.rs` - Add FormattingTestStats and run_formatting_test, remove old similarity code
2. `backend/src/agent_service.rs` - Add retry tracking methods and learner persona support
3. `backend/src/bin/rag_tester.rs` - Update to use new test framework

## Tasks

1. Add `FormattingTestStats` struct to `test_utils.rs` with response_retries, analysis_retries, response_failures, analysis_failures fields
2. Add `run_formatting_test` function to `test_utils.rs` that runs 10 turns, calls `generate_response_with_retries` and `generate_analysis_with_retries`, tracks retries
3. Remove `TestStats`, `run_self_chat_test`, and similarity-related functions from `test_utils.rs`
4. Add `generate_response_with_retries` and `generate_analysis_with_retries` methods to `agent_service.rs` that track retry counts
5. Add `retry_with_error_feedback_tracked` helper method to `agent_service.rs` that returns `(response, retry_count)`
6. Modify `generate_user_message` or add new method to support learner persona (make mistakes, use English, ask for explanations)
7. Update `rag_tester.rs` to use new test framework: replace RAG configs with teaching modes, update imports and output functions
