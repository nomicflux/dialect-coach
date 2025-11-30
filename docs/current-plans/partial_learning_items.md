# Partial Learning Items Enrichment - Implementation Plan

**Feature:** Allow users to provide partial information when manually adding learning items, with AI agent filling in missing required fields.

**Created:** 2025-11-30

---

## Architecture Analysis

### Current State Assessment

**Data Structures** (`shared/src/models/agent.rs`):
- All learning item types have required fields
- Translated has one optional field (`context`)
- No existing "partial" or "draft" types

**UI State** (`frontend/src/components/learning_panel.rs`):
- Form validates that ALL required fields are non-empty
- Direct construction from form fields
- Manual entry complete (Phase 1-4 from manual_learning_items.md)

**Backend Architecture**:
- AgentService has 3 channels: response_agent, learning_agent, analysis_agent
- Translation handler exists (`translation_handler.rs`) - HTTP POST endpoint
- No existing enrichment/completion endpoint

### Key Architectural Decisions

#### 1. Enrichment Request Types

We need new types to represent partial data:

**Problem:** Current types (Mistake, Translated, etc.) enforce ALL required fields
**Solution:** Create parallel "partial" request types that mirror the full types but with `Option<T>` for fields that can be auto-filled

**Example:**
```rust
// Full type (existing)
pub struct Translated {
    pub translated_word: String,      // required
    pub translated_to: String,         // required
    pub context: Option<String>,       // already optional
}

// Partial type (new)
pub struct PartialTranslated {
    pub translated_word: Option<String>,  // can be filled
    pub translated_to: Option<String>,    // can be filled
    pub context: Option<String>,          // never auto-filled
}
```

#### 2. Bidirectional Language Detection

**Challenge:** Translation fields `translated_word` and `translated_to` don't indicate which is English vs target language

**Solution:** Enrichment agent must:
1. Receive current `Dialect` from frontend
2. Detect language of provided text
3. Determine if provided text is English or target language
4. Fill the opposite field accordingly

**Example Flow:**
- User dialect: SpanishMexican
- User provides: "Hola"
- Agent detects: Spanish text
- Agent fills: `translated_word: "Hello"` (English)
- Result: `translated_word: "Hello"`, `translated_to: "Hola"`

#### 3. Validation Rules

**Mistake:**
- MUST have `specific_mistake` (the error)
- Can auto-fill `correction` OR `mistake_category` (not both)
- Never allow correction-only (mistakes must show the error)

**Translated:**
- MUST have exactly one of: `translated_word` OR `translated_to`
- Never auto-fill `context` (explicit requirement)

**Explained:**
- Can provide `new_phrase` OR `explanation`
- Agent fills missing field

**Exploratory:**
- Can provide `point_to_try` OR `instructions_for_use`
- Agent fills missing field

#### 4. HTTP Endpoint Design

**Endpoint:** `POST /api/learning/enrich`

**Authentication:** Use existing auth middleware (same as /api/translate)

**Request Body:**
```rust
pub struct EnrichRequest {
    pub dialect: Dialect,
    pub item_type: LearningItemType,  // Mistake/Explained/Translated/Exploratory
    pub partial_data: PartialLearningItem,
}

pub enum PartialLearningItem {
    Mistake(PartialMistake),
    Explained(PartialExplained),
    Translated(PartialTranslated),
    Exploratory(PartialExploratory),
}
```

**Response Body:**
```rust
pub struct EnrichResponse {
    pub enriched_item: LearningItem,  // Complete item ready to save
}
```

---

## Research Findings

### Agent Service Structure
- Located: `backend/src/agent_service.rs`
- Has 3 agent channels (response, learning, analysis)
- **Recommendation:** Use `learning_agent` for enrichment since it already handles learning item extraction

### Existing Translation Handler Pattern
- Located: `backend/src/translation_handler.rs`
- Shows HTTP endpoint pattern for AI operations
- Uses request/response structs
- Calls agent service directly
- **Pattern to follow:** Create similar `enrichment_handler.rs`

