# Multi-Vector Retrieval Implementation Plan

## Overview
Rewrite the RAG retrieval system to use the new `dialect_documents_v2` collection with named vectors (`content`, `context`, `keyword`) and implement multi-path search with formality-aware filtering.

## Current State
- Collection: `dialect_documents` (single unnamed vector)
- Single search path: content embedding → dialect filter → results
- Random sampling for diversity
- Formality grouping post-retrieval

## Target State
- Collection: `dialect_documents_v2` (named vectors: content, context, keyword)
- Three search paths: content, context, keyword embeddings
- No random sampling (deprecated)
- Formality filtering at query time + secondary search for diversity

---

## Phase 1: RAGConfig Update

### Subagent: kiss-code-generator

### Code Style Checklist
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No dead code
- [ ] Tests for new functions

### Files to Modify
| File | Action |
|------|--------|
| `backend/src/rag_config.rs` | Modify |

### Deliverables
- Updated `RAGConfig` struct with new fields
- Constructor and tests updated

### Implementation Details

**Replace RAGConfig struct:**
```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct RAGConfig {
    /// Content vector search WITH formality filter
    pub content_with_formality_limit: usize,
    /// Content vector search WITHOUT formality filter (diversity)
    pub content_without_formality_limit: usize,
    /// Context vector search WITH formality filter
    pub context_with_formality_limit: usize,
    /// Context vector search WITHOUT formality filter (diversity)
    pub context_without_formality_limit: usize,
    /// Keyword vector search WITH formality filter
    pub keyword_with_formality_limit: usize,
    /// Keyword vector search WITHOUT formality filter (diversity)
    pub keyword_without_formality_limit: usize,
}
```

**Default values:**
- content_with_formality_limit: 5
- content_without_formality_limit: 3
- context_with_formality_limit: 5
- context_without_formality_limit: 3
- keyword_with_formality_limit: 5
- keyword_without_formality_limit: 3

**Update constructor:**
```rust
impl RAGConfig {
    pub fn new(
        content_with_formality_limit: usize,
        content_without_formality_limit: usize,
        context_with_formality_limit: usize,
        context_without_formality_limit: usize,
        keyword_with_formality_limit: usize,
        keyword_without_formality_limit: usize,
    ) -> Self {
        Self {
            content_with_formality_limit,
            content_without_formality_limit,
            context_with_formality_limit,
            context_without_formality_limit,
            keyword_with_formality_limit,
            keyword_without_formality_limit,
        }
    }

    pub fn default_config() -> Self {
        Self::new(5, 3, 5, 3, 5, 3)
    }
}
```

### Phase Completion
- [ ] Run `cargo test -p backend` - 100% pass required
- [ ] Run `cargo clippy -p backend` - no warnings
- [ ] Update status document
- [ ] `git add -A && git commit -m "Phase 1 (RAGConfig update) complete"`

---

## Phase 2: Qdrant Service - Named Vector Search

### Subagent: modular-builder

### Code Style Checklist
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No dead code
- [ ] Tests for new functions

### Files to Modify
| File | Action |
|------|--------|
| `backend/src/qdrant_service.rs` | Modify |

### Deliverables
- Collection name updated to `dialect_documents_v2`
- Three new search methods for named vectors
- `random_dialect_samples` removed
- `search_dialect_examples` removed (replaced by specific methods)
- Helper functions for filter construction

### Implementation Details

**Update constant:**
```rust
const COLLECTION_NAME: &str = "dialect_documents_v2";
```

**Add formality string conversion helper:**
```rust
fn formality_to_filter_string(formality: &Formality) -> String {
    match formality {
        Formality::Formal => "Formal",
        Formality::ProfessionalCasual => "ProfessionalCasual",
        Formality::Informal => "Informal",
        Formality::Slang => "Slang",
    }.to_string()
}
```

