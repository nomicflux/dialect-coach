# Serde Tutorial: Serialization Framework for Rust

## Overview

**Serde** is a framework for serializing and deserializing Rust data structures efficiently and generically. It supports JSON, YAML, TOML, MessagePack, and many other formats.

**Version used in dialect-coach:** `1.0` with derive feature, `serde_json` for JSON

**Why we use it:** Type-safe serialization across network boundaries (frontend ↔ backend), persistent storage, configuration files.

## Dependencies

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

## Core Concepts

### 1. Derive Macros

Automatically implement serialization:

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
struct Person {
    name: String,
    age: u32,
    email: Option<String>,
}
```

### 2. Serialization

Convert Rust data to format:

```rust
let person = Person {
    name: "Alice".to_string(),
    age: 30,
    email: Some("alice@example.com".to_string()),
};

// To JSON string
let json = serde_json::to_string(&person)?;
// {"name":"Alice","age":30,"email":"alice@example.com"}

// To JSON with pretty printing
let json_pretty = serde_json::to_string_pretty(&person)?;
```

### 3. Deserialization

Parse format into Rust data:

```rust
let json = r#"{"name":"Bob","age":25,"email":null}"#;
let person: Person = serde_json::from_str(json)?;

println!("Name: {}", person.name);
println!("Age: {}", person.age);
println!("Email: {:?}", person.email);
```

### 4. Field Attributes

Customize serialization:

```rust
#[derive(Serialize, Deserialize)]
struct Config {
    #[serde(rename = "userName")]
    user_name: String,

    #[serde(default)]
    timeout: u32,

    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}
```

**Why `skip_serializing_if`?**
- **Smaller payloads** - Omit fields with default/empty values, reducing JSON size
- **Cleaner APIs** - External APIs don't see fields you haven't set
- **Compatibility** - Works with APIs that don't expect certain fields
- **Example:** `{"userName":"alice","timeout":30}` instead of `{"userName":"alice","timeout":30,"description":null}`

## Step-by-Step: Building a Config System

### Step 1: Basic Struct

```rust
use serde::{Serialize, Deserialize};
use std::fs;

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    host: String,
    port: u16,
    debug: bool,
}

fn load_config(path: &str) -> Result<Config, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let config: Config = serde_json::from_str(&content)?;
    Ok(config)
}

fn save_config(path: &str, config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string_pretty(config)?;
    fs::write(path, json)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config {
        host: "localhost".to_string(),
        port: 8080,
        debug: true,
    };

    save_config("config.json", &config)?;

    let loaded = load_config("config.json")?;
    println!("{:?}", loaded);

    Ok(())
}
```

### Step 2: Nested Structures

```rust
#[derive(Serialize, Deserialize, Debug)]
struct DatabaseConfig {
    url: String,
    max_connections: u32,
}

#[derive(Serialize, Deserialize, Debug)]
struct ServerConfig {
    host: String,
    port: u16,
}

#[derive(Serialize, Deserialize, Debug)]
struct AppConfig {
    server: ServerConfig,
    database: DatabaseConfig,
    log_level: String,
}

// JSON structure:
// {
//   "server": {
//     "host": "localhost",
//     "port": 8080
//   },
//   "database": {
//     "url": "postgres://localhost/db",
//     "max_connections": 10
//   },
//   "log_level": "info"
// }
```

### Step 3: Enums

```rust
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
enum Message {
    Text { content: String },
    Image { url: String, width: u32, height: u32 },
    Video { url: String, duration: u32 },
}

// Tagged enum JSON:
// {"type":"Text","content":"Hello"}
// {"type":"Image","url":"img.jpg","width":800,"height":600}
```

### Step 4: Optional Fields and Defaults

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    /// Required field
    name: String,

    /// Optional field (can be missing)
    #[serde(default)]
    timeout: u32,

    /// Optional with custom default
    #[serde(default = "default_port")]
    port: u16,

    /// Truly optional
    description: Option<String>,

    /// Skip if None when serializing
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<String>,
}

fn default_port() -> u16 {
    8080
}

impl Default for Config {
    fn default() -> Self {
        Self {
            name: String::new(),
            timeout: 30,
            port: 8080,
            description: None,
            metadata: None,
        }
    }
}
```

