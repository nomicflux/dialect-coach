# Corpus Processor Recovery Log

**Date Started**: 2025-10-18T19:47:05Z  
**Objective**: Implement comprehensive unit and integration testing for corpus-processor crate

## Goals

- **Full test coverage**: Minimum 90% line coverage on corpus-processor crate
- **Systematic workflow**: Write complete suite → run full suite once → capture all errors verbatim → propose fixes based only on those errors → implement → rerun full suite → repeat
- **No shortcuts**: Never run individual tests during development; always run the full suite

## Non-Negotiables

1. **RAG-only, ABSOLUTELY NO TRAINING**: This tool prepares RAG documents for Qdrant upload. It does NOT train AI models. ANY mention of training concepts (train/, training, model training, etc.) in code, comments, documentation, or directory names is STRICTLY PROHIBITED.
2. **Never create train/ directories or use training terminology**: Any code path creating a train/ directory or using training concepts is a critical bug.
3. **Canonical corpus input directory**: `corpus-data` (hyphen, not underscore)
4. **Preserve downloads**: Never delete anything in `corpus-downloads/`; copy/sync into `corpus-data/` for processing
5. **No random changes**: Only make changes based on specific test failures
6. **No individual test runs**: Always run the complete test suite

## Directory Policy (Authoritative)

### Input Directory Rules
- **Canonical input directory**: `corpus-data/` (hyphen, not underscore)
- **Default behavior**: When no `--input` specified, default to `./corpus-data/`
- **Legacy directories that must NOT be used by default**:
  - `corpus-downloads/` (preserved but rejected as input with helpful error)
  - Any directory containing `train/` or ANY training-related terminology (explicitly rejected)
  - `corpus_data/` (underscore version - redirect to hyphen version)

### Output Directory Rules  
- **Default output**: Change from generic `output/` to follow existing pattern: `corpus-data/{language}/{dialect}/processed/`
- **Never create**: Any directory named or containing `train/` or ANY training terminology
- **Structure**: Files follow existing pattern - dialect-specific directories with processed/ subdirs
- **Examples**: 
  - `corpus-data/arabic/egyptian/processed/arabic_egyptian.jsonl`
  - `corpus-data/french/african/processed/documents.jsonl`
  - Matches existing: `corpus-data/argentinian/processed/`, `corpus-data/colombian/processed/`

### Migration Approach
- **Sync script**: `tools/sync_corpus_downloads_to_data.sh`
  - Copies/syncs `corpus-downloads/` → `corpus-data/` 
  - Preserves `corpus-downloads/` (no deletion)
  - Idempotent, with dry-run mode
- **CLI behavior**: 
  - Default to `corpus-data/` when `--input` not specified
  - Error with migration guidance if pointed at `corpus-downloads/`
  - Clear error messages guide users to sync script

### Enforcement Strategy
1. **Path validation function** - centralized input path checking, rejects ANY training terminology
2. **Startup validation** - check and reject problematic paths early
3. **Test guardrails** - tests MUST fail if ANY training concepts (train/, training, etc.) are found
4. **Code scanning** - grep for training terminology in CI pipeline
5. **Documentation** - clear messaging about RAG-only purpose, ZERO tolerance for training concepts

## How to Restart From Scratch

If context is lost, run these commands to rebuild the exact state:

```bash
# 1. Navigate to project root
cd /Users/demouser/Code/dialect-coach

# 2. Check current state
cargo build -p corpus-processor
cargo test -p corpus-processor --no-run

# 3. Read this recovery log completely
# 4. Check TODO status
cat docs/TODO_corpus_processor.md

# 5. Run current test suite (capture output)
cargo test -p corpus-processor 2>&1 | tee docs/recovery/test_run_$(date +%Y%m%d_%H%M%S).log

# 6. Continue from last completed TODO item
```

## Test Execution Logs

### Error Log (Run 1)
*[Will be populated when first full test suite is executed]*

### Changes Applied (Run 1)
*[Will be populated with specific fixes applied based on Run 1 errors]*