**Add filter builder:**
```rust
fn build_dialect_filter(dialect: &Dialect, formality: Option<&Formality>) -> Filter {
    let mut conditions = vec![
        Condition::matches("dialect", MatchValue::Keyword(dialect.id().to_string()))
    ];

    if let Some(f) = formality {
        conditions.push(Condition::matches(
            "formality",
            MatchValue::Keyword(formality_to_filter_string(f))
        ));
    }

    Filter::must(conditions)
}
```

**New search methods (use Query API with `.using()`):**

```rust
/// Search by content vector
pub async fn search_by_content(
    &self,
    embedding: &[f32],
    dialect: &Dialect,
    formality: Option<&Formality>,
    limit: usize,
) -> Result<Vec<(DialectDocument, f32)>>

/// Search by context vector
pub async fn search_by_context(
    &self,
    embedding: &[f32],
    dialect: &Dialect,
    formality: Option<&Formality>,
    limit: usize,
) -> Result<Vec<(DialectDocument, f32)>>

/// Search by keyword vector (multiple embeddings, returns combined results)
pub async fn search_by_keywords(
    &self,
    keyword_embeddings: &[Vec<f32>],
    dialect: &Dialect,
    formality: Option<&Formality>,
    limit_per_keyword: usize,
) -> Result<Vec<(DialectDocument, f32)>>
```

**Internal helper for named vector search:**
```rust
async fn search_named_vector(
    &self,
    vector_name: &str,
    embedding: &[f32],
    filter: Filter,
    limit: usize,
) -> Result<Vec<(DialectDocument, f32)>>
```

This uses `QueryPointsBuilder::new(COLLECTION_NAME).query(embedding).using(vector_name).filter(filter).limit(limit)`.

**Remove:**
- `random_dialect_samples()` method
- `search_dialect_examples()` method (callers will use specific methods)

### Phase Completion
- [ ] Run `cargo test -p backend` - 100% pass required (expect compilation errors from callers - fix in next phase)
- [ ] Run `cargo clippy -p backend` - no warnings
- [ ] Update status document
- [ ] `git add -A && git commit -m "Phase 2 (Qdrant named vector search) complete"`

---

## Phase 3: Keyword Extraction Service

### Subagent: modular-builder

### Code Style Checklist
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No dead code
- [ ] Tests for new functions

### Files to Create/Modify
| File | Action |
|------|--------|
| `backend/src/agent_service/keyword_extraction.rs` | Create |
| `backend/src/agent_service.rs` | Modify (add module declaration) |

### Deliverables
- New `KeywordExtractor` struct
- GPT-5-nano integration for keyword extraction
- Sorted keyword output (unicode order)

### Implementation Details

**New file: `backend/src/agent_service/keyword_extraction.rs`**

```rust
use anyhow::Result;
use dialect_coach_shared::Dialect;
use rig::prelude::*;
use rig::providers::openai::{
    self,
    responses_api::{AdditionalParameters, Reasoning, ReasoningEffort},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Deserialize, Serialize, Debug, Clone)]
pub struct ExtractedKeywords {
    pub keywords: Vec<String>,
}

pub struct KeywordExtractor {
    client: openai::Client,
}

impl KeywordExtractor {
    pub fn new() -> Result<Self> {
        let client = openai::Client::from_env();
        Ok(Self { client })
    }

    /// Extract keywords from text using GPT-5-nano
    /// Returns dictionary forms of: nouns, verbs, slang, proper nouns,
    /// unusual adverbs/adjectives - sorted by unicode order
    pub async fn extract(&self, text: &str, dialect: Dialect) -> Result<Vec<String>> {
        // ... implementation following corpus-processor/llm.rs pattern
    }
}
```

**Prompt for keyword extraction:**
```
Role: Extract searchable keywords from user input for {dialect_name} dialect retrieval.

Task: Extract ALL substantive vocabulary terms in their dictionary form (lemma).

Include:
- Nouns (dictionary/singular form)
- Verbs (dictionary/infinitive form)
- Slang terms
- Proper nouns
- Unusual adjectives/adverbs

Exclude:
- Articles, particles, conjunctions, prepositions
- Common adjectives/adverbs (very, too, really, etc.)

Input: "{text}"

Return keywords sorted alphabetically.
```

