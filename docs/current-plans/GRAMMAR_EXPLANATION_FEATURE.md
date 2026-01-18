# Plan: Explain Grammar Feature + Selection Caching

## Overview

Add an "Explain Grammar" button alongside the existing "Translate" button when text is selected. Both features will be cached using exact text matching (context sentence excluded from cache key).

## Design Decisions (User Confirmed)

1. **Learning item type:** Reuse existing `Explained` type - `new_phrase` = grammar element, `explanation` = grammar explanation
2. **Button layout:** Side-by-side buttons ("Translate" and "Grammar")
3. **Modal design:** Same modal style for both features - consistent UX, less code

---

## Key Findings

### Existing Types
- **`Explained`** (`shared/src/models/agent.rs:104-151`): `new_phrase` and `explanation` fields - will reuse
- **`PhraseTranslation`** (`shared/src/models/message.rs:77-80`): `target_text` and `english` fields

### Current Translation Flow
- Frontend: `TranslateSelectionButton` → `MainContent.on_selection_translate_click` → `TranslationService`
- Backend: `POST /api/translate` → `translate_handler` → AI prompt → JSON response

### TTS Caching Pattern
- In-memory LRU cache in `backend/src/tts_service.rs`
- Key: `tts:{hash}` using `std::hash::Hasher`
- 100 entries per instance

---

## Cache Key Design

```rust
pub fn generate_selection_cache_key(operation: &str, dialect: &Dialect, text: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    dialect.id().hash(&mut hasher);

    format!("{}:{}:{:x}", operation, dialect.id(), hasher.finish())
}
```

**Context sentence is NOT in cache key** - allows reusing results when same phrase appears in different sentences.

---

## Phase 1: Backend - Cache Infrastructure for Translation

### Code Style Checklist
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No dead code - cache is immediately used by existing translation

### Subagent: `kiss-code-generator`
Rationale: Single focused module with clear spec, then integration into existing handler.

### Deliverables

**1. Create `backend/src/selection_cache.rs`:**
```rust
use dialect_coach_shared::models::PhraseTranslation;
use lru::LruCache;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct SelectionCache {
    cache: Arc<Mutex<LruCache<String, Vec<PhraseTranslation>>>>,
}

impl SelectionCache {
    pub fn new(capacity: usize) -> Self { ... }

    pub fn generate_key(operation: &str, dialect_id: &str, text: &str) -> String { ... }

    pub async fn get_translation(&self, key: &str) -> Option<Vec<PhraseTranslation>> { ... }

    pub async fn set_translation(&self, key: &str, value: Vec<PhraseTranslation>) { ... }
}
```
- Capacity: 100 entries
- Key format: `translate:{dialect_id}:{hash}`

**2. Modify `backend/src/lib.rs`:**
- Add `pub mod selection_cache;`

**3. Modify `backend/src/startup.rs`:**
- Create `SelectionCache` instance in `create_app()`
- Pass cache to `translate_handler` via state or extension

**4. Modify `backend/src/translation_handler.rs`:**
- Accept `SelectionCache` (via State or Extension)
- In `translate_handler`:
  - Generate cache key from (dialect, phrase)
  - Check cache before AI call
  - If hit: return cached result, log "Cache hit"
  - If miss: call AI, store result, return
- Add tests for cache key generation

### Files Summary
| File | Action |
|------|--------|
| `backend/src/selection_cache.rs` | Create |
| `backend/src/lib.rs` | Modify - export module |
| `backend/src/startup.rs` | Modify - init cache, pass to handler |
| `backend/src/translation_handler.rs` | Modify - use cache |

### Phase 1 End Verification
```bash
cargo check
cargo test
cargo clippy                   # No warnings, no dead code
```

**Manual test:**
```bash
# First call - cache miss
curl -X POST http://localhost:3000/api/translate \
  -H "Content-Type: application/json" \
  -d '{"phrase":"hola","context":"hola amigo","dialect":"spanish_mexican"}'
# Check logs: "Cache miss"

# Second call - cache hit
curl -X POST http://localhost:3000/api/translate \
  -H "Content-Type: application/json" \
  -d '{"phrase":"hola","context":"different context here","dialect":"spanish_mexican"}'
# Check logs: "Cache hit" (same phrase, different context)
```

**After verification:** `git add . && git commit -m "Phase 1 (translation caching) complete"`

---

## Phase 2: Backend - Grammar Endpoint

### Code Style Checklist
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No dead code - types used by handler, handler registered and callable

### Subagent: `kiss-code-generator`
Rationale: Clear parallel to translation_handler, straightforward implementation.

### Deliverables

**1. Add shared types to `shared/src/models/message.rs`:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrammarRequest {
    pub phrase: String,
    pub context: String,
    pub dialect: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrammarResponse {
    pub original_phrase: String,
    pub explanations: Vec<GrammarExplanation>,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrammarExplanation {
    pub element: String,
    pub explanation: String,
}
```

**2. Extend `backend/src/selection_cache.rs`:**
```rust
// Add to existing cache struct or create enum
pub enum CachedResult {
    Translation(Vec<PhraseTranslation>),
    Grammar(Vec<GrammarExplanation>),
}

// Add methods:
pub async fn get_grammar(&self, key: &str) -> Option<Vec<GrammarExplanation>> { ... }
pub async fn set_grammar(&self, key: &str, value: Vec<GrammarExplanation>) { ... }
```

**3. Create `backend/src/grammar_handler.rs`:**
```rust
pub async fn grammar_handler(
    State(state): State<AppState>,
    Json(request): Json<GrammarRequest>,
) -> impl IntoResponse {
    // 1. Parse dialect
    // 2. Generate cache key: "grammar:{dialect}:{hash(phrase)}"
    // 3. Check cache - if hit, return cached
    // 4. Build AI prompt:
    //    "Explain the grammar of "{phrase}" in this {dialect} sentence: "{context}"
    //     For each grammatical element, provide element name and explanation.
    //     Return JSON: [{"element": "...", "explanation": "..."}]"
    // 5. Call AI via state.agent.generate_simple_response()
    // 6. Parse response as Vec<GrammarExplanation>
    // 7. Store in cache
    // 8. Return GrammarResponse
}
```

**4. Modify `backend/src/lib.rs`:**
- Add `pub mod grammar_handler;`

**5. Modify `backend/src/startup.rs`:**
- Add route: `.route("/api/grammar", post(grammar_handler::grammar_handler))`

### Files Summary
| File | Action |
|------|--------|
| `shared/src/models/message.rs` | Modify - add 3 types |
| `backend/src/selection_cache.rs` | Modify - add grammar methods |
| `backend/src/grammar_handler.rs` | Create |
| `backend/src/lib.rs` | Modify - export handler |
| `backend/src/startup.rs` | Modify - add route |

### Phase 2 End Verification
```bash
cargo check
cargo test
cargo clippy
```

**Manual test:**
```bash
curl -X POST http://localhost:3000/api/grammar \
  -H "Content-Type: application/json" \
  -d '{"phrase":"が","context":"私が食べる","dialect":"japanese_standard"}'
# Should return grammar explanations

# Test caching:
curl -X POST http://localhost:3000/api/grammar \
  -H "Content-Type: application/json" \
  -d '{"phrase":"が","context":"彼が走る","dialect":"japanese_standard"}'
# Check logs: "Cache hit" (same phrase, different context)
```

**After verification:** `git add . && git commit -m "Phase 2 (grammar endpoint) complete"`

---

## Phase 3: Frontend - Generalize Modal

### Code Style Checklist
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No dead code - existing translation flow continues to use modal

### Subagent: `kiss-code-generator`
Rationale: Single file modification with clear backwards-compatible changes.

### Deliverables

**1. Modify `frontend/src/components/translation_modal.rs`:**

Change props to accept either result type:
```rust
#[derive(Clone, PartialEq)]
pub enum SelectionResult {
    Translation(Vec<PhraseTranslation>),
    Grammar(Vec<GrammarExplanation>),
}

#[derive(Properties, PartialEq)]
pub struct SelectionModalProps {
    pub original_text: String,
    pub result: Option<SelectionResult>,  // None = loading
    pub on_close: Callback<()>,
    pub on_save: Callback<(String, String, String)>,  // (element, explanation, context)
}
```

- Rename component to `SelectionModal` (update mod.rs export)
- `render_content` matches on `SelectionResult` variant
- Both variants render same layout: left text → right text, save button
- For Translation: `target_text → english`
- For Grammar: `element → explanation`

**2. Modify `frontend/src/components/mod.rs`:**
- Update export if component renamed

**3. Modify `frontend/src/components/main_content.rs`:**
- Update modal usage to use new props structure
- Keep translation flow working with `SelectionResult::Translation`

### Files Summary
| File | Action |
|------|--------|
| `frontend/src/components/translation_modal.rs` | Modify - generalize |
| `frontend/src/components/mod.rs` | Modify - update export |
| `frontend/src/components/main_content.rs` | Modify - use new props |

### Phase 3 End Verification
```bash
cd frontend && trunk build
cargo test
cargo clippy
```

**Manual test:**
1. Start backend and frontend
2. Select text in a message
3. Click "Translate" button
4. Verify translation modal still works exactly as before
5. Verify save still creates learning item

**After verification:** `git add . && git commit -m "Phase 3 (modal generalization) complete"`

---

## Phase 4: Frontend - Grammar Service, Button, and Wiring

### Code Style Checklist
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No dead code - service called, button triggers it, modal displays results

### Subagent: `modular-builder`
Rationale: Multiple interconnected files with callback threading.

### Deliverables

**1. Create `frontend/src/services/grammar.rs`:**
```rust
pub struct GrammarService {
    base_url: String,
}

impl GrammarService {
    pub fn new(base_url: String) -> Self { ... }

    pub async fn explain_grammar(
        &self,
        phrase: &str,
        context: &str,
        dialect: &Dialect,
    ) -> Result<Vec<GrammarExplanation>, String> {
        // POST to /api/grammar
        // Parse GrammarResponse
        // Return explanations or error
    }
}
```

**2. Modify `frontend/src/services/mod.rs`:**
- Add `pub mod grammar;`
- Export `GrammarService`

**3. Modify `frontend/src/components/translate_selection_button.rs`:**
```rust
#[derive(Clone, PartialEq)]
pub enum SelectionAction {
    Translate,
    ExplainGrammar,
}

#[derive(Properties, PartialEq)]
pub struct Props {
    pub selected_text: String,
    pub message_context: String,
    pub position: (f64, f64),
    pub on_action: Callback<(SelectionAction, String, String)>,  // (action, text, context)
    #[prop_or(false)]
    pub is_loading: bool,
}
```
- Render TWO buttons side-by-side: "Translate" and "Grammar"
- Each button emits appropriate `SelectionAction`

**4. Modify `frontend/src/components/message_bubble.rs`:**
- Update callback prop type to match new signature
- Pass action through to parent

**5. Modify `frontend/src/components/chat_window.rs`:**
- Update callback threading

**6. Modify `frontend/src/app/app_state/app.rs`:**
- Add `grammar_service: GrammarService` to `AppState`
- Initialize in `AppState::default()`

**7. Modify `frontend/src/components/main_content.rs`:**
- Add state for which action type is active
- Handle `SelectionAction::ExplainGrammar`:
  - Call `grammar_service.explain_grammar()`
  - Open modal with `SelectionResult::Grammar`
- Update save handler to create `Explained` learning item for grammar

### Files Summary
| File | Action |
|------|--------|
| `frontend/src/services/grammar.rs` | Create |
| `frontend/src/services/mod.rs` | Modify - export |
| `frontend/src/components/translate_selection_button.rs` | Modify - dual buttons |
| `frontend/src/components/message_bubble.rs` | Modify - callback type |
| `frontend/src/components/chat_window.rs` | Modify - callback threading |
| `frontend/src/app/app_state/app.rs` | Modify - add service |
| `frontend/src/components/main_content.rs` | Modify - grammar handler |

### Phase 4 End Verification
```bash
cd frontend && trunk build
cargo test
cargo clippy
```

**Full E2E Test:**
1. Start backend: `cd backend && cargo run`
2. Start frontend: `cd frontend && trunk serve`
3. Open app, start a conversation
4. Select text in a message
5. Verify BOTH "Translate" and "Grammar" buttons appear side-by-side
6. Click "Translate" → modal shows translations → save works → creates `Translated` item
7. Click "Grammar" → modal shows grammar explanations → save works → creates `Explained` item
8. Select same phrase in different message → check backend logs for "Cache hit"
9. Verify learning items appear in sidebar with correct icons (📝 for Translation, 💡 for Explanation)

**After verification:** `git add . && git commit -m "Phase 4 (grammar UI integration) complete"`

---

## Summary

| Phase | Scope | Testable Deliverable | Dead Code? |
|-------|-------|---------------------|------------|
| 1 | Backend cache + translation integration | Translation caching works | No - cache used by translation |
| 2 | Backend grammar types + handler | Grammar endpoint callable | No - types used by handler, handler registered |
| 3 | Frontend modal generalization | Translation still works with new modal | No - translation uses it |
| 4 | Frontend grammar service + buttons + wiring | Full grammar feature works | No - all connected |

Each phase produces a testable, working increment with no dead code.