### Error Log (Run 2)
*[Will be populated if Run 1 fixes don't resolve all issues]*

## Current State Analysis

### Workspace Structure
- **Crate**: `corpus-processor v0.1.0` (binary crate)
- **Location**: `/Users/demouser/Code/dialect-coach/corpus-processor/`
- **Source files**: 7 Rust files in `src/`
  - `main.rs` - CLI entry point with clap subcommands
  - `lib.rs` - Module exports
  - `chunking.rs` - Text chunking with overlap
  - `embeddings.rs` - Fastembed wrapper
  - `loaders.rs` - File format loaders (txt, csv, tsv, json)
  - `processor.rs` - Main processing pipeline
  - `qdrant.rs` - Qdrant upload client
- **Tests**: `tests/integration_tests.rs` exists with basic tests
- **Build status**: ✅ Compiles successfully (1 dead code warning)

### CLI Surface
**Commands**:
- `process` - Load corpus files and generate embeddings
  - Required: `--language`, `--dialect`, `--input`
  - Optional: `--output [default: output]`, `--chunk-size [default: 512]`, `--overlap [default: 50]`
- `upload` - Upload processed documents to Qdrant
- `list` - List available dialects
- `status` - Check Qdrant collection status

**Key findings**:
- ❌ **No default corpus directory** - input path is required, no default to corpus-data
- ❌ **Generic output default** - defaults to "output" not corpus-data/{language}/{dialect}/processed
- ✅ **No hardcoded training paths found** in source code (CRITICAL to maintain)
- ❌ **No corpus-downloads rejection** - accepts any input path
- ❌ **No training terminology validation** - must add checks to reject ALL training concepts
- ✅ **Existing structure understood** - follows corpus-data/{language}/{dialect}/processed/ pattern

### Dependencies
- **CLI**: clap 4.5, dotenvy 0.15
- **Data processing**: csv 1.3, serde/serde_json 1.0, walkdir 2.5
- **ML/Embeddings**: fastembed 4.9.1 (heavy ML deps: tokenizers, ort, ndarray)
- **Vector DB**: qdrant-client 1.15.0 (gRPC: tonic, prost)
- **Async**: tokio 1.47.1
- **Error handling**: anyhow 1.0, thiserror 2.0
- **Test utils**: tempfile 3.23.0 (dev-dependency)

### Existing Test State
- **File**: `corpus-processor/tests/integration_tests.rs`
- **Tests**: 7 integration tests covering:
  - DialectDocument serialization
  - Chunking with Arabic text
  - Text file loading
  - JSONL save/load format
  - Some manual testing of processing functions
- **Status**: Tests use real file I/O, some functions not exposed for testing
- **Coverage**: Unknown, no coverage tooling integrated

## Coverage Reports

### Baseline Coverage
*[To be populated when coverage tooling is integrated]*

### Target Coverage
- **Minimum**: 90% line coverage
- **Exclusions**: Optional integration tests with external services

## Test Architecture Decisions

### Testing Tools Selected
*[To be populated during implementation]*

### Seams and Mocking Strategy
*[To be populated during implementation]*

### Fixture Strategy
*[To be populated during implementation]*

## Issues Found and Fixes Applied

*[Running log of every issue discovered and the specific fix applied]*

## Data Migration Status

### Sync Script Progress
*[Status of corpus-downloads → corpus-data sync script]*

### Directory Policy Enforcement
*[Status of path validation and error handling]*

## Next Steps

1. **Task 3 - Complete Crate Audit**: ✅ DONE - Audit completed, findings documented above
2. **Task 4 - Define Acceptance Criteria**: 📋 TODO - Define test coverage targets and specifications  
3. **Task 5 - Design Test Seams**: 📋 TODO - Plan traits for embeddings/uploader mocking
4. **Begin parallel agent work**: Create specialized agents for test architecture and implementation

### Immediate Priority
- Need to progress to test architecture phase
- Directory policy is now fully defined and documented
- Current CLI needs modifications: default input path, output path, path validation

---

**Last Updated**: 2025-10-18T19:50:00Z  
**Status**: Audit completed, directory policy finalized, ready for test architecture phase