**Add to agent_service.rs:**
```rust
pub mod keyword_extraction;
```

### Phase Completion
- [ ] Run `cargo test -p backend` - 100% pass required
- [ ] Run `cargo clippy -p backend` - no warnings
- [ ] Update status document
- [ ] `git add -A && git commit -m "Phase 3 (keyword extraction service) complete"`

---

## Phase 4: Update Examples Module

### Subagent: kiss-code-generator

### Code Style Checklist
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No dead code
- [ ] Tests for new functions

### Files to Modify
| File | Action |
|------|--------|
| `backend/src/agent_service/response/examples.rs` | Modify |

### Deliverables
- Remove `get_sample_formalities()` (no longer needed)
- Add `merge_search_results()` function
- Add `deduplicate_excluding()` function
- Update existing functions as needed

### Implementation Details

**Remove:**
- `get_sample_formalities()` function and its tests

**Add merge function:**
```rust
/// Merge results from multiple search sources, deduplicating by content
/// Each search source already has its own limit from RAGConfig
pub fn merge_search_results(
    results: Vec<Vec<(DialectDocument, f32)>>,
) -> Vec<DialectDocument> {
    let mut seen = std::collections::HashSet::new();
    let mut merged = Vec::new();

    for result_set in results {
        for (doc, _score) in result_set {
            if seen.insert(doc.content.clone()) {
                merged.push(doc);
            }
        }
    }

    merged
}
```

**Add exclusion-based deduplication:**
```rust
/// Deduplicate results, excluding content already seen in primary results
/// Each search source already has its own limit from RAGConfig
pub fn deduplicate_excluding(
    results: Vec<Vec<(DialectDocument, f32)>>,
    exclude: &[DialectDocument],
) -> Vec<DialectDocument> {
    let excluded_content: std::collections::HashSet<_> =
        exclude.iter().map(|d| d.content.clone()).collect();

    let mut seen = excluded_content;
    let mut deduped = Vec::new();

    for result_set in results {
        for (doc, _score) in result_set {
            if seen.insert(doc.content.clone()) {
                deduped.push(doc);
            }
        }
    }

    deduped
}
```

**Update `deduplicate_examples`:**
Simplify to work with the new flow (may just remove if no longer needed).

### Phase Completion
- [ ] Run `cargo test -p backend` - 100% pass required
- [ ] Run `cargo clippy -p backend` - no warnings
- [ ] Update status document
- [ ] `git add -A && git commit -m "Phase 4 (examples module update) complete"`

---

## Phase 5: Update Retrieval Module

### Subagent: modular-builder

### Code Style Checklist
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No dead code
- [ ] Tests for new functions

### Files to Modify
| File | Action |
|------|--------|
| `backend/src/agent_service/response/retrieval.rs` | Modify |
| `backend/src/agent_service/response/mod.rs` | Modify |

### Deliverables
- Add `KeywordExtractor` to `ResponseContext`
- Rewrite `collect_examples` for multi-path retrieval
- Remove `retrieve_random_samples`
- Update helper functions

### Implementation Details

**Update ResponseContext (mod.rs):**
```rust
use super::keyword_extraction::KeywordExtractor;

pub struct ResponseContext {
    pub response_agent: Arc<dyn CompletionAgent>,
    pub learning_agent: Arc<dyn CompletionAgent>,
    pub qdrant: Arc<QdrantService>,
    pub embeddings: Arc<EmbeddingService>,
    pub keyword_extractor: Arc<KeywordExtractor>,
}
```