### Step 5: Working with JSONL (Newline-Delimited JSON)

```rust
use std::fs::File;
use std::io::{BufReader, BufRead, Write};

#[derive(Serialize, Deserialize, Debug)]
struct Record {
    id: u32,
    data: String,
}

// Write JSONL
fn write_jsonl(path: &str, records: Vec<Record>) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = File::create(path)?;

    for record in records {
        let json = serde_json::to_string(&record)?;
        writeln!(file, "{}", json)?;
    }

    Ok(())
}

// Read JSONL
fn read_jsonl(path: &str) -> Result<Vec<Record>, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut records = Vec::new();

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let record: Record = serde_json::from_str(&line)?;
        records.push(record);
    }

    Ok(records)
}
```

**When to Use JSONL (Newline-Delimited JSON):**

**Use JSONL when:**
- **Processing large datasets** - Stream line-by-line without loading entire file into memory
- **Append-only logs** - Add new records without rewriting entire file
- **Parallel processing** - Different workers can process different lines independently
- **Error resilience** - One corrupt line doesn't break the entire file
- **Progressive loading** - Start processing before entire file is downloaded

**Use regular JSON when:**
- **Small datasets** - Entire structure fits comfortably in memory
- **Complex nesting** - Data is deeply nested or has complex relationships
- **Human editing** - File will be manually edited (JSONL harder to read)
- **API responses** - Single request/response model

**Example use case:** Corpus processing in dialect-coach uses JSONL because:
1. Thousands of dialect examples would overflow memory as single JSON
2. Can process examples in batches
3. Can resume processing if interrupted
4. Easy to append new examples without rewriting

## How dialect-coach Uses Serde

### 1. Shared Data Models (shared/src/models/)

Type-safe communication between frontend and backend:

```rust
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Message {
    pub id: Uuid,
    pub session_id: Uuid,
    pub participant_id: String,
    pub content: String,
    pub language: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub metadata: MessageMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct MessageMetadata {
    pub formality: Option<Formality>,
    pub teaching_mode: Option<TeachingMode>,
}
```

**Pattern:** Shared structs ensure frontend and backend agree on format. `PartialEq` for testing.

### 2. Enum Serialization (shared/src/models/language.rs)

Enums with specific serialization:

```rust
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    Spanish,
    Arabic,
    French,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Dialect {
    SpanishMexican,
    SpanishArgentinian,
    SpanishSpanish,
    ArabicEgyptian,
    ArabicLevantine,
    ArabicGulf,
    FrenchParisian,
    FrenchQuebecois,
    FrenchAfrican,
}
```

**Pattern:** Simple enum serialization. Serde defaults to variant names as strings.

### 3. WebSocket Message Parsing (websocket.rs:113, 200, 290)

Parsing JSON over WebSocket:

```rust
// Deserialize incoming message
match serde_json::from_str::<Message>(&text) {
    Ok(parsed_msg) => {
        tracing::info!(
            "Valid message from {} in session {}",
            parsed_msg.participant_id,
            parsed_msg.session_id
        );
        // Process message...
    }
    Err(e) => {
        tracing::error!("Failed to parse message JSON: {}", e);
    }
}

// Serialize outgoing message
let response_msg = Message::new(/* ... */);
match serde_json::to_string(&response_msg) {
    Ok(response_json) => {
        tx.send(response_json)?;
    }
    Err(e) => {
        tracing::error!("Failed to serialize: {}", e);
    }
}
```

**Pattern:** Deserialize incoming, validate, serialize outgoing. Always handle errors.

### 4. JSONL File Processing (corpus-processor/src/main.rs:185-203)

Reading newline-delimited JSON:

```rust
fn load_documents_from_jsonl(path: &str) -> Result<Vec<DialectDocument>> {
    let content = fs::read_to_string(path)
        .context(format!("Failed to read JSONL file: {}", path))?;

    let mut documents = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }

        let doc: DialectDocument = serde_json::from_str(line)
            .context(format!("Failed to parse JSON at line {}", line_num + 1))?;

        documents.push(doc);
    }

    Ok(documents)
}
```

