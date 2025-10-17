# FastEmbed Tutorial: Fast Text Embeddings in Rust

## Overview

**fastembed** is a Rust library for generating text embeddings using ONNX Runtime. It provides fast, lightweight embedding models that run locally without external API calls, supporting multiple languages.

**Version used in dialect-coach:** `4.2`

**Why we use it:** Local embedding generation for RAG applications, fast inference, multilingual support, no API costs.

## Dependencies

```toml
[dependencies]
fastembed = "4.2"
```

## Core Concepts

### 1. Embedding Models

FastEmbed supports various embedding models:

- **MultilingualE5Base** - 768 dimensions, supports 100+ languages
- **MultilingualE5Small** - 384 dimensions, smaller/faster
- **BGEBaseEN** - 768 dimensions, optimized for English
- **AllMiniLML6V2** - 384 dimensions, small and fast

## Choosing the Right Model

**Decision tree:**

```
Do you need non-English support?
├─ Yes → Use MultilingualE5Base or MultilingualE5Small
│         ├─ Need best quality? → MultilingualE5Base (768 dims)
│         └─ Need speed/small size? → MultilingualE5Small (384 dims)
│
└─ No (English only) → Use BGEBaseEN or AllMiniLML6V2
          ├─ Need best quality? → BGEBaseEN (768 dims)
          └─ Need speed/small size? → AllMiniLML6V2 (384 dims)
```

**Comparison table:**

| Model | Dimensions | Languages | Model Size | Speed | Best For |
|-------|------------|-----------|------------|-------|----------|
| **MultilingualE5Base** | 768 | 100+ | ~220MB | Medium | Multi-language, high quality |
| **MultilingualE5Small** | 384 | 100+ | ~120MB | Fast | Multi-language, resource-constrained |
| **BGEBaseEN** | 768 | English | ~130MB | Medium | English-only, high quality |
| **AllMiniLML6V2** | 384 | English | ~90MB | Fastest | English-only, speed critical |

**Why dialect-coach uses MultilingualE5Base:**
1. Supports multiple languages (Spanish, Arabic, French)
2. Higher quality embeddings (768 dimensions)
3. Better semantic understanding for dialect nuances
4. Model size acceptable for server deployment

**When to choose smaller models:**
- Mobile/edge deployment
- Real-time processing requirements
- Memory constraints
- Only need keyword-level matching (not deep semantic understanding)

### 2. Text Embedding

Convert text to dense vector representations:

```rust
use fastembed::{TextEmbedding, InitOptions, EmbeddingModel};

let model = TextEmbedding::try_new(
    InitOptions::new(EmbeddingModel::MultilingualE5Base)
)?;

let embedding = model.embed(vec!["Hello, world!".to_string()], None)?;
// Returns Vec<Vec<f32>> - one vector per input string
```

### 3. Dimensions

Each model outputs fixed-dimension vectors:

```rust
let embedding = model.embed(vec!["text".to_string()], None)?;
println!("Dimensions: {}", embedding[0].len());  // 768 for MultilingualE5Base
```

## Step-by-Step: Building a Semantic Search Engine

### Step 1: Initialize Model

```rust
use fastembed::{TextEmbedding, InitOptions, EmbeddingModel};
use anyhow::Result;

fn create_embedder() -> Result<TextEmbedding> {
    let model = TextEmbedding::try_new(
        InitOptions::new(EmbeddingModel::MultilingualE5Base)
            .with_show_download_progress(true)
    )?;

    println!("Model initialized successfully!");
    Ok(model)
}
```

**Note:** First run downloads model (~220MB for MultilingualE5Base), subsequent runs use cached model.

### Step 2: Generate Single Embedding

```rust
fn embed_text(model: &TextEmbedding, text: &str) -> Result<Vec<f32>> {
    let embeddings = model.embed(vec![text.to_string()], None)?;

    embeddings
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("No embedding generated"))
}

// Usage
let embedding = embed_text(&model, "The quick brown fox")?;
println!("Generated {} dimensions", embedding.len());
```

### Step 3: Batch Embeddings

```rust
fn embed_batch(model: &TextEmbedding, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
    let embeddings = model.embed(texts, None)?;
    Ok(embeddings)
}

// Usage
let documents = vec![
    "Rust is a systems programming language".to_string(),
    "Python is great for data science".to_string(),
    "JavaScript runs in browsers".to_string(),
];

let embeddings = embed_batch(&model, documents)?;
println!("Generated {} embeddings", embeddings.len());
```

## Understanding Similarity Metrics

**What is a similarity metric?** A way to measure how "close" two vectors are in semantic space.

**Cosine Similarity** - Measures the angle between vectors:
```
similarity = (A · B) / (||A|| × ||B||)
```

