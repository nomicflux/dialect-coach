# Corpus Processor Testing TODO

**Created**: 2025-10-18T19:47:05Z  
**Objective**: Implement comprehensive testing for corpus-processor crate with systematic approach

## Status Legend
- ✅ **DONE** - Task completed and verified
- 🔄 **IN_PROGRESS** - Currently working on this task  
- 📋 **TODO** - Not started yet
- ❌ **BLOCKED** - Cannot proceed without external input
- ⏸️ **PAUSED** - Temporarily stopped

---

## Phase 1: Documentation & Setup

### 📋 Task 1: Recovery Documentation
**Owner**: Main Agent  
**Status**: ✅ **DONE**  
**Started**: 2025-10-18T19:47:05Z  
**Completed**: 2025-10-18T19:47:05Z  

- [x] Create `docs/recovery/corpus_processor_recovery.md`
- [x] Create `docs/TODO_corpus_processor.md` 
- [x] Define non-negotiables and directory policy
- [x] Document restart-from-scratch procedure

### 📋 Task 2: Directory Policy Finalization
**Owner**: Main Agent  
**Status**: ✅ **DONE**  
**Started**: 2025-10-18T19:50:00Z  
**Completed**: 2025-10-18T19:50:00Z  
**Dependencies**: Task 1 ✅

**Subtasks**:
- [x] Document authoritative policy in recovery log
- [x] Define enforcement approach (tests-first)
- [x] Plan error messages for corpus-downloads rejection
- [x] Design guardrail tests for ANY training terminology detection
- [x] Move processed files to proper dialect structure (corpus-data/{language}/{dialect}/processed/)
- [x] Strengthen training prohibition to cover ALL training concepts, not just train/

### 📋 Task 3: Crate Audit
**Owner**: Main Agent (completed directly)  
**Status**: ✅ **DONE**  
**Started**: 2025-10-18T19:48:00Z  
**Completed**: 2025-10-18T19:50:00Z  
**Dependencies**: Task 1 ✅

**Commands to run** (paste output into recovery doc):
- [x] `cargo metadata -q --format-version 1 | jq '.packages[] | select(.name == "corpus-processor")'`
- [x] `cargo tree -p corpus-processor`
- [x] `ls -la corpus-processor/`
- [x] `find corpus-processor -name "*.rs" | head -20`
- [x] `grep -r "corpus-downloads\|corpus_data\|corpus-data\|train" corpus-processor/src/`
- [x] `cargo build -p corpus-processor` (verify it builds)

**Document findings**:
- [x] CLI commands and defaults
- [x] Current directory handling 
- [x] Dependency analysis
- [x] Existing test state

**Key Audit Results**:
- ✅ Builds successfully with 1 dead code warning
- ❌ No default input directory (--input required)
- ❌ Generic "output" default instead of corpus-data/{language}/{dialect}/processed
- ✅ No hardcoded training paths found (CRITICAL to maintain)
- ❌ No corpus-downloads path validation
- ❌ No training terminology validation (must add)
- ✅ Good existing test foundation (7 integration tests)
- ✅ Processed files moved to proper dialect structure (corpus-data/{language}/{dialect}/processed/)

---

## Phase 2: Test Architecture

### 📋 Task 4: Acceptance Criteria Definition
**Owner**: Main Agent  
**Status**: ✅ **DONE**  
**Started**: 2025-10-18T20:26:33Z  
**Completed**: 2025-10-18T20:26:33Z  
**Dependencies**: Task 3 ✅

**Coverage targets**:
- [x] 90% line coverage minimum on corpus-processor
- [x] Unit tests for all concerns listed in requirements
- [x] Integration tests (offline with mocks)
- [x] Optional integration tests (real services, ignored by default)
- [x] Directory policy tests (critical for training prohibition)
- [x] Code scanning tests for training terminology

### 📋 Task 5: Test Seams Design
**Owner**: TestArchitectAgent  
**Status**: 📋 **TODO**  
**Dependencies**: Task 3 ✅

**Seams to introduce** (additions only, no behavior changes):
- [ ] EmbeddingProvider trait with deterministic test impl
- [ ] UploaderService trait with HTTP mock impl  
- [ ] PathResolver utility with policy enforcement
- [ ] test_support module with temp dirs and fixtures

### 📋 Task 6: Testing Tools Selection
**Owner**: TestArchitectAgent  
**Status**: 📋 **TODO**  
**Dependencies**: Task 4 ✅

**Required capabilities** (choose specific crates):
- [ ] CLI binary testing (`assert_cmd`?)
- [ ] Temp filesystem (`tempfile`?)
- [ ] Output matching (`predicates`?)
- [ ] Snapshot testing (`insta`?)
- [ ] HTTP mocking (`wiremock`?)
- [ ] JSON assertions (`serde_json`?)