**Update retrieval.rs - New `collect_examples`:**
```rust
pub(super) async fn collect_examples(
    &self,
    user_message: &str,
    conversation_history: &[RigMessage],
    dialect: DialectWithFeatures,
    formality: Formality,
    rag_config: &RAGConfig,
) -> Result<(Vec<DialectDocument>, Vec<DialectDocument>)> {
    if !dialect.has_corpus {
        return Ok((Vec::new(), Vec::new()));
    }

    // 1. Generate content embedding
    let content_embedding = self.embeddings.embed_text(user_message)?;

    // 2. Generate context embedding from conversation history
    let context_text = build_context_string(conversation_history);
    let context_embedding = self.embeddings.embed_text(&context_text)?;

    // 3. Extract keywords and generate embeddings
    let keywords = self.keyword_extractor
        .extract(user_message, dialect.dialect)
        .await
        .unwrap_or_default();
    let keyword_embeddings = if !keywords.is_empty() {
        self.embeddings.embed_batch(keywords)?
    } else {
        Vec::new()
    };

    // 4. Primary search (with formality filter) - parallel
    let (content_with_form, context_with_form, keyword_with_form) = tokio::join!(
        self.qdrant.search_by_content(
            &content_embedding,
            &dialect.dialect,
            Some(&formality),
            rag_config.content_with_formality_limit
        ),
        self.qdrant.search_by_context(
            &context_embedding,
            &dialect.dialect,
            Some(&formality),
            rag_config.context_with_formality_limit
        ),
        self.qdrant.search_by_keywords(
            &keyword_embeddings,
            &dialect.dialect,
            Some(&formality),
            rag_config.keyword_with_formality_limit
        ),
    );

    // 5. Merge primary results (with formality)
    let primary = merge_search_results(vec![
        content_with_form?,
        context_with_form?,
        keyword_with_form?,
    ]);

    // 6. Secondary search (no formality filter) - parallel
    let (content_without_form, context_without_form, keyword_without_form) = tokio::join!(
        self.qdrant.search_by_content(
            &content_embedding,
            &dialect.dialect,
            None,
            rag_config.content_without_formality_limit
        ),
        self.qdrant.search_by_context(
            &context_embedding,
            &dialect.dialect,
            None,
            rag_config.context_without_formality_limit
        ),
        self.qdrant.search_by_keywords(
            &keyword_embeddings,
            &dialect.dialect,
            None,
            rag_config.keyword_without_formality_limit
        ),
    );

    // 7. Merge secondary, excluding primary content
    let secondary = deduplicate_excluding(
        vec![
            content_without_form?,
            context_without_form?,
            keyword_without_form?,
        ],
        &primary,
    );

    Ok((primary, secondary))
}
```

**Add context string builder:**
```rust
fn build_context_string(conversation_history: &[RigMessage]) -> String {
    conversation_history
        .iter()
        .take(3)
        .map(get_message_text)
        .collect::<Vec<_>>()
        .join(" ")
}
```

**Remove:**
- `retrieve_random_samples()`
- `retrieve_all_rag_examples()` (replaced by direct calls)
- Unused helper methods

### Phase Completion
- [ ] Run `cargo test -p backend` - 100% pass required
- [ ] Run `cargo clippy -p backend` - no warnings
- [ ] Update status document
- [ ] `git add -A && git commit -m "Phase 5 (retrieval module rewrite) complete"`

---

## Phase 6: Update Callers and Integration

### Subagent: modular-builder

### Code Style Checklist
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No dead code
- [ ] Tests for new functions

### Files to Modify
| File | Action |
|------|--------|
| `backend/src/agent_service.rs` | Modify |
| Any files that construct `ResponseContext` | Modify |
| Any files that construct `RAGConfig` | Modify |

### Deliverables
- All callers updated to new APIs
- `KeywordExtractor` initialized in `AgentService`
- `RAGConfig` construction updated throughout codebase

### Implementation Details