**Range:** -1 to 1 (but usually 0 to 1 for embeddings)
- 1.0 = Identical meaning
- 0.5 = Somewhat related
- 0.0 = Unrelated

**Why cosine?** It's direction-based, not magnitude-based:
- "cat" and "kitten" point in similar directions → high similarity
- Vector length doesn't matter, only angle
- Works well for text where we care about meaning, not word count

**Visual example:**
```
Vector Space (simplified to 2D):

       cat •           kitten •
           \         /
            \       /
             \     /
              \   /
               \ /
                •───────────• car
           (origin)
```
"cat" and "kitten" have small angle → high cosine similarity
"cat" and "car" have large angle → low cosine similarity

**Other metrics (not used in dialect-coach):**
- **Euclidean distance** - Straight-line distance, affected by magnitude
- **Dot product** - Raw similarity without normalization

### Step 4: Semantic Similarity

```rust
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    dot_product / (norm_a * norm_b)
}

fn find_most_similar(
    query_embedding: &[f32],
    document_embeddings: &[Vec<f32>],
    documents: &[String],
) -> Vec<(usize, f32, String)> {
    let mut similarities: Vec<_> = document_embeddings
        .iter()
        .enumerate()
        .map(|(idx, doc_emb)| {
            let similarity = cosine_similarity(query_embedding, doc_emb);
            (idx, similarity, documents[idx].clone())
        })
        .collect();

    similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    similarities
}

// Usage
let query = "programming languages";
let query_emb = embed_text(&model, query)?;

let results = find_most_similar(&query_emb, &embeddings, &documents);
for (idx, score, text) in results.iter().take(3) {
    println!("Score: {:.4} - {}", score, text);
}
```

### Step 5: Complete Search System

```rust
struct SearchEngine {
    model: TextEmbedding,
    documents: Vec<String>,
    embeddings: Vec<Vec<f32>>,
}

impl SearchEngine {
    fn new() -> Result<Self> {
        let model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::MultilingualE5Base)
                .with_show_download_progress(true)
        )?;

        Ok(Self {
            model,
            documents: Vec::new(),
            embeddings: Vec::new(),
        })
    }

    fn add_documents(&mut self, docs: Vec<String>) -> Result<()> {
        println!("Embedding {} documents...", docs.len());

        let new_embeddings = self.model.embed(docs.clone(), None)?;

        self.documents.extend(docs);
        self.embeddings.extend(new_embeddings);

        Ok(())
    }

    fn search(&self, query: &str, top_k: usize) -> Result<Vec<(f32, String)>> {
        let query_embedding = self.model
            .embed(vec![query.to_string()], None)?
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("Failed to embed query"))?;

        let mut results: Vec<_> = self.embeddings
            .iter()
            .zip(self.documents.iter())
            .map(|(doc_emb, doc)| {
                let score = cosine_similarity(&query_embedding, doc_emb);
                (score, doc.clone())
            })
            .collect();

        results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        results.truncate(top_k);

        Ok(results)
    }
}

// Usage
fn main() -> Result<()> {
    let mut engine = SearchEngine::new()?;

    engine.add_documents(vec![
        "Rust provides memory safety without garbage collection".to_string(),
        "Machine learning models require large datasets".to_string(),
        "Web development uses HTML, CSS, and JavaScript".to_string(),
    ])?;

    let results = engine.search("safe programming languages", 2)?;

    for (score, doc) in results {
        println!("{:.4}: {}", score, doc);
    }

    Ok(())
}
```

## How dialect-coach Uses FastEmbed

### 1. Service Initialization (embedding_service.rs:10-23)

Creating embedding service:

```rust
pub struct EmbeddingService {
    model: TextEmbedding,
}

impl EmbeddingService {
    pub fn new() -> Result<Self> {
        tracing::info!("Initializing Fastembed model (MultilingualE5Base)...");

        let model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::MultilingualE5Base)
                .with_show_download_progress(true),
        )
        .context("Failed to initialize Fastembed model")?;

        tracing::info!("Fastembed model initialized successfully");

        Ok(Self { model })
    }
}
```

**Pattern:** Wrap model in service struct, show download progress, log initialization.

### 2. Single Text Embedding (embedding_service.rs:26-36)

Generate embedding for one text:

```rust
pub fn embed_text(&self, text: &str) -> Result<Vec<f32>> {
    let embeddings = self
        .model
        .embed(vec![text.to_string()], None)
        .context("Failed to generate embedding")?;

    embeddings
        .into_iter()
        .next()
        .context("No embedding generated")
}
```

**Pattern:** Take first (and only) embedding from result. Error handling with context.

### 3. Batch Embedding (embedding_service.rs:39-43)

Process multiple texts efficiently:

```rust
pub fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
    self.model
        .embed(texts, None)
        .context("Failed to generate batch embeddings")
}
```