Pin versions and document in recovery log.

---

## Phase 3: Test Implementation

### 📋 Task 7: Fixture Strategy Implementation
**Owner**: FixtureBuilderAgent (to be created)  
**Status**: 📋 **TODO**  
**Dependencies**: Task 5 ✅, Task 6 ✅

**Deliverables**:
- [ ] Temp directory scaffolding
- [ ] Generated fixture builders (2-3 small .txt files)
- [ ] CSV fixtures (if CSV supported)
- [ ] Snapshot storage under corpus-processor/tests/snapshots/
- [ ] Deterministic, isolated per-test fixtures

### 📋 Task 8: Unit Tests Implementation
**Owner**: UnitTestAgent (to be created)  
**Status**: 📋 **TODO**  
**Dependencies**: Task 7 ✅

**Test modules to implement**:
- [ ] Chunking tests (size, overlap, Unicode, boundaries)
- [ ] Dialect parsing tests (BCP-47 codes, error cases)
- [ ] File scanning tests (empty files, BOMs, CSV edge cases)
- [ ] Embeddings seam tests (deterministic vectors, errors)
- [ ] JSONL emission tests (schema, ordering, newlines)
- [ ] CLI parsing tests (flags, exit codes, paths)
- [ ] Path policy tests (corpus-data default, reject corpus-downloads/train)

### 📋 Task 9: Integration Tests Implementation  
**Owner**: IntegrationTestAgent (to be created)  
**Status**: 📋 **TODO**  
**Dependencies**: Task 7 ✅

**Test files to create**:
- [ ] `corpus-processor/tests/cli_process.rs` - End-to-end processing
- [ ] `corpus-processor/tests/cli_upload_mock.rs` - Upload with HTTP mock  
- [ ] `corpus-processor/tests/integration_qdrant.rs` - Real Qdrant (ignored)

**Test scenarios**:
- [ ] CLI "process" with temp corpus-data → JSONL output
- [ ] No train/ directories created anywhere
- [ ] Upload via HTTP mock with correct payloads
- [ ] Real Qdrant test (env var gated, ignored by default)

---

## Phase 4: Test Execution & Fixes

### 📋 Task 10: Complete Test Suite Writing
**Owner**: Multiple agents (parallel)  
**Status**: 📋 **TODO**  
**Dependencies**: Tasks 8 ✅, Task 9 ✅

**Verification**:
- [ ] All unit tests written and compiling
- [ ] All integration tests written and compiling  
- [ ] No production code changes made yet
- [ ] Tests use isolated temp dirs only

### 📋 Task 11: First Full Test Run
**Owner**: RunnerAgent (to be created)  
**Status**: 📋 **TODO**  
**Dependencies**: Task 10 ✅

**Critical requirements**:
- [ ] Run `cargo test -p corpus-processor` ONCE only
- [ ] Capture complete output verbatim
- [ ] Paste ALL failure messages into recovery doc "Error Log (Run 1)"
- [ ] Do NOT run individual tests
- [ ] Do NOT make any code changes yet

### 📋 Task 12: Failure Triage
**Owner**: TriageAgent (to be created)  
**Status**: 📋 **TODO**  
**Dependencies**: Task 11 ✅

**For every failing test, document**:
- [ ] Exact error text and location
- [ ] Suspected root cause
- [ ] Category: test bug, production bug, missing seam, policy mismatch
- [ ] Proposed minimal fix
- [ ] Priority: directory policy > CLI determinism > core pipeline > uploader

### 📋 Task 13: Fix Proposal
**Owner**: TriageAgent  
**Status**: 📋 **TODO**  
**Dependencies**: Task 12 ✅

**Deliverable**:
- [ ] Ordered list of minimal fixes in recovery doc
- [ ] Each fix: file(s) to change, exact code change, follow-up tests
- [ ] Get sign-off before proceeding

### 📋 Task 14: Fix Implementation
**Owner**: FixerAgent (to be created)  
**Status**: 📋 **TODO**  
**Dependencies**: Task 13 ✅

**Likely fixes** (only if triage confirms):
- [ ] Enforce corpus-data default, reject corpus-downloads
- [ ] Remove any train/ creation, add guards
- [ ] Stabilize JSONL output (ordering, newlines, schema)  
- [ ] Wire embedding/uploader traits for test mocks
- [ ] Align chunking to spec

### 📋 Task 15: Iteration Until Green
**Owner**: RunnerAgent + FixerAgent  
**Status**: 📋 **TODO**  
**Dependencies**: Task 14 ✅

