# Formality Enum Refactoring Plan

## Overview
Modify the `Formality` enum to remove `DialectRich`, add `ProfessionalCasual`, and rename `Casual` to `Informal`.

## Current Definition
**File:** `shared/src/models/dialect.rs` (lines 487-499)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Formality {
    #[serde(rename = "formal")]
    Formal,
    #[serde(rename = "casual")]
    Casual,
    #[serde(rename = "dialect_rich")]
    DialectRich,
    #[serde(rename = "slang")]
    Slang,
}
```

## Target Definition
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Formality {
    #[serde(rename = "formal")]
    Formal,
    #[serde(rename = "professional_casual")]
    ProfessionalCasual,
    #[serde(rename = "informal")]
    Informal,
    #[serde(rename = "slang")]
    Slang,
}
```

## Files Requiring Updates

### Core Definition (Phase 1)
- `shared/src/models/dialect.rs` - Enum definition + impl blocks (id, name, from_id, FromStr)

### Backend Updates (Phase 2)
- `backend/src/qdrant_service.rs` - Lines 199-202, 263-266: Match arms for parsing
- `backend/src/translation_handler.rs` - Lines 66, 109-112, 160, 210, 214: Default values and match arms
- `backend/src/bin/rag_tester.rs` - Lines 73, 78, 81, 82, 85: Test queries
- `backend/src/agent_service/learning.rs` - Lines 413, 448: Test data
- `backend/src/websocket.rs` - Lines 1035, 1070, 1106: Test data
- `backend/src/test_utils.rs` - Uses Formality

### Frontend Updates (Phase 3)
- `frontend/src/app/app_state.rs` - Lines 446-449, 746, 1207-1210, 1256, 1261: cycle_formality function and tests
- `frontend/src/app/user_state/callbacks.rs` - Lines 57-61, 115-118: Parsing and cycling
- `frontend/src/services/translation.rs` - Lines 88, 124-127, 175-178: Tests
- `frontend/src/components/settings_panel.rs` - Formality selector UI (if any hardcoded values)

### Corpus Processor Updates (Phase 4)
- `corpus-processor/src/processor.rs` - Lines 324-327: Match arms for stats
- `corpus-processor/src/loaders.rs` - Lines 233-235, 246-248: Parse function and tests
- `corpus-processor/src/main.rs` - Line 304: Test data
- `corpus-processor/tests/test_qdrant.rs` - Lines 41, 118, 224, 241: Test data
- `corpus-processor/tests/test_processor.rs` - Lines 16, 56, 61, 142, 147, 152, 157, 162, 255, 260, 321, 326, 341, 344, 358, 363, 368, 373: Test data
- `corpus-processor/tests/test_loaders.rs` - Line 297: Test data

### Shared Library (Additional Phase 1)
- `shared/src/models/participant.rs` - Line 118: Test data
- `shared/src/models/message.rs` - Lines 197, 212: Test data

### Documentation (Phase 5)
- `DEVELOPER_GUIDE.md` - Lines 118-123, 251-252: Examples
- Tutorial files (informational only, not code)

---

## Phase 1: Core Enum Definition
**Subagent:** kiss-code-generator

### Code Style Checklist
- [x] Functions <20 lines (all match arms are concise)
- [x] Pure functions (all impls are pure)
- [x] No defensive coding
- [x] No dead code

### Deliverables
- Modified `Formality` enum with new variants
- Updated `id()` method
- Updated `name()` method
- Updated `from_id()` method
- Updated `FromStr` error message

### Files to Update
- `shared/src/models/dialect.rs`

### Steps
1. Replace enum definition (lines 490-499):
   - Remove `DialectRich` variant
   - Add `ProfessionalCasual` variant after `Formal`
   - Rename `Casual` to `Informal`
   - Update serde renames accordingly

2. Update `id()` method (lines 504-511):
   ```rust
   pub fn id(&self) -> &'static str {
       match self {
           Self::Formal => "formal",
           Self::ProfessionalCasual => "professional_casual",
           Self::Informal => "informal",
           Self::Slang => "slang",
       }
   }
   ```