**Pattern:** Direct passthrough to model for batch operations. More efficient than multiple single calls.

### 4. Dimension Information (embedding_service.rs:46-48)

Document model dimensions:

```rust
pub fn dimension(&self) -> usize {
    768  // MultilingualE5Base produces 768-dimensional embeddings
}
```

**Pattern:** Expose dimension for vector database schema creation.

## Multi-Vector Retrieval Strategy

**The problem with single embeddings:** One query embedding might miss relevant results because it only captures one perspective.

**The solution:** Generate multiple embeddings from different angles and combine results.

**Why it works better:**

**Single embedding approach:**
```
User: "How do I say 'What's up?' casually in Spanish?"
  ↓
[Generate 1 embedding]
  ↓
[Search for similar examples]
  ↓
Results: Might miss examples that match style but not exact words
```

**Multi-vector approach:**
```
User: "How do I say 'What's up?' casually in Spanish?"
  ↓
Embedding 1: "How do I say 'What's up?'" (content/meaning)
Embedding 2: "casual Spanish greeting" (style/formality)
Embedding 3: [conversation history] (context/topic)
  ↓
[Search with each embedding]
  ↓
Combine Results (diversity + relevance)
```

**Benefits:**
1. **Better coverage** - Capture different aspects (meaning, style, context)
2. **More diverse results** - Not all similar in the same way
3. **Handles ambiguity** - Multiple interpretations considered
4. **Context-aware** - Previous conversation influences retrieval

### 5. Multi-Vector RAG (agent_service.rs:68-98)

Generate multiple embeddings for better retrieval:

```rust
// Embedding 1: Semantic (content-based)
let content_embedding = self
    .embeddings
    .embed_text(&query_text)
    .context("Failed to generate content embedding")?;

// Embedding 2: Stylistic cue
let style_query = format!("{} response in {}", formality_str, dialect.name());
let style_embedding = self
    .embeddings
    .embed_text(&style_query)
    .context("Failed to generate style embedding")?;

// Embedding 3: Topic summary (if history exists)
let topic_embedding = if !conversation_history.is_empty() {
    let topic_summary = conversation_history.join(" ");
    Some(
        self.embeddings
            .embed_text(&topic_summary)
            .context("Failed to generate topic embedding")?,
    )
} else {
    None
};
```

**Pattern:** Generate multiple embeddings from different perspectives:
- **Content embedding** - What the user is asking about
- **Style embedding** - How they want the answer (formal/casual)
- **Topic embedding** - What they've been talking about

**How to combine:**
1. Search with each embedding separately
2. Merge results, removing duplicates
3. Re-rank by combined relevance scores
4. Return top K diverse results

## Best Practices from dialect-coach

1. **Show download progress** - First-time setup downloads model, inform users
2. **Wrap in service** - Encapsulate model for clean API
3. **Error context** - Add context to errors for debugging
4. **Document dimensions** - Make embedding size explicit
5. **Batch when possible** - More efficient than individual embeds
6. **Multilingual model** - Use MultilingualE5 for non-English text
7. **Reuse model** - Initialize once, embed many times

## Troubleshooting

### Issue: Model download takes long time

**Cause:** First-time download of ~220MB model

**Solution:** Enable progress indicator:

```rust
InitOptions::new(EmbeddingModel::MultilingualE5Base)
    .with_show_download_progress(true)
```

### Issue: Out of memory

**Cause:** Processing too many texts at once

**Solution:** Batch in chunks:

```rust
fn embed_large_batch(model: &TextEmbedding, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
    let chunk_size = 100;
    let mut all_embeddings = Vec::new();

    for chunk in texts.chunks(chunk_size) {
        let embeddings = model.embed(chunk.to_vec(), None)?;
        all_embeddings.extend(embeddings);
    }

    Ok(all_embeddings)
}
```

### Issue: Embeddings not similar for related texts

**Cause:** Using wrong model for language

**Solution:** Use MultilingualE5 for non-English:

```rust
EmbeddingModel::MultilingualE5Base  // 100+ languages
```

## Further Resources

- **GitHub:** https://github.com/Anush008/fastembed-rs
- **Crate:** https://crates.io/crates/fastembed
- **Docs:** https://docs.rs/fastembed/latest/fastembed/
- **ONNX Runtime:** https://onnxruntime.ai/
- **E5 Paper:** https://arxiv.org/abs/2212.03533

## Summary

fastembed provides local embedding generation:

- **Fast inference** with ONNX Runtime
- **Multilingual support** (100+ languages)
- **No API calls** - runs locally
- **Multiple models** - different sizes and quality
- **Batch processing** - efficient for multiple texts
- **768 dimensions** (MultilingualE5Base)

The dialect-coach project demonstrates production use for RAG: multi-vector retrieval with content, style, and topic embeddings, all generated locally for fast, cost-effective semantic search.