**Process**:
- [ ] Run `cargo test -p corpus-processor` 
- [ ] Update recovery doc with "Error Log (Run N)" and "Changes Applied (Run N)"
- [ ] Repeat triage → fixes → full run until all tests pass
- [ ] Final run shows 0 failures

---

## Phase 5: Coverage & CI

### 📋 Task 16: Coverage Integration
**Owner**: CoverageAgent (to be created)  
**Status**: 📋 **TODO**  
**Dependencies**: Task 15 ✅

**Deliverables**:
- [ ] Integrate Rust coverage tool (document commands in recovery log)
- [ ] Generate HTML coverage report locally
- [ ] Capture percentage in recovery doc
- [ ] Enforce 90% threshold
- [ ] Exclude optional/ignored tests from threshold

### 📋 Task 17: CI Pipeline Setup
**Owner**: CIEngineerAgent (to be created)  
**Status**: 📋 **TODO**  
**Dependencies**: Task 16 ✅

**CI workflow for corpus-processor**:
- [ ] Format check (`cargo fmt --check`)
- [ ] Lint with warnings as errors (`cargo clippy -- -D warnings`)  
- [ ] Full test suite (`cargo test -p corpus-processor`)
- [ ] Coverage report generation and threshold enforcement
- [ ] Separate job for ignored real-service tests (secrets conditional)
- [ ] Cache dependencies and artifacts
- [ ] Publish test results and coverage as artifacts

---

## Phase 6: Data Migration & Documentation

### 📋 Task 18: Sync Script Implementation
**Owner**: Main Agent  
**Status**: 📋 **TODO**  
**Dependencies**: Task 15 ✅ (tests green)

**Deliverable**: `tools/sync_corpus_downloads_to_data.sh`
- [ ] Copy/sync corpus-downloads/ → corpus-data/
- [ ] Preserve corpus-downloads/ (no deletion)
- [ ] Idempotent, dry-run mode
- [ ] Handle duplicates (document rule)
- [ ] Add tests using temp dirs

### 📋 Task 19: Documentation Updates
**Owner**: DocWriterAgent (to be created)  
**Status**: 📋 **TODO**  
**Dependencies**: Task 17 ✅, Task 18 ✅

**Files to update**:
- [ ] `corpus-processor/README.md` - canonical directory, RAG-only statement
- [ ] `corpus-processor/TESTING.md` - test procedures, snapshots, ignored tests
- [ ] `WARP.md` - align references to corpus-data, sync script
- [ ] Recovery doc - final status and handoff notes

### 📋 Task 20: Policy Enforcement Tests
**Owner**: Main Agent  
**Status**: 📋 **TODO**  
**Dependencies**: Task 18 ✅

**Validation tests**:
- [ ] Path resolution unit tests (corpus-data default, reject others)
- [ ] Startup validation in CLI (error messages guide to sync script)
- [ ] No train/ directory creation confirmed via tests

---

## Phase 7: Delivery

### 📋 Task 21: Final Verification
**Owner**: Main Agent  
**Status**: 📋 **TODO**  
**Dependencies**: All previous tasks ✅

**Verification checklist**:
- [ ] Full test suite passes: `cargo test -p corpus-processor`
- [ ] Coverage meets 90% threshold
- [ ] CI pipeline runs successfully
- [ ] Documentation is current and accurate
- [ ] Sync script works and is tested
- [ ] Directory policy is enforced
- [ ] Recovery doc is complete

### 📋 Task 22: Handoff Preparation
**Owner**: Main Agent  
**Status**: 📋 **TODO**  
**Dependencies**: Task 21 ✅

**Final deliverables**:
- [ ] Update recovery doc status to "COMPLETE"
- [ ] Mark all TODO items as DONE
- [ ] Verify restart-from-scratch procedure works
- [ ] Commit all changes
- [ ] Document any known limitations or future work

---

## Agent Assignments

### Parallel Workstreams (can work simultaneously)
- **DocWriterAgent**: Tasks 1 ✅, 19
- **CorpusAuditAgent**: Task 3  
- **TestArchitectAgent**: Tasks 4, 5, 6
- **FixtureBuilderAgent**: Task 7
- **UnitTestAgent**: Task 8  
- **IntegrationTestAgent**: Task 9

### Sequential Workstreams (must wait for dependencies)
- **RunnerAgent**: Tasks 11, 15 (test execution only)
- **TriageAgent**: Tasks 12, 13 (failure analysis only)
- **FixerAgent**: Tasks 14, 15 (code changes only)
- **CoverageAgent**: Task 16
- **CIEngineerAgent**: Task 17

### Main Agent: Tasks 2, 10, 18, 20, 21, 22

---

**Last Updated**: 2025-10-18T19:47:05Z  
**Overall Status**: Phase 1 started - Recovery docs created