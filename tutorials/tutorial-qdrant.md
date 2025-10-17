# Qdrant Tutorial: Vector Database for Semantic Search

## Overview

**qdrant-client** is the Rust client for Qdrant, a vector similarity search engine. It enables storing and searching high-dimensional vectors with metadata filtering, making it ideal for RAG (Retrieval-Augmented Generation) applications.

**Version used in dialect-coach:** `1.13`

**Why we use it:** Fast similarity search for semantic retrieval, metadata filtering for precise results, cloud hosting available, open source.

## Dependencies

```toml
[dependencies]
qdrant-client = "1.13"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

## Qdrant Data Model

Understanding the hierarchy is essential:

```
Qdrant Server
  ├── Collection 1 (e.g., "dialect_examples")
  │     ├── Vector Configuration
  │     │     ├── Dimensions: 768
  │     │     └── Distance: Cosine
  │     ├── Points (documents with vectors)
  │     │     ├── Point ID: 1
  │     │     │     ├── Vector: [0.1, 0.2, ..., 0.9]
  │     │     │     └── Payload: {dialect: "mexican", content: "¿Qué onda?"}
  │     │     ├── Point ID: 2
  │     │     │     ├── Vector: [0.3, 0.1, ..., 0.7]
  │     │     │     └── Payload: {dialect: "spanish", content: "¿Qué tal?"}
  │     │     └── ...
  │     └── Indexes (for filtering)
  │           ├── Field: "dialect" (Keyword index)
  │           └── Field: "formality" (Keyword index)
  ├── Collection 2
  └── ...
```

**Key concepts:**
- **Collection** - Container for vectors with same dimensions/distance metric
- **Point** - A vector + metadata (like a database row)
- **Payload** - Metadata stored with each point (JSON-like structure)
- **Index** - Speed up filtering on payload fields

**Think of it as:**
- Collection = SQL table
- Point = Row
- Vector = Special column for similarity search
- Payload = Other columns
- Index = Database index

## Core Concepts

### 1. Client Connection

Connect to Qdrant server (local or cloud):

```rust
use qdrant_client::Qdrant;

// Local
let client = Qdrant::from_url("http://localhost:6334").build()?;

// Cloud with API key
let client = Qdrant::from_url("https://xyz.cloud.qdrant.io")
    .api_key("your-api-key")
    .build()?;
```

### 2. Collections

Collections are like tables, storing vectors with metadata:

```rust
use qdrant_client::qdrant::{CreateCollectionBuilder, Distance, VectorParamsBuilder};

client.create_collection(
    CreateCollectionBuilder::new("my_collection")
        .vectors_config(
            VectorParamsBuilder::new(768, Distance::Cosine)
        )
).await?;
```

### 3. Points

Points are documents with vectors and metadata:

```rust
use qdrant_client::qdrant::{PointStruct, Payload};
use serde_json::json;

let point = PointStruct::new(
    1,  // ID
    vec![0.1, 0.2, 0.3, /* ... 768 dimensions */],
    json!({
        "text": "Hello, world!",
        "category": "greeting"
    }).try_into()?
);
```

## Filter Query Language

**Filters let you combine similarity search with metadata conditions:**

**Basic syntax:**
```rust
use qdrant_client::qdrant::{Filter, Condition};

// Single condition
let filter = Filter::must([
    Condition::matches("dialect", "mexican")
]);

// Multiple conditions (AND)
let filter = Filter::must([
    Condition::matches("dialect", "mexican"),
    Condition::matches("formality", "casual")
]);

// OR conditions
let filter = Filter::should([
    Condition::matches("dialect", "mexican"),
    Condition::matches("dialect", "argentinian")
]);