**Pattern:** Process line-by-line, skip empty, provide line numbers in errors.

### 5. Corpus Data Structure (shared/src/models/corpus.rs)

Data for RAG:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialectDocument {
    pub content: String,
    pub dialect: Dialect,
    pub formality: Option<Formality>,
    pub embedding: Vec<f32>,
}
```

**Pattern:** Optional fields for metadata, vector for embeddings.

## Common Patterns

### Pattern 1: Rename Fields

```rust
#[derive(Serialize, Deserialize)]
struct ApiResponse {
    #[serde(rename = "userId")]
    user_id: u32,

    #[serde(rename = "createdAt")]
    created_at: String,
}
```

### Pattern 2: Skip Fields

```rust
#[derive(Serialize, Deserialize)]
struct User {
    name: String,

    #[serde(skip_serializing)]
    password_hash: String,  // Never sent to client

    #[serde(skip)]
    cached_data: Option<Vec<u8>>,  // Never serialized/deserialized
}
```

### Pattern 3: Custom Serialization

```rust
use serde::{Serializer, Deserializer};

#[derive(Serialize, Deserialize)]
struct Duration {
    #[serde(serialize_with = "as_seconds", deserialize_with = "from_seconds")]
    value: std::time::Duration,
}

fn as_seconds<S>(duration: &std::time::Duration, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_u64(duration.as_secs())
}

fn from_seconds<'de, D>(deserializer: D) -> Result<std::time::Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let secs = u64::deserialize(deserializer)?;
    Ok(std::time::Duration::from_secs(secs))
}
```

### Pattern 4: Flatten Nested Structs

```rust
#[derive(Serialize, Deserialize)]
struct Metadata {
    created: String,
    updated: String,
}

#[derive(Serialize, Deserialize)]
struct Document {
    id: u32,
    content: String,

    #[serde(flatten)]
    metadata: Metadata,
}

// JSON: {"id":1,"content":"...","created":"...","updated":"..."}
```

## Best Practices from dialect-coach

1. **Shared models** - Define once, use in frontend and backend
2. **Always handle errors** - Parsing can fail, especially user input
3. **Use Option for optional** - Clear intent, proper serialization
4. **JSONL for large datasets** - Process line-by-line, memory efficient
5. **PartialEq for testing** - Easy to verify deserialization
6. **Context in errors** - Include line numbers, field names
7. **Skip sensitive data** - Use `#[serde(skip_serializing)]`

## Troubleshooting

### Issue: "Missing field" error

**Cause:** Required field not in JSON

**Solution:** Make optional or provide default:

```rust
#[serde(default)]
field: Type,

// Or
field: Option<Type>,
```

### Issue: Enum doesn't deserialize

**Cause:** JSON doesn't match variant name

**Solution:** Use rename or tag:

```rust
#[serde(rename_all = "lowercase")]
enum MyEnum {
    VariantOne,  // Matches "variantone"
}
```

### Issue: "Data did not match any variant"

**Cause:** Enum structure mismatch

**Solution:** Use tagged representation:

```rust
#[serde(tag = "type")]
enum Message {
    Text { content: String },
    Image { url: String },
}
```

## Further Resources

- **Serde Website:** https://serde.rs/
- **Data Formats:** https://serde.rs/#data-formats
- **Attributes:** https://serde.rs/attributes.html
- **Examples:** https://serde.rs/examples.html
- **JSON Docs:** https://docs.rs/serde_json/latest/serde_json/

## Summary

Serde provides serialization framework:

- **Derive macros** for automatic implementation
- **Format-agnostic** - JSON, YAML, TOML, etc.
- **Type-safe** - Compile-time guarantees
- **Flexible** - Attributes for customization
- **Cross-boundary** - Share types between frontend/backend
- **JSONL support** - Efficient for large datasets

The dialect-coach project demonstrates production patterns: shared data models across WASM/server boundary, WebSocket message serialization, JSONL processing for corpus data, and proper error handling with context.