3. Update `name()` method (lines 514-521):
   ```rust
   pub fn name(&self) -> &'static str {
       match self {
           Self::Formal => "Formal",
           Self::ProfessionalCasual => "Professional Casual",
           Self::Informal => "Informal",
           Self::Slang => "Slang",
       }
   }
   ```

4. Update `from_id()` method (lines 525-533):
   ```rust
   pub fn from_id(id: &str) -> Option<Self> {
       match id {
           "formal" => Some(Self::Formal),
           "professional_casual" => Some(Self::ProfessionalCasual),
           "informal" => Some(Self::Informal),
           "slang" => Some(Self::Slang),
           _ => None,
       }
   }
   ```

5. Update `FromStr` error message (line 542):
   Change `'casual' or 'dialect_rich'` to `'informal' or 'professional_casual'`

### Phase End Verification
- [x] Run `cargo test -p dialect-coach-shared` (expect failures in other crates) - PASSED 130/130
- [x] Run `cargo clippy -p dialect-coach-shared` - PASSED no warnings
- [x] No dead code
- [x] Update status document
- [x] Commit: `git commit -m "Phase 1 (core enum definition) complete"`

### Phase 1 Status: COMPLETE

**Date Completed:** 2025-11-17

**Changes Made:**
1. Updated `Formality` enum definition in `shared/src/models/dialect.rs` (lines 490-499)
   - Added `ProfessionalCasual` variant with serde rename `"professional_casual"`
   - Renamed `Casual` to `Informal` with serde rename `"informal"`
   - Removed `DialectRich` variant

2. Updated all impl methods in `shared/src/models/dialect.rs`:
   - `id()` method (lines 504-511): Updated match arms for new variants
   - `name()` method (lines 514-521): Updated display names
   - `from_id()` method (lines 525-533): Updated docstring and match arms
   - `FromStr` error message (line 542): Updated example IDs in error message

3. Updated shared library test files for Phase 1 compilation:
   - `shared/src/models/dialect.rs` (line 597): Default formality changed to `Informal`
   - `shared/src/logic/chat.rs` (lines 23, 45): Test data updated to use `Informal`
   - `shared/src/models/message.rs` (lines 197, 212, 221): Test data updated to use `Informal`, assertion updated
   - `shared/src/models/participant.rs` (line 118): Test data updated to use `Informal`
   - `shared/src/models/session.rs` (lines 106, 172): Test data updated to use `Informal`
   - `shared/src/models/user_state.rs` (lines 57, 94-95, 265): Constructor, display method, and test data updated

**Test Results:**
- All 130 tests in `dialect-coach-shared` pass
- No clippy warnings
- No dead code detected

---

## Phase 2: Backend Crate Updates
**Subagent:** modular-builder (cross-file changes)

### Code Style Checklist
- [ ] Functions <20 lines
- [ ] Pure functions where applicable
- [ ] No defensive coding
- [ ] No dead code

### Deliverables
- All backend code compiles with new enum
- All backend tests pass

### Files to Update
1. `backend/src/qdrant_service.rs`
2. `backend/src/translation_handler.rs`
3. `backend/src/bin/rag_tester.rs`
4. `backend/src/agent_service/learning.rs`
5. `backend/src/websocket.rs`

### Steps

1. **backend/src/qdrant_service.rs** (lines 199-202, 263-266):
   Replace match arms:
   ```rust
   "Formal" => Some(dialect_coach_shared::Formality::Formal),
   "ProfessionalCasual" => Some(dialect_coach_shared::Formality::ProfessionalCasual),
   "Informal" => Some(dialect_coach_shared::Formality::Informal),
   "Slang" => Some(dialect_coach_shared::Formality::Slang),
   ```
   (Do this for both occurrences)

2. **backend/src/translation_handler.rs**:
   - Line 66: Change `Formality::Casual` to `Formality::Informal`
   - Lines 109-112: Update match arms:
     ```rust
     Formality::Formal => "formal and polite",
     Formality::ProfessionalCasual => "professional yet casual",
     Formality::Informal => "casual and conversational",
     Formality::Slang => "informal with slang and colloquialisms",
     ```
   - Line 160: Change `Formality::Casual` to `Formality::Informal`
   - Line 210: Change `Formality::Casual` to `Formality::Informal`
   - Line 214: Change match arm for `Formality::Casual` to `Formality::Informal`

3. **backend/src/bin/rag_tester.rs**:
   - Lines 73, 78, 81, 82, 85: Change `Formality::DialectRich` to `Formality::Informal`

4. **backend/src/agent_service/learning.rs**:
   - Lines 413, 448: Change `Formality::Casual` to `Formality::Informal`

5. **backend/src/websocket.rs**:
   - Lines 1035, 1070, 1106: Change `Formality::Casual` to `Formality::Informal`

### Phase End Verification
- [ ] Run `cargo test -p dialect-coach-backend`
- [ ] Run `cargo clippy -p dialect-coach-backend`
- [ ] No dead code
- [ ] Update status document
- [ ] Commit: `git commit -m "Phase 2 (backend updates) complete"`

---

## Phase 3: Frontend Crate Updates
**Subagent:** modular-builder (cross-file changes)

### Code Style Checklist
- [ ] Functions <20 lines
- [ ] Pure functions where applicable
- [ ] No defensive coding
- [ ] No dead code

### Deliverables
- All frontend code compiles with new enum
- cycle_formality reflects new variant ordering
- All frontend tests pass

### Files to Update
1. `frontend/src/app/app_state.rs`
2. `frontend/src/app/user_state/callbacks.rs`
3. `frontend/src/services/translation.rs`

### Steps

1. **frontend/src/app/app_state.rs**:
   - Lines 446-449: Update `cycle_formality` function:
     ```rust
     fn cycle_formality(current: Formality) -> Formality {
         match current {
             Formality::Formal => Formality::ProfessionalCasual,
             Formality::ProfessionalCasual => Formality::Informal,
             Formality::Informal => Formality::Slang,
             Formality::Slang => Formality::Formal,
         }
     }
     ```
   - Line 746: Change `Formality::Casual` to `Formality::Informal`
   - Lines 1207-1210: Update test assertions:
     ```rust
     assert_eq!(cycle_formality(Formality::Formal), Formality::ProfessionalCasual);
     assert_eq!(cycle_formality(Formality::ProfessionalCasual), Formality::Informal);
     assert_eq!(cycle_formality(Formality::Informal), Formality::Slang);
     assert_eq!(cycle_formality(Formality::Slang), Formality::Formal);
     ```
   - Line 1256: Change `Formality::Formal` stays same
   - Line 1261: Change expected `Formality::Casual` to `Formality::ProfessionalCasual`

2. **frontend/src/app/user_state/callbacks.rs**:
   - Lines 57-61: Update parsing:
     ```rust
     "formal" => Formality::Formal,
     "professional_casual" => Formality::ProfessionalCasual,
     "informal" => Formality::Informal,
     "slang" => Formality::Slang,
     _ => Formality::Informal,
     ```
   - Lines 115-118: Update formalities array:
     ```rust
     Formality::Formal,
     Formality::ProfessionalCasual,
     Formality::Informal,
     Formality::Slang,
     ```

3. **frontend/src/services/translation.rs**:
   - Line 88: Change `Formality::Casual` to `Formality::Informal`
   - Lines 124-127: Update test cases:
     ```rust
     (Formality::Formal, "formal"),
     (Formality::ProfessionalCasual, "professional_casual"),
     (Formality::Informal, "informal"),
     (Formality::Slang, "slang"),
     ```
   - Lines 175-178: Update assertions:
     ```rust
     assert_eq!(Formality::Formal.id(), "formal");
     assert_eq!(Formality::ProfessionalCasual.id(), "professional_casual");
     assert_eq!(Formality::Informal.id(), "informal");
     assert_eq!(Formality::Slang.id(), "slang");
     ```

### Phase End Verification
- [ ] Run `cargo test -p dialect-coach-frontend`
- [ ] Run `cargo clippy -p dialect-coach-frontend`
- [ ] No dead code
- [ ] Update status document
- [ ] Commit: `git commit -m "Phase 3 (frontend updates) complete"`

---

## Phase 4: Corpus Processor and Shared Tests
**Subagent:** modular-builder (cross-file changes)

### Code Style Checklist
- [ ] Functions <20 lines
- [ ] Pure functions where applicable
- [ ] No defensive coding
- [ ] No dead code

### Deliverables
- All corpus-processor code compiles
- All shared library tests pass
- All corpus-processor tests pass

### Files to Update
1. `corpus-processor/src/processor.rs`
2. `corpus-processor/src/loaders.rs`
3. `corpus-processor/src/main.rs`
4. `corpus-processor/tests/test_qdrant.rs`
5. `corpus-processor/tests/test_processor.rs`
6. `corpus-processor/tests/test_loaders.rs`
7. `shared/src/models/participant.rs`
8. `shared/src/models/message.rs`

### Steps

1. **corpus-processor/src/processor.rs** (lines 324-327):
   Update match arms (note: might need context to see full match):
   ```rust
   Some(dialect_coach_shared::Formality::Formal) => formal += 1,
   Some(dialect_coach_shared::Formality::ProfessionalCasual) => professional_casual += 1,
   Some(dialect_coach_shared::Formality::Informal) => informal += 1,
   Some(dialect_coach_shared::Formality::Slang) => slang += 1,
   ```
   Also update corresponding counter variables.

2. **corpus-processor/src/loaders.rs**:
   - Lines 233-235: Update parse_formality:
     ```rust
     "formal" => Some(Formality::Formal),
     "professional_casual" => Some(Formality::ProfessionalCasual),
     "informal" | "casual" => Some(Formality::Informal),
     "slang" => Some(Formality::Slang),
     ```
   - Lines 246-248: Update tests:
     ```rust
     assert!(matches!(parse_formality("formal"), Some(Formality::Formal)));
     assert!(matches!(parse_formality("informal"), Some(Formality::Informal)));
     assert!(matches!(parse_formality("SLANG"), Some(Formality::Slang)));
     ```

3. **corpus-processor/src/main.rs** (line 304):
   Keep `Some(Formality::Formal)` (no change needed)

4. **corpus-processor/tests/test_qdrant.rs**:
   Keep `Some(Formality::Formal)` usages (no change needed)

5. **corpus-processor/tests/test_processor.rs**:
   - Lines using `Formality::Casual` → `Formality::Informal`
   - Lines using `Formality::DialectRich` → `Formality::ProfessionalCasual` or `Formality::Informal`
   - Update all test assertions accordingly

6. **corpus-processor/tests/test_loaders.rs** (line 297):
   Keep `Some(Formality::Formal)` (no change needed)

7. **shared/src/models/participant.rs** (line 118):
   Change `Formality::Casual` to `Formality::Informal`

8. **shared/src/models/message.rs** (lines 197, 212):
   Change `Formality::Casual` to `Formality::Informal`

### Phase End Verification
- [ ] Run `cargo test --workspace`
- [ ] Run `cargo clippy --workspace`
- [ ] No dead code
- [ ] Update status document
- [ ] Commit: `git commit -m "Phase 4 (corpus-processor and shared tests) complete"`

---

## Phase 5: Documentation Updates
**Subagent:** kiss-code-generator

### Code Style Checklist
- [ ] No dead code references

### Deliverables
- DEVELOPER_GUIDE.md reflects new enum

### Files to Update
- `DEVELOPER_GUIDE.md`

### Steps
1. Update enum definition example (lines 118-123)
2. Update any usage examples (lines 251-252)

### Phase End Verification
- [ ] Verify all documentation is accurate
- [ ] Commit: `git commit -m "Phase 5 (documentation) complete"`

---

## Final Verification
- [ ] `cargo build --workspace` succeeds
- [ ] `cargo test --workspace` passes 100%
- [ ] `cargo clippy --workspace` clean
- [ ] No dead code anywhere
- [ ] All commits complete