// NOT conditions
let filter = Filter::must_not([
    Condition::matches("formality", "slang")
]);
```

**Common filter patterns:**
- **`must`** - All conditions must be true (AND)
- **`should`** - At least one condition must be true (OR)
- **`must_not`** - No conditions can be true (NOT)

**Why filters matter:** You want similar vectors BUT only from specific categories. Example: "Find similar Spanish phrases, but only Mexican dialect, casual formality."

## Distance Metrics Explained

**Distance metrics determine how similarity is calculated:**

**Cosine Distance** (dialect-coach uses this):
```rust
Distance::Cosine
```
- **Measures:** Angle between vectors (direction, not magnitude)
- **Range:** 0 (identical) to 2 (opposite)
- **Best for:** Text embeddings, semantic similarity
- **Why:** Captures meaning regardless of document length

**Euclidean Distance:**
```rust
Distance::Euclidean
```
- **Measures:** Straight-line distance in vector space
- **Range:** 0 (identical) to ∞
- **Best for:** When magnitude matters (e.g., physical measurements)
- **Less common** for text embeddings

**Dot Product:**
```rust
Distance::Dot
```
- **Measures:** Raw similarity (no normalization)
- **Range:** -∞ to +∞
- **Best for:** Pre-normalized vectors
- **Fast** but less intuitive

**Which to choose?**
- **Text/semantic search** → Cosine (most common)
- **Image embeddings** → Cosine or Euclidean
- **Pre-normalized vectors** → Dot (fastest)

**dialect-coach uses Cosine** because text embeddings from MultilingualE5 work best with directional similarity.

## Search vs Scroll: Decision Guide

**Two ways to retrieve points:**

### **Search** (Similarity-based)
```rust
client.search_points(
    SearchPointsBuilder::new(collection, query_vector, limit)
).await?;
```

**When to use:**
- Finding similar items to a query
- Semantic search
- RAG retrieval
- Recommendation systems

**How it works:** Ranks all points by vector similarity, returns top K.

### **Scroll** (Iteration-based)
```rust
client.scroll(
    ScrollPointsBuilder::new(collection)
        .limit(100)
).await?;
```

**When to use:**
- Random sampling
- Exporting all data
- Getting diverse examples
- No specific query, just browsing

**How it works:** Iterates through points in arbitrary order (not similarity-based).

**Comparison:**

| Feature | Search | Scroll |
|---------|--------|--------|
| **Requires query vector** | Yes | No |
| **Returns** | Most similar | Random/sequential |
| **Use case** | Find relevant items | Get diverse samples |
| **Speed** | Fast (indexed) | Very fast (no scoring) |
| **Example** | "Find Spanish greetings like 'hola'" | "Give me 100 random examples" |

**dialect-coach uses both:**
- **Search** - Find examples similar to user query (RAG)
- **Scroll** - Get random examples for variety/exploration

### 4. Search

Find similar vectors:

```rust
use qdrant_client::qdrant::SearchPointsBuilder;

let results = client.search_points(
    SearchPointsBuilder::new("my_collection", query_vector, 10)
        .with_payload(true)
).await?;
```

## Step-by-Step: Building a Q&A System

### Step 1: Connect and Create Collection

```rust
use qdrant_client::Qdrant;
use qdrant_client::qdrant::{
    CreateCollectionBuilder, Distance, VectorParamsBuilder,
};
use anyhow::Result;

async fn setup_qdrant() -> Result<Qdrant> {
    let client = Qdrant::from_url("http://localhost:6334").build()?;

    // Create collection if it doesn't exist
    let collection_name = "knowledge_base";

    client.create_collection(
        CreateCollectionBuilder::new(collection_name)
            .vectors_config(VectorParamsBuilder::new(768, Distance::Cosine))
    ).await?;

    println!("Collection created successfully!");

    Ok(client)
}
```

### Step 2: Insert Documents

```rust
use qdrant_client::qdrant::{PointStruct, UpsertPointsBuilder};
use serde_json::json;

async fn insert_documents(
    client: &Qdrant,
    documents: Vec<(String, Vec<f32>)>  // (text, embedding)
) -> Result<()> {
    let points: Vec<PointStruct> = documents
        .into_iter()
        .enumerate()
        .map(|(idx, (text, vector))| {
            PointStruct::new(
                idx as u64,
                vector,
                json!({
                    "text": text,
                    "indexed_at": chrono::Utc::now().to_rfc3339()
                }).try_into().unwrap()
            )
        })
        .collect();

    client.upsert_points(
        UpsertPointsBuilder::new("knowledge_base", points)
    ).await?;

    println!("Inserted {} documents", points.len());

    Ok(())
}
```

### Step 3: Search with Filters

```rust
use qdrant_client::qdrant::{
    SearchPointsBuilder, Filter, Condition, FieldCondition, Match,
};

async fn search_filtered(
    client: &Qdrant,
    query_vector: Vec<f32>,
    category: &str,
    limit: usize,
) -> Result<Vec<String>> {
    // Build filter
    let filter = Filter::must([
        Condition::field(
            FieldCondition::new_match(
                "category",
                Match::from(category.to_string())
            )
        )
    ]);

    // Search with filter
    let results = client.search_points(
        SearchPointsBuilder::new("knowledge_base", query_vector, limit as u64)
            .filter(filter)
            .with_payload(true)
    ).await?;

    // Extract texts
    let texts: Vec<String> = results
        .result
        .into_iter()
        .filter_map(|point| {
            point.payload.get("text")?.as_str().map(|s| s.to_string())
        })
        .collect();

    Ok(texts)
}
```

### Step 4: Complete Q&A System

```rust
struct QASystem {
    client: Qdrant,
    embedder: fastembed::TextEmbedding,
    collection_name: String,
}

impl QASystem {
    async fn new() -> Result<Self> {
        let client = Qdrant::from_url("http://localhost:6334").build()?;

        // Initialize embedder
        let embedder = fastembed::TextEmbedding::try_new(
            fastembed::InitOptions::new(fastembed::EmbeddingModel::MultilingualE5Base)
        )?;

        Ok(Self {
            client,
            embedder,
            collection_name: "qa_system".to_string(),
        })
    }

    async fn index_documents(&self, documents: Vec<String>) -> Result<()> {
        // Generate embeddings
        let embeddings = self.embedder.embed(documents.clone(), None)?;

        // Create points
        let points: Vec<PointStruct> = documents
            .into_iter()
            .zip(embeddings.into_iter())
            .enumerate()
            .map(|(idx, (text, vector))| {
                PointStruct::new(
                    idx as u64,
                    vector,
                    json!({"text": text}).try_into().unwrap()
                )
            })
            .collect();

        // Upsert to Qdrant
        self.client.upsert_points(
            UpsertPointsBuilder::new(&self.collection_name, points)
        ).await?;

        Ok(())
    }

    async fn ask(&self, question: &str, top_k: usize) -> Result<Vec<String>> {
        // Embed question
        let query_embedding = self.embedder
            .embed(vec![question.to_string()], None)?
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("Failed to embed question"))?;

        // Search
        let results = self.client.search_points(
            SearchPointsBuilder::new(&self.collection_name, query_embedding, top_k as u64)
                .with_payload(true)
        ).await?;

        // Extract answers
        let answers: Vec<String> = results
            .result
            .into_iter()
            .filter_map(|point| {
                point.payload.get("text")?.as_str().map(|s| s.to_string())
            })
            .collect();

        Ok(answers)
    }
}
```

## How dialect-coach Uses Qdrant

### 1. Client Creation (qdrant_service.rs:14-24, 27-34)

Connection with API key:

```rust
pub async fn new(url: &str, api_key: &str) -> Result<Self> {
    let client = Qdrant::from_url(url)
        .api_key(api_key)
        .build()
        .context("Failed to connect to Qdrant")?;

    tracing::info!("Connected to Qdrant at {}", url);

    Ok(Self { client })
}

pub async fn from_env() -> Result<Self> {
    let url = std::env::var("QDRANT_URL")
        .context("QDRANT_URL environment variable not set")?;
    let api_key = std::env::var("QDRANT_API_KEY")
        .context("QDRANT_API_KEY environment variable not set")?;

    Self::new(&url, &api_key).await
}
```

**Pattern:** Environment-based configuration for flexibility across environments.

### 2. Vector Search with Filters (qdrant_service.rs:37-64)

Finding dialect examples:

```rust
pub async fn search_dialect_examples(
    &self,
    query_embedding: Vec<f32>,
    dialect: Dialect,
    limit: usize,
) -> Result<Vec<DialectDocument>> {
    // Build filter for dialect
    let filter = Filter::must([
        Condition::matches("dialect", dialect.id().to_string())
    ]);

    let search_result = self
        .client
        .search_points(
            SearchPointsBuilder::new(COLLECTION_NAME, query_embedding, limit as u64)
                .filter(filter)
                .with_payload(true),
        )
        .await
        .context("Failed to search Qdrant")?;

    let documents = self.parse_search_results(search_result.result, dialect)?;

    tracing::info!(
        "Found {} dialect examples for {}",
        documents.len(),
        dialect.name()
    );

    Ok(documents)
}
```

**Pattern:** Combine similarity search with metadata filtering for precise results.

### 3. Random Sampling (qdrant_service.rs:68-139)

Get diverse examples:

```rust
pub async fn random_dialect_samples(
    &self,
    dialect: Dialect,
    formality_levels: Vec<Formality>,
    limit: usize,
) -> Result<Vec<DialectDocument>> {
    use qdrant_client::qdrant::ScrollPointsBuilder;

    let filter = Filter::must([
        Condition::matches("dialect", dialect.id().to_string())
    ]);

    // Use scroll to get random samples
    let scroll_result = self
        .client
        .scroll(
            ScrollPointsBuilder::new(COLLECTION_NAME)
                .filter(filter)
                .limit(limit as u32)
                .with_payload(true),
        )
        .await
        .context("Failed to scroll Qdrant for random samples")?;

    // Parse and filter by formality
    let mut documents = Vec::new();
    for point in scroll_result.result {
        let payload = point.payload;

        let content = payload
            .get("content")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_default();

        let formality = parse_formality(&payload);

        // Filter by formality if specified
        if !formality_levels.is_empty() {
            if let Some(f) = formality {
                if !formality_levels.contains(&f) {
                    continue;
                }
            }
        }

        documents.push(DialectDocument {
            content,
            dialect,
            formality,
            embedding: Vec::new(),
        });
    }

    Ok(documents)
}
```

**Pattern:** Use scroll API for random sampling instead of similarity search. Good for diverse retrieval.

### 4. Payload Parsing (qdrant_service.rs:142-182)

Extract structured data from results:

```rust
fn parse_search_results(
    &self,
    results: Vec<qdrant_client::qdrant::ScoredPoint>,
    dialect: Dialect,
) -> Result<Vec<DialectDocument>> {
    let mut documents = Vec::new();

    for point in results {
        let payload = point.payload;

        // Extract content
        let content = payload
            .get("content")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_default();

        // Parse formality enum from string
        let formality = payload
            .get("formality")
            .and_then(|v| v.as_str())
            .and_then(|s| match s.as_ref() {
                "Formal" => Some(Formality::Formal),
                "Casual" => Some(Formality::Casual),
                "Slang" => Some(Formality::Slang),
                _ => None,
            });

        documents.push(DialectDocument {
            content,
            dialect,
            formality,
            embedding: Vec::new(),
        });
    }

    Ok(documents)
}
```

**Pattern:** Safe payload extraction with defaults for missing fields.

### 5. Collection Info (qdrant_service.rs:185-201)

Debugging helper:

```rust
pub async fn get_collection_info(&self) -> Result<()> {
    let collection_info = self
        .client
        .collection_info(COLLECTION_NAME)
        .await
        .context("Failed to get collection info")?;

    if let Some(result) = collection_info.result {
        tracing::info!(
            "Collection '{}' has {:?} points",
            COLLECTION_NAME,
            result.points_count
        );
    }

    Ok(())
}
```

**Pattern:** Verify collection state during initialization.

## Best Practices from dialect-coach

1. **Filter + similarity** - Combine both for precise retrieval
2. **Parse payloads safely** - Use `Option` chain for missing fields
3. **Use scroll for sampling** - Not just similarity search
4. **Create field indexes** - Enable efficient filtering
5. **Environment config** - URL and API key from env vars
6. **Log operations** - Track search results and counts
7. **Error context** - Add context to all Qdrant operations

## Troubleshooting

### Issue: Connection refused

**Cause:** Qdrant server not running

**Solution:** Start Qdrant:

```bash
docker run -p 6333:6333 -p 6334:6334 qdrant/qdrant
```

### Issue: "Collection not found"

**Cause:** Collection doesn't exist

**Solution:** Create collection first:

```rust
client.create_collection(/* ... */).await?;
```

### Issue: Filter not working

**Cause:** Field index not created

**Solution:** Create index:

```rust
use qdrant_client::qdrant::{CreateFieldIndexCollectionBuilder, FieldType};

client.create_field_index(
    CreateFieldIndexCollectionBuilder::new(
        collection_name,
        "dialect",
        FieldType::Keyword,
    )
).await?;
```

## Further Resources

- **Qdrant Docs:** https://qdrant.tech/documentation/
- **Rust Client:** https://github.com/qdrant/rust-client
- **Crate:** https://crates.io/crates/qdrant-client
- **API Docs:** https://docs.rs/qdrant-client/latest/qdrant_client/
- **Cloud:** https://cloud.qdrant.io/

## Summary

qdrant-client provides vector database capabilities:

- **Similarity search** with cosine/euclidean distance
- **Metadata filtering** for precise results
- **Scroll API** for random sampling
- **Cloud or self-hosted** deployment
- **Payload storage** with structured data
- **Field indexes** for efficient filtering

The dialect-coach project demonstrates production RAG patterns: combined similarity + metadata filtering, safe payload extraction, diverse retrieval through scroll, and proper error handling throughout.