### Frontend Service Layer
- Located: `frontend/src/services/`
- Has `translation_service.rs` as HTTP call example
- **Pattern to follow:** Create `enrichment_service.rs` with similar error handling

---

## Implementation Phases

### Phase 1: Shared Types for Partial Data

**Boundary:** Create type system for partial learning items (shared crate only)

**Subagent:** kiss-code-generator

**Code Style Checklist:**
- [x] Functions <20 lines
- [x] Pure functions where possible
- [x] No defensive coding
- [x] Helper functions for complex logic
- [x] Full functionality (no TODOs)

**Status:** COMPLETED 2025-11-30

**Deliverables:**
1. New file: `shared/src/models/partial_learning_item.rs`
   - `PartialMistake` struct with `Option<String>` fields
   - `PartialExplained` struct
   - `PartialTranslated` struct
   - `PartialExploratory` struct
   - `PartialLearningItem` enum wrapping all 4 types
   - Serde derives for all types

2. New file: `shared/src/models/enrichment.rs`
   - `EnrichRequest` struct
   - `EnrichResponse` struct
   - Both with Serialize + Deserialize

3. Update: `shared/src/models/mod.rs`
   - Export new types publicly

**Actionable Items:**
1. Create `partial_learning_item.rs` with 4 partial structs matching agent.rs structure but with Option<String> fields
2. Create `enrichment.rs` with request/response types
3. Add validation helper: `fn validate_partial_mistake(p: &PartialMistake) -> Result<()>` - ensures specific_mistake is Some
4. Add validation helper: `fn validate_partial_translated(p: &PartialTranslated) -> Result<()>` - ensures exactly one field is Some
5. Add validation helper: `fn validate_partial_explained(p: &PartialExplained) -> Result<()>` - ensures at least one field is Some
6. Add validation helper: `fn validate_partial_exploratory(p: &PartialExploratory) -> Result<()>` - ensures at least one field is Some
7. Update mod.rs exports

**Test Requirements:**
- Test validation functions with valid partial data
- Test validation functions reject invalid combinations
- Test serialization round-trip for all partial types
- Test EnrichRequest/EnrichResponse serialization

**Success Criteria:**
- All shared types compile
- Validation prevents invalid partial data
- Serde works for all types
- Tests pass: `cargo test -p dialect-coach-shared`

**Phase End:**
```bash
cargo test -p dialect-coach-shared
cargo clippy -p dialect-coach-shared -- -D warnings
# Remove ALL dead code warnings
# Update this document with completion status
git add shared/src/models/partial_learning_item.rs shared/src/models/enrichment.rs shared/src/models/mod.rs
git commit -m "Phase 1 (partial data types) complete"
```

**Test Results:**
- cargo test -p dialect-coach-shared: 184 tests PASSED
- cargo clippy -p dialect-coach-shared -- -D warnings: NO WARNINGS

**Deliverables Completed:**
- [x] `/Users/demouser/Code/dialect-coach/shared/src/models/partial_learning_item.rs` - 4 partial structs + PartialLearningItem enum + validation functions + 38 tests
- [x] `/Users/demouser/Code/dialect-coach/shared/src/models/enrichment.rs` - EnrichRequest + EnrichResponse + 4 tests
- [x] Updated `/Users/demouser/Code/dialect-coach/shared/src/models/mod.rs` - Added module and public exports

**Implementation Details:**
- PartialMistake: specific_mistake (required), correction, mistake_category - all Option<String>
- PartialExplained: new_phrase, explanation - both Option<String>
- PartialTranslated: translated_word, translated_to, context - all Option<String>
- PartialExploratory: point_to_try, instructions_for_use - both Option<String>
- Validation functions enforce business rules:
  - validate_partial_mistake: ensures specific_mistake is Some
  - validate_partial_translated: ensures exactly one of translated_word OR translated_to is Some
  - validate_partial_explained: ensures at least one field is Some
  - validate_partial_exploratory: ensures at least one field is Some