**Update AgentService:**
```rust
pub struct AgentService {
    pub response_agent: Arc<dyn CompletionAgent>,
    pub learning_agent: Arc<dyn CompletionAgent>,
    pub analysis_agent: Arc<dyn CompletionAgent>,
    pub planning_agent: Arc<dyn CompletionAgent>,
    pub planning_config: ProviderAgentConfig,
    pub qdrant: Arc<QdrantService>,
    pub embeddings: Arc<EmbeddingService>,
    pub keyword_extractor: Arc<KeywordExtractor>,  // NEW
}
```

**Update AgentService::from_env:**
```rust
let keyword_extractor = KeywordExtractor::new()?;
// ...
Ok(Self {
    // ... existing fields
    keyword_extractor: Arc::new(keyword_extractor),
})
```

**Update ResponseContext creation in generate_response:**
```rust
let ctx = ResponseContext {
    response_agent: self.response_agent.clone(),
    learning_agent: self.learning_agent.clone(),
    qdrant: self.qdrant.clone(),
    embeddings: self.embeddings.clone(),
    keyword_extractor: self.keyword_extractor.clone(),
};
```

**Search for and update all RAGConfig::new calls** to use new signature or `RAGConfig::default_config()`.

### Phase Completion
- [ ] Run `cargo test -p backend` - 100% pass required
- [ ] Run `cargo clippy -p backend` - no warnings
- [ ] Update status document
- [ ] `git add -A && git commit -m "Phase 6 (caller updates and integration) complete"`

---

## Phase 7: Final Cleanup and Testing

### Subagent: kiss-code-generator

### Code Style Checklist
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No dead code
- [ ] Tests for new functions

### Files to Modify
| File | Action |
|------|--------|
| All modified files | Review for dead code |
| Test files | Update/add tests |

### Deliverables
- All dead code removed
- All tests passing
- Integration verified

### Tasks
1. Run `cargo clippy -p backend` and fix ALL warnings
2. Search for any remaining references to:
   - `random_dialect_samples`
   - `search_dialect_examples`
   - `get_sample_formalities`
   - `num_random_documents`
   - `num_conversation_documents`
   - `content_limit` (old field name)
   - `context_limit` (old field name)
   - `keyword_limit` (old field name)
   - `primary_formality_limit` (old field name)
   - `secondary_formality_limit` (old field name)
3. Remove any dead code found
4. Add integration test for multi-vector retrieval (if not already covered)
5. Verify all existing tests pass with new implementation

### Phase Completion
- [ ] Run `cargo test -p backend` - 100% pass required
- [ ] Run `cargo clippy -p backend` - no warnings
- [ ] Run `cargo build -p backend` - successful
- [ ] Update status document
- [ ] `git add -A && git commit -m "Phase 7 (final cleanup) complete"`

---

## Status Document

### Phase Status
| Phase | Description | Status |
|-------|-------------|--------|
| 1 | RAGConfig Update | ✅ Complete |
| 2 | Qdrant Named Vector Search | ✅ Complete |
| 3 | Keyword Extraction Service | ✅ Complete |
| 4 | Examples Module Update | Not Started |
| 5 | Retrieval Module Rewrite | Not Started |
| 6 | Caller Updates and Integration | Not Started |
| 7 | Final Cleanup and Testing | Not Started |

### Agreements Made
- User specified: cartesian product config (6 fields: content/context/keyword × with-formality/without-formality) - 2026-01-06
- User specified: no weighting, just samples from each source - 2026-01-06
- User specified: deprecate random samples - 2026-01-06
- User specified: prefer same-formality data, supplement with general data - 2026-01-06
- User specified: new client for GPT-5-nano using existing patterns - 2026-01-06
- User specified: default limits (5 with formality, 3 without formality for each vector type) - 2026-01-06

### Explicitly Rejected
- Weighting strategy for merging results (use simple limits instead)
- Keeping random_dialect_samples (deprecated)
- Two-level config approach with separate type and formality limits (confusing, unclear how to split limits) - 2026-01-06

### Issues Encountered
(To be updated during implementation)
