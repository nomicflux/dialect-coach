# Test Coverage Plan: corpus-processor

## Goal
Achieve 100% line and branch coverage for the corpus-processor package, ensuring all code paths are tested with deterministic, offline tests.

## Target Modules

### Core Modules (Full Coverage Required)
- `main.rs` - CLI entry point, argument parsing, command dispatch
- `processor.rs` - Core processing pipeline (load → chunk → embed → save)
- `embeddings.rs` - Embedding service wrapper 
- `qdrant.rs` - Vector database client
- `loaders.rs` - File format parsers (text, CSV, JSON, TSV)
- `chunking.rs` - Text segmentation with overlap
- `test_seams.rs` - Dependency injection traits and path resolution

### Test Infrastructure
- Unit tests with mocks/dependency injection
- Integration tests for CLI commands
- End-to-end pipeline tests with temporary directories
- All external dependencies (Qdrant, Fastembed) mocked by default
- Real external service tests marked with `#[ignore]`

## Coverage Tools & Commands

### Setup
```bash
# Install coverage tool
cargo install cargo-llvm-cov

# Run coverage
cargo llvm-cov --package corpus-processor --html --open
```

### CI Integration
- Minimum 100% line coverage threshold
- Exclude patterns documented below
- Fail build on coverage regression

## Current Status

### Compilation Issues (Blocking)
- `test_qdrant.rs`: Invalid `contains_key()` calls on `Payload` type
- Multiple unused import warnings

### Test Coverage Gaps
- Main CLI functions (`parse_dialect`, `load_documents_from_jsonl`) 
- Error paths in all modules
- Edge cases and boundary conditions
- Embedding service error handling
- Qdrant connection/upload failures

## Coverage Strategy

### 1. Fix Compilation (Immediate)
- Replace `payload.contains_key()` with proper Payload API
- Clean up unused imports and warnings

### 2. Mock Infrastructure
- Trait-based dependency injection for EmbeddingService and QdrantService
- Deterministic mock implementations in `test_seams.rs`
- No network access in default test runs

### 3. Systematic Test Coverage
- **Happy path**: Each public function with valid inputs
- **Error paths**: Each distinct error condition and propagation
- **Edge cases**: Boundary values, empty inputs, size limits
- **Integration**: CLI command combinations with mocked external services

### 4. Coverage Measurement Loop
1. Run coverage tool
2. Identify uncovered lines/branches
3. Write targeted tests
4. Repeat until 100% or documented exclusions

## Exclusions (If Any)
Document any intentionally untested code with justification:
- Generated code (if applicable)
- Unreachable error paths (with proof)
- External service integration (real network calls, covered by ignored tests)

## Test Organization

### Unit Tests (src/lib.rs modules)
- `chunking.rs` → embedded `#[cfg(test)]` module
- `embeddings.rs` → embedded tests + mock trait impl
- `loaders.rs` → embedded tests (already partially done)
- `processor.rs` → comprehensive error path coverage
- `qdrant.rs` → mock client tests
- `test_seams.rs` → PathResolver and mock infrastructure tests

### Integration Tests (tests/ directory)
- `test_cli_integration.rs` → command-line interface (expand existing)
- `test_processor.rs` → pipeline integration (expand existing)  
- `test_qdrant.rs` → fix compilation, add mock-based tests
- `integration_tests.rs` → end-to-end flows with temps

### Test Principles
1. **Deterministic**: No randomness, fixed seeds if needed
2. **Offline**: No network, filesystem access via tempfile only
3. **Fast**: Sub-second execution for full test suite
4. **Isolated**: Tests don't depend on each other or external state
5. **Comprehensive**: Every branch, error path, and edge case covered

## Sub-Agent Delegation Opportunities
- **Qdrant Mock Agent**: Focus on Qdrant client abstraction and testing
- **Embeddings Mock Agent**: Embedding service traits and deterministic test implementations  
- **CLI Integration Agent**: Command-line argument parsing and error message validation
- **Coverage Agent**: Coverage measurement, gap analysis, and reporting

## Success Criteria
- [ ] `cargo test -p corpus-processor` passes with 0 failures
- [ ] `cargo llvm-cov --package corpus-processor` reports 100% line coverage
- [ ] All external dependencies properly mocked
- [ ] No network access during default test runs
- [ ] All error paths tested and documented
- [ ] CI enforces coverage threshold
- [ ] Documentation updated in WARP.md

## Development Workflow
1. Fix compilation errors
2. Establish coverage baseline measurement
3. Implement missing unit tests (parallel development possible)
4. Add integration test coverage
5. Measure and iterate until 100%
6. Enforce in CI and update documentation