- EnrichRequest: dialect + partial_data (serializable)
- EnrichResponse: enriched_item as serde_json::Value (flexible for different types)

---

### Phase 2: Backend Enrichment Logic

**Boundary:** AI agent logic to fill missing fields (backend agent_service only)

**Subagent:** modular-builder

**Code Style Checklist:**
- [x] Functions <20 lines
- [x] Pure functions where possible
- [x] No defensive coding
- [x] Helper functions for complex logic
- [x] Full functionality (no TODOs)

**Status:** COMPLETED 2025-11-30

**Deliverables:**
1. New file: `backend/src/agent_service/enrichment.rs`
   - `build_enrichment_prompt()` - creates prompt for each partial type
   - `parse_enrichment_response()` - parses AI response to complete item
   - `enrich_partial_mistake()` - calls agent to fill missing Mistake fields
   - `enrich_partial_translated()` - handles bidirectional translation with dialect context
   - `enrich_partial_explained()` - fills Explained fields
   - `enrich_partial_exploratory()` - fills Exploratory fields

2. Update: `backend/src/agent_service.rs`
   - Add `pub mod enrichment;`
   - Add public method: `pub async fn enrich_learning_item(&self, req: EnrichRequest) -> Result<EnrichResponse>`
   - Method delegates to enrichment module functions

**Actionable Items:**
1. Create enrichment.rs module skeleton
2. Write `build_mistake_prompt(partial: &PartialMistake, dialect: Dialect) -> String` - returns AI prompt
3. Write `build_translated_prompt(partial: &PartialTranslated, dialect: Dialect) -> String` - includes language detection instructions
4. Write `build_explained_prompt(partial: &PartialExplained, dialect: Dialect) -> String`
5. Write `build_exploratory_prompt(partial: &PartialExploratory, dialect: Dialect) -> String`
6. Write `parse_mistake_response(text: &str) -> Result<Mistake>` - extracts fields from AI response
7. Write `parse_translated_response(text: &str, partial: &PartialTranslated) -> Result<Translated>` - handles bidirectional mapping
8. Write `parse_explained_response(text: &str) -> Result<Explained>`
9. Write `parse_exploratory_response(text: &str) -> Result<Exploratory>`
10. Write top-level `enrich_partial_mistake()` that calls learning_agent
11. Write top-level `enrich_partial_translated()` with language detection
12. Write top-level `enrich_partial_explained()`
13. Write top-level `enrich_partial_exploratory()`
14. Add `AgentService::enrich_learning_item()` dispatcher
15. Update agent_service.rs to export enrichment module

**Prompt Design Examples:**

**Mistake enrichment (correction missing):**
```
The user made this mistake in Mexican Spanish: "Cómo andes"

Provide the correct version and categorize the mistake type.

Response format:
CORRECTION: [correct version]
CATEGORY: [SpellingError|VocabularyError|GrammarError|DialectUsageError|Other]
CONTEXT: [brief explanation for category context field]
```

**Translation enrichment (English missing, dialect=SpanishMexican):**
```
Translate this Mexican Spanish word to English: "Hola"

Response format:
ENGLISH: [translation]
```

**Translation enrichment (Spanish missing, dialect=SpanishMexican):**
```
Translate this English word to Mexican Spanish: "Hello"

Response format:
SPANISH: [translation]
```

**Explained enrichment (explanation missing):**
```
Explain this Mexican Spanish phrase: "órale"

Response format:
EXPLANATION: [what it means and how it's used]
```

**Exploratory enrichment (instructions missing):**
```
The user wants to practice: "Use subjunctive mood"

Provide specific instructions with examples for Mexican Spanish.

Response format:
INSTRUCTIONS: [concrete examples and usage guidance]
```

**Test Requirements:**
- Test each prompt builder produces valid prompts
- Test each parser handles valid AI responses
- Test each parser rejects malformed responses
- Test translation handles both directions correctly
- Integration test: Call learning_agent with mock data

**Success Criteria:**
- All enrichment functions compile
- Prompts are clear and structured
- Parsers are robust
- Tests pass: `cargo test -p dialect-coach-backend enrichment`

**Phase End:**
```bash
cargo test -p dialect-coach-backend
cargo clippy -p dialect-coach-backend -- -D warnings
# Remove ALL dead code warnings
# Update this document with completion status
git add backend/src/agent_service/enrichment.rs backend/src/agent_service.rs
git commit -m "Phase 2 (enrichment logic) complete"
```

**Test Results:**
- cargo test -p dialect-coach-backend: 105 tests PASSED
- cargo clippy -p dialect-coach-backend -- -D warnings: NO WARNINGS

**Deliverables Completed:**
- [x] `/Users/demouser/Code/dialect-coach/backend/src/agent_service/enrichment.rs` - All prompt builders, parsers, and enrichment functions + 12 tests
- [x] Updated `/Users/demouser/Code/dialect-coach/backend/src/agent_service.rs` - Added enrichment module and public method

**Implementation Details:**
- All prompt builder functions create structured prompts for AI agent with clear response format
- Parser functions use `extract_field()` helper to parse AI responses
- Each partial type has dedicated `enrich_partial_*()` function using learning_agent
- Mistake enrichment fills correction and category fields
- Translated enrichment handles bidirectional translation (detects which field to fill based on which is provided)
- Explained enrichment fills phrase or explanation based on what's missing
- Exploratory enrichment fills point or instructions based on what's missing
- Top-level `enrich_learning_item()` dispatcher delegates to appropriate enrichment function
- All functions follow <20 line limit with helper functions for complex logic

---

### Phase 3: HTTP Endpoint

**Boundary:** HTTP handler for enrichment API (backend main.rs + new handler only)

**Subagent:** kiss-code-generator

**Code Style Checklist:**
- [x] Functions <20 lines
- [x] Pure functions where possible
- [x] No defensive coding
- [x] Helper functions for complex logic
- [x] Full functionality (no TODOs)

**Status:** COMPLETED 2025-11-30

**Deliverables:**
1. New file: `backend/src/enrichment_handler.rs`
   - `enrich_handler(State, Json<EnrichRequest>) -> Result<Json<EnrichResponse>, StatusCode>`
   - Error handling helper: `fn map_enrich_error(e: Error) -> StatusCode`

2. Update: `backend/src/main.rs`
   - Add `mod enrichment_handler;`
   - Add route: `.route("/api/learning/enrich", post(enrichment_handler::enrich_handler))`

**Actionable Items:**
1. ✓ Create enrichment_handler.rs
2. ✓ Write `enrich_handler()` - extracts request, calls agent_service, returns response
3. ✓ Write `map_enrich_error()` - maps anyhow::Error to appropriate StatusCode
4. ✓ Add module declaration to main.rs
5. ✓ Add route to router in main.rs

**Error Handling:**
- Validation errors -> 400 Bad Request
- Agent errors -> 500 Internal Server Error
- Auth errors -> 401 Unauthorized (existing middleware)

**Test Results:**
- cargo test -p dialect-coach-backend: 162 tests PASSED
- cargo clippy -p dialect-coach-backend -- -D warnings: NO WARNINGS

**Deliverables Completed:**
- [x] `/Users/demouser/Code/dialect-coach/backend/src/enrichment_handler.rs` - Handler + error mapping + 3 tests
- [x] Updated `/Users/demouser/Code/dialect-coach/backend/src/main.rs` - Added module declaration and route

**Implementation Details:**
- `enrich_handler()` (10 lines) - Accepts EnrichRequest, calls agent service, delegates to success/error handlers
- `handle_success()` (4 lines) - Returns 200 OK with EnrichResponse
- `handle_error()` (10 lines) - Returns appropriate status code with error details
- `map_enrich_error()` (7 lines) - Maps error strings to 400 (validation/invalid) or 500 (server errors)
- All functions follow <20 line limit with helpers for complex logic
- Tests verify error mapping for validation errors, invalid data, and server errors

---

### Phase 4: Frontend Service Layer

**Boundary:** HTTP client for enrichment API (frontend services only)

**Subagent:** kiss-code-generator

**Code Style Checklist:**
- [x] Functions <20 lines
- [x] Pure functions where possible
- [x] No defensive coding
- [x] Helper functions for complex logic
- [x] Full functionality (no TODOs)

**Status:** COMPLETED 2025-11-30

**Deliverables:**
1. New file: `frontend/src/services/enrichment_service.rs`
   - `pub async fn enrich_learning_item(req: EnrichRequest) -> Result<EnrichResponse, String>`
   - Helper: `fn build_request_url() -> String`
   - Helper: `fn parse_response(text: String) -> Result<EnrichResponse, String>`

2. Update: `frontend/src/services/mod.rs`
   - Add `pub mod enrichment_service;`

3. Update: `frontend/src/app/app_state.rs`
   - Add enrichment_service to AppState
   - Initialize in AppState::new()

**Actionable Items:**
1. ✓ Create enrichment_service.rs following translation_service.rs pattern
2. ✓ Write `enrich_learning_item()` using gloo_net::http::Request
3. ✓ Error handling: Maps 400 to "Invalid learning item data", 500 to "Server error while enriching item"
4. ✓ Update services/mod.rs - Added module declaration and pub export
5. ✓ Update app_state.rs - Added enrichment_service field and initialization

**Request Flow:**
```
Frontend -> POST /api/learning/enrich
         -> Request body: EnrichRequest (JSON)
         -> Response body: EnrichResponse (JSON)
         -> Error: String (user-facing message)
```

**Error Handling:**
- Network errors -> "Failed to connect to server"
- 400 errors -> "Invalid learning item data"
- 500 errors -> "Server error while enriching item"
- Parse errors -> "Invalid response from server"

**Test Results:**
- cargo test -p dialect-coach-frontend: 43 tests PASSED
- cargo clippy -p dialect-coach-frontend -- -D warnings: NO WARNINGS

**Deliverables Completed:**
- [x] `/Users/demouser/Code/dialect-coach/frontend/src/services/enrichment_service.rs` - EnrichmentService struct + enrich_learning_item() method + 2 tests
- [x] Updated `/Users/demouser/Code/dialect-coach/frontend/src/services/mod.rs` - Added module declaration and pub export
- [x] Updated `/Users/demouser/Code/dialect-coach/frontend/src/app/app_state.rs` - Added enrichment_service field and initialized in default()

**Implementation Details:**
- EnrichmentService: Mirrors TranslationService pattern with base_url field
- enrich_learning_item(): POST to /api/learning/enrich with EnrichRequest, returns EnrichResponse
- Error handling maps HTTP status codes to user-friendly messages
- All functions under 20 lines following KISS principles
- Tests verify service creation and request serialization

---

---

### Phase 5: UI Integration - Form Changes

**Boundary:** Update learning panel form to support partial entry (frontend components only)

**Subagent:** modular-builder

**Code Style Checklist:**
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] Helper functions for complex logic
- [ ] Full functionality (no TODOs)

**Deliverables:**
1. Update: `frontend/src/components/learning_panel.rs`
   - Change validation: Allow partial data (at least ONE field filled)
   - Update validation functions: `is_mistake_valid_partial()`, `is_translation_valid_partial()`, etc.
   - Add "Enrich" button alongside "Save" button
   - Add loading state for enrichment: `use_state(|| false)`
   - Add error state: `use_state(|| Option::<String>::None)`
   - Add callback: `on_enrich` - calls enrichment service, fills missing fields
   - Update "Save" button: Only enabled when form is COMPLETE (all required fields)
   - Keep existing "Add" button for fully manual entry

**UI Flow:**
```
1. User enters partial data (e.g., just English word for translation)
2. "Enrich" button becomes enabled (at least one field filled)
3. "Save" button stays disabled (required fields still missing)
4. User clicks "Enrich"
5. Loading spinner shows
6. Backend fills missing fields
7. Form updates with enriched data
8. "Save" button now enabled
9. User can edit any field before saving
10. User clicks "Save" to add to learning items
```

**Actionable Items:**
1. Add loading state: `let enriching = use_state(|| false);`
2. Add error state: `let enrich_error = use_state(|| None::<String>);`
3. Write `is_mistake_valid_partial()` - returns true if specific_mistake is filled
4. Write `is_translation_valid_partial()` - returns true if at least one field filled (not context)
5. Write `is_explained_valid_partial()` - returns true if at least one field filled
6. Write `is_exploratory_valid_partial()` - returns true if at least one field filled
7. Write `create_partial_mistake()` - builds PartialMistake from form state
8. Write `create_partial_translated()` - builds PartialTranslated from form state
9. Write `create_partial_explained()` - builds PartialExplained from form state
10. Write `create_partial_exploratory()` - builds PartialExploratory from form state
11. Write `on_enrich` callback - calls enrichment_service, updates form fields on success
12. Write `populate_fields_from_mistake()` - helper to update form state from enriched Mistake
13. Write `populate_fields_from_translated()` - helper to update form state from enriched Translated
14. Write `populate_fields_from_explained()` - helper to update form state
15. Write `populate_fields_from_exploratory()` - helper to update form state
16. Update `render_save_cancel_buttons()` - add "Enrich" button between Save and Cancel
17. Update button enable logic: Save requires complete, Enrich requires partial
18. Add error display UI below buttons when enrich_error is Some

**Button States:**

| Form State | Enrich Button | Save Button |
|------------|---------------|-------------|
| Empty | Disabled | Disabled |
| Partial (1+ fields) | **Enabled** | Disabled |
| Complete (all required) | Disabled | **Enabled** |
| Enriching... | Disabled (loading) | Disabled |

**Test Requirements:**
- No new tests (Yew components)
- Manual testing in Phase 6

**Success Criteria:**
- Form accepts partial data
- Enrich button shows/enables correctly
- Loading state displays during enrichment
- Error state shows user-friendly messages
- Enriched data populates form fields
- User can still edit before saving
- Tests pass: `cargo test` (all crates)

**Phase End:**
```bash
cargo test
cd frontend && trunk build
cargo clippy --all -- -D warnings
# Remove ALL dead code warnings
# Update this document with completion status
git add frontend/src/components/learning_panel.rs
git commit -m "Phase 5 (UI form changes) complete"
```

---

### Phase 6: Manual Testing & Documentation

**Boundary:** End-to-end testing and user documentation

**Subagent:** None (orchestrator performs manual testing)

**Deliverables:**
1. Manual test all scenarios from user request examples
2. Update this document with test results
3. Create user-facing documentation in README or docs/

**Test Scenarios:**

**Translation:**
- [ ] User provides English "Hello" -> AI fills Spanish "Hola"
- [ ] User provides Spanish "Hola" -> AI fills English "Hello"
- [ ] User provides both -> Enrich button disabled, Save enabled immediately
- [ ] Context field stays empty when enriched
- [ ] User can manually add context after enrichment
- [ ] Dialect matches active branch dialect

**Mistake:**
- [ ] User provides incorrect "Cómo andes" -> AI fills correct "Cómo andás" + category
- [ ] User provides both incorrect and correct -> Enrich button disabled
- [ ] User tries correct-only -> Validation prevents (correct-only is invalid)
- [ ] Category is appropriate (Grammar/Dialect/Spelling/Vocabulary/Other)

**Explained:**
- [ ] User provides phrase "órale" -> AI fills explanation
- [ ] User provides explanation -> AI fills appropriate phrase
- [ ] User provides both -> Enrich disabled, Save enabled

**Exploratory:**
- [ ] User provides point "Use subjunctive" -> AI fills instructions with examples
- [ ] User provides instructions -> AI fills appropriate practice point
- [ ] User provides both -> Enrich disabled, Save enabled

**Error Cases:**
- [ ] Network error -> Shows user-friendly message
- [ ] Invalid partial data -> Shows validation error
- [ ] Server error -> Shows appropriate error message
- [ ] User can retry after error

**Edge Cases:**
- [ ] Form disabled when no active dialect (existing behavior maintained)
- [ ] Enrichment preserves manually entered data
- [ ] Cancel button still clears form
- [ ] Form clears after successful save (existing behavior maintained)

**Success Criteria:**
- All test scenarios pass
- Error messages are clear
- UI is responsive and intuitive
- No regressions to existing manual entry flow

**Phase End:**
```bash
# No code changes in this phase
# Update this document with test results
# Create user documentation if needed
git add docs/current-plans/partial_learning_items.md
git commit -m "Phase 6 (testing & documentation) complete"
```

---

## Cross-Cutting Concerns

### Security
- Enrichment endpoint requires authentication (existing middleware)
- Rate limiting applies (existing rate_limiter)
- No new security holes introduced

### Error Handling Strategy
- Backend: Use anyhow::Result for internal errors, map to StatusCode at handler
- Frontend: Convert all errors to user-friendly String messages
- Never expose internal error details to users

### Data Integrity
- Context field NEVER auto-filled (explicit requirement)
- Mistake must always have specific_mistake (the error)
- Bidirectional detection for translations ensures correct field mapping
- Validation prevents invalid partial data from reaching backend

### Performance
- Enrichment is async (doesn't block UI)
- Loading state provides user feedback
- Single round-trip to backend
- No caching needed (one-time operation per item)

---

## Dependencies

**New Crates:**
- None (uses existing dependencies)

**Existing Crates Used:**
- shared: serde, uuid (already used)
- backend: axum, anyhow (already used)
- frontend: gloo_net, yew (already used)

---

## Rollback Plan

If feature needs to be disabled:

1. Comment out route in `backend/src/main.rs`:
   ```rust
   // .route("/api/learning/enrich", post(enrichment_handler::enrich_handler))
   ```

2. Hide Enrich button in `frontend/src/components/learning_panel.rs`:
   ```rust
   const ENRICHMENT_ENABLED: bool = false;
   ```

This allows quick disable without removing code.

---

## Future Enhancements (Out of Scope)

- Batch enrichment (multiple items at once)
- Context auto-fill for translations (currently explicitly forbidden)
- Enrichment suggestions before user submits
- "Smart fill" that remembers user preferences
- Enrichment for items extracted from conversations (not manual entry)

---

## Key Constraints

1. **Context field is NEVER automatically filled for translations** (explicit requirement)
2. **Mistakes MUST include the actual error** - correction-only is invalid
3. **Use HTTP endpoint** (not WebSocket) for enrichment
4. **Keep existing "Add" button** for manual entry of complete items
5. **Bidirectional detection** for translations based on dialect
6. **Validation must prevent invalid partial data** before sending to backend

---

## Success Metrics

- [ ] Phase 1: Shared types compile and serialize correctly
- [ ] Phase 2: Enrichment logic produces valid learning items
- [ ] Phase 3: HTTP endpoint responds correctly
- [ ] Phase 4: Frontend service calls backend successfully
- [ ] Phase 5: UI allows partial entry and enrichment
- [ ] Phase 6: All manual test scenarios pass
- [ ] All tests pass: `cargo test`
- [ ] All clippy warnings resolved: `cargo clippy --all -- -D warnings`
- [ ] No dead code remains
- [ ] Git history shows 6 phase commits

---

## Notes

- Implementation follows existing patterns (translation_handler.rs, translation_service.rs)
- Type system ensures compile-time safety for partial data
- UI preserves existing manual entry flow (no regressions)
- Enrichment is optional enhancement to manual entry feature
- Agent prompts are structured for consistent parsing
- Error handling provides user-friendly messages at all layers
