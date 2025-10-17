# IndexedDB Tutorial: Browser Storage in Rust/WASM

## Overview

**indexed_db_futures** provides async/await Rust bindings for the browser's IndexedDB API. IndexedDB is a low-level API for client-side storage of structured data, including files and blobs.

**Version used in dialect-coach:** `0.5`

**Why we use it:** Enables persistent, offline-capable storage in web applications with a Rust-friendly async API.

## Dependencies

```toml
[dependencies]
indexed_db_futures = "0.5"
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

## IndexedDB Data Model

Understanding the hierarchy is essential for working with IndexedDB:

```
Browser Storage (per origin: https://example.com)
  ├── Database 1 (e.g., "my_app_db")
  │     ├── Version: 3
  │     ├── Object Store 1 (e.g., "users")
  │     │     ├── Configuration
  │     │     │     ├── Key Path: "id"
  │     │     │     ├── Auto-increment: true
  │     │     │     └── Key Generator: starts at 1
  │     │     ├── Records (key-value pairs)
  │     │     │     ├── Key: 1
  │     │     │     │     └── Value: {id: 1, name: "Alice", email: "alice@example.com"}
  │     │     │     ├── Key: 2
  │     │     │     │     └── Value: {id: 2, name: "Bob", email: "bob@example.com"}
  │     │     │     └── Key: 3
  │     │     │           └── Value: {id: 3, name: "Charlie", email: "charlie@example.com"}
  │     │     └── Indexes (for querying)
  │     │           ├── "by_name" (field: "name", unique: false)
  │     │           └── "by_email" (field: "email", unique: true)
  │     ├── Object Store 2 (e.g., "messages")
  │     │     ├── Configuration
  │     │     │     ├── Key Path: "timestamp"
  │     │     │     └── Auto-increment: false
  │     │     ├── Records
  │     │     │     ├── Key: 1634567890123
  │     │     │     │     └── Value: {timestamp: 1634567890123, text: "Hello", user_id: 1}
  │     │     └── Indexes
  │     │           └── "by_user" (field: "user_id")
  │     └── Object Store 3 (e.g., "settings")
  ├── Database 2 (e.g., "cache_db")
  └── ...
```

**Key concepts:**

- **Database** - Top-level container, versioned (like a schema)
  - Each origin (domain) can have multiple databases
  - Version changes trigger upgrade handlers

- **Object Store** - Collection of records (like a SQL table)
  - Has a key path (field to use as primary key)
  - Can auto-generate keys (auto-increment)
  - Contains key-value pairs

- **Record** - A single data entry (like a SQL row)
  - Must have a unique key
  - Value is typically a JavaScript object/struct
  - Stored as key → value mapping

- **Index** - Secondary index for queries (like SQL index)
  - Allows querying by non-key fields
  - Can be unique or non-unique
  - Example: Search users by email without knowing ID

- **Version** - Schema version number
  - Starts at 1, increments when schema changes
  - Changing version triggers `onupgradeneeded` event
  - Used to create/modify/delete object stores

**Think of it as:**
- Database = SQL database
- Object Store = SQL table
- Record = SQL row
- Key = Primary key
- Index = SQL index
- Version = Database migration version

**Example in dialect-coach:**
```
Database: "dialect_coach"
  ├── Object Store: "sessions"
  │     ├── Key Path: "id" (auto-increment)
  │     └── Records: Session data for each user session
  ├── Object Store: "chat_history"
  │     ├── Key Path: "timestamp"
  │     └── Records: Chat messages with timestamps
  └── Object Store: "user_preferences"
        ├── Key Path: "key" (e.g., "theme", "language")
        └── Records: User settings
```

## Understanding Transactions

**What is a transaction?** An atomic unit of work where all operations succeed together or fail together.

**Why transactions matter:**

```
Without transactions (problem):
1. Write user data → SUCCESS
2. Write user settings → FAILS
3. Write user history → SUCCESS

Result: Partial data (inconsistent state)
```

```
With transactions (solution):
BEGIN TRANSACTION
1. Write user data
2. Write user settings ← FAILS
3. Rollback automatically

Result: No data written (consistent state)
```

**Transaction modes:**

1. **Readonly** - Can only read data
   ```rust
   let tx = db.transaction_on_one("users")?;
   // Default mode, safe for concurrent access
   ```
   - **Use when:** Only querying data
   - **Benefits:** Multiple readonly transactions can run concurrently
   - **Limitations:** Cannot write

2. **Readwrite** - Can read and write data
   ```rust
   let tx = db.transaction_on_one_with_mode("users", IdbTransactionMode::Readwrite)?;
   ```
   - **Use when:** Creating, updating, or deleting data
   - **Exclusive:** Only one readwrite transaction per object store at a time
   - **Must commit:** Await the transaction to commit changes

3. **Versionchange** - Can modify schema (create/delete object stores)
   ```rust
   // Only available in onupgradeneeded handler
   db.create_object_store("new_store", params)?;
   ```
   - **Use when:** Changing database schema
   - **Only in:** Upgrade event handler
   - **Blocks:** All other transactions

**Transaction lifecycle:**

```
1. Create transaction
   let tx = db.transaction_on_one_with_mode("users", IdbTransactionMode::Readwrite)?;
         ↓
2. Get object store
   let store = tx.object_store("users")?;
         ↓
3. Perform operations
   store.add(...)?.await?;
   store.put(...)?.await?;
   store.delete(...)?.await?;
         ↓
4. Commit (by awaiting transaction)
   tx.await.into_result()?;  // ← CRITICAL: Don't forget this!
         ↓
5. Changes persisted ✓
```

**Common mistakes:**

❌ **Forgetting to commit:**
```rust
let tx = db.transaction_on_one_with_mode("users", Readwrite)?;
let store = tx.object_store("users")?;
store.add(value)?.await?;
// Missing: tx.await.into_result()?;
// Result: Changes NOT saved!
```

✓ **Correct pattern:**
```rust
let tx = db.transaction_on_one_with_mode("users", Readwrite)?;
let store = tx.object_store("users")?;
store.add(value)?.await?;
tx.await.into_result()?;  // ✓ Committed
```

**Best practices:**

1. **Keep transactions short** - Hold locks for minimal time
   ```rust
   // BAD: Long-running operation inside transaction
   let tx = db.transaction_on_one_with_mode("users", Readwrite)?;
   let data = expensive_calculation().await;  // Blocks other transactions!
   tx.object_store("users")?.add(data)?.await?;

   // GOOD: Prepare data first
   let data = expensive_calculation().await;
   let tx = db.transaction_on_one_with_mode("users", Readwrite)?;
   tx.object_store("users")?.add(data)?.await?;
   tx.await.into_result()?;
   ```

2. **Use readonly when possible** - Allows concurrency
   ```rust
   // Don't use Readwrite if you're only reading
   let tx = db.transaction_on_one("users")?;  // Readonly by default
   ```

3. **Always commit** - Await the transaction
   ```rust
   tx.await.into_result()?;  // Required for Readwrite
   ```

4. **Handle errors** - Transactions auto-rollback on error
   ```rust
   let result = async {
       let tx = db.transaction_on_one_with_mode("users", Readwrite)?;
       store.add(data1)?.await?;
       store.add(data2)?.await?;  // If this fails, data1 is rolled back
       tx.await.into_result()?;
       Ok(())
   }.await;
   ```

## Core Concepts

### 1. Database Lifecycle

IndexedDB uses a versioned database model:

1. **Open/Create Database** - Opens existing or creates new
2. **Upgrade Event** - Triggered when version changes, used to create/modify object stores
3. **Transaction** - All operations occur within transactions
4. **Object Store** - Similar to tables in SQL databases

### 2. Async Operations

All operations are asynchronous:

```rust
use indexed_db_futures::prelude::*;
use wasm_bindgen_futures::spawn_local;

spawn_local(async {
    let db = open_database().await.unwrap();
    // Use database
});
```

### 3. Object Stores and Indexes

Object stores hold data, indexes enable querying:

```rust
// Create object store with auto-incrementing key
let store = db.create_object_store("notes")
    .auto_increment(true)
    .build()?;

// Create index for querying
store.create_index("by_date")
    .key_path("created_at")
    .build()?;
```

## Step-by-Step: Building an Offline Note App

### Step 1: Database Setup

```rust
use indexed_db_futures::prelude::*;
use indexed_db_futures::IdbDatabase;
use wasm_bindgen::JsValue;
use serde::{Serialize, Deserialize};

const DB_NAME: &str = "notes_db";
const STORE_NAME: &str = "notes";
const DB_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Note {
    id: Option<u32>,
    title: String,
    content: String,
    created_at: f64,  // Timestamp
}

async fn open_db() -> Result<IdbDatabase, JsValue> {
    let mut db_req = IdbDatabase::open_u32(DB_NAME, DB_VERSION)?;

    // Set up upgrade handler for schema changes
    db_req.set_on_upgrade_needed(Some(move |evt: &IdbVersionChangeEvent| -> Result<(), JsValue> {
        // Get database from event
        if let Some(db) = evt.db() {
            // Create object store if it doesn't exist
            if !db.object_store_names().any(|n| &n == STORE_NAME) {
                let store_params = IdbObjectStoreParameters::new()
                    .auto_increment(true)
                    .key_path(Some(&"id".into()));

                let store = db.create_object_store(STORE_NAME, store_params)?;

                // Create index for sorting by date
                let index_params = IdbIndexParameters::new().unique(false);
                store.create_index_with_params(
                    "by_date",
                    &IdbKeyPath::str("created_at"),
                    index_params,
                )?;
            }
        }
        Ok(())
    }));

    db_req.await
}
```

### Step 2: CRUD Operations

```rust
use serde_wasm_bindgen::{to_value, from_value};
use web_sys::window;

impl Note {
    // Create new note
    async fn create(title: String, content: String) -> Result<u32, JsValue> {
        let db = open_db().await?;

        let timestamp = window()
            .unwrap()
            .performance()
            .unwrap()
            .now();

        let note = Note {
            id: None,
            title,
            content,
            created_at: timestamp,
        };

        // Start read-write transaction
        let tx = db.transaction_on_one_with_mode(STORE_NAME, IdbTransactionMode::Readwrite)?;
        let store = tx.object_store(STORE_NAME)?;

        // Add note
        let js_value = to_value(&note)?;
        let key = store.add_key_val(&JsValue::undefined(), &js_value)?.await?;

        // Commit transaction
        tx.await.into_result()?;

        // Extract ID
        let id = key.as_f64().ok_or("Invalid key")? as u32;
        Ok(id)
    }

    // Read single note
    async fn get(id: u32) -> Result<Option<Note>, JsValue> {
        let db = open_db().await?;

        let tx = db.transaction_on_one(STORE_NAME)?;
        let store = tx.object_store(STORE_NAME)?;

        let js_value = store.get(&JsValue::from_f64(id as f64))?.await?;

        if js_value.is_undefined() {
            Ok(None)
        } else {
            let note: Note = from_value(js_value)?;
            Ok(Some(note))
        }
    }

    // Read all notes
    async fn list_all() -> Result<Vec<Note>, JsValue> {
        let db = open_db().await?;

        let tx = db.transaction_on_one(STORE_NAME)?;
        let store = tx.object_store(STORE_NAME)?;

        // Get all entries
        let array = store.get_all()?.await?;
        let mut notes = Vec::new();

        for i in 0..array.length() {
            if let Some(js_val) = array.get(i) {
                let note: Note = from_value(js_val)?;
                notes.push(note);
            }
        }

        Ok(notes)
    }

    // Update note
    async fn update(&self) -> Result<(), JsValue> {
        let db = open_db().await?;

        let tx = db.transaction_on_one_with_mode(STORE_NAME, IdbTransactionMode::Readwrite)?;
        let store = tx.object_store(STORE_NAME)?;

        let js_value = to_value(self)?;
        store.put_key_val(&JsValue::from_f64(self.id.unwrap() as f64), &js_value)?.await?;

        tx.await.into_result()?;

        Ok(())
    }

    // Delete note
    async fn delete(id: u32) -> Result<(), JsValue> {
        let db = open_db().await?;

        let tx = db.transaction_on_one_with_mode(STORE_NAME, IdbTransactionMode::Readwrite)?;
        let store = tx.object_store(STORE_NAME)?;

        store.delete(&JsValue::from_f64(id as f64))?.await?;

        tx.await.into_result()?;

        Ok(())
    }
}
```

### Step 3: Using Indexes for Queries

```rust
impl Note {
    // Get notes sorted by date (newest first)
    async fn list_by_date_desc() -> Result<Vec<Note>, JsValue> {
        let db = open_db().await?;

        let tx = db.transaction_on_one(STORE_NAME)?;
        let store = tx.object_store(STORE_NAME)?;
        let index = store.index("by_date")?;

        // Open cursor in reverse direction
        let mut cursor = index
            .open_cursor_with_direction(web_sys::IdbCursorDirection::Prev)?
            .await?;

        let mut notes = Vec::new();

        while let Some(c) = cursor {
            if let Some(js_val) = c.value() {
                let note: Note = from_value(js_val)?;
                notes.push(note);
            }

            cursor = c.continue_cursor()?.await?;
        }

        Ok(notes)
    }

    // Search by title (simple contains)
    async fn search(query: &str) -> Result<Vec<Note>, JsValue> {
        let all_notes = Self::list_all().await?;

        let query_lower = query.to_lowercase();
        let results = all_notes
            .into_iter()
            .filter(|note| {
                note.title.to_lowercase().contains(&query_lower) ||
                note.content.to_lowercase().contains(&query_lower)
            })
            .collect();

        Ok(results)
    }
}
```

## Cursors vs get_all(): Decision Guide

**Two ways to retrieve multiple records:**

### **get_all()** - Load everything at once
```rust
let array = store.get_all()?.await?;
for i in 0..array.length() {
    let item = array.get(i);
    // Process item
}
```

**How it works:** Fetches all records into memory as a JavaScript array, then iterate over it.

### **Cursor** - Stream one at a time
```rust
let mut cursor = store.open_cursor()?.await?;
while let Some(c) = cursor {
    let item = c.value();
    // Process item
    cursor = c.continue_cursor()?.await?;
}
```

**How it works:** Iterates through records one by one, fetching only current record into memory.

---

## When to Use Which?

**Decision table:**

| Scenario | Use | Why |
|----------|-----|-----|
| **< 1000 records** | `get_all()` | Simple, fast, low memory overhead |
| **> 1000 records** | Cursor | Memory efficient, prevents browser freeze |
| **Need all data at once** | `get_all()` | Single operation, easier to work with |
| **Processing large datasets** | Cursor | Stream processing, can handle millions of records |
| **Sorting by index** | Cursor with direction | Natural index ordering |
| **Filtering during iteration** | Cursor | Can skip records efficiently with `advance()` |
| **Simple queries** | `get_all()` + filter | Easier to code, more Rust-idiomatic |
| **Complex filtering** | Cursor | Fine-grained control, can stop early |
| **Transforming all data** | `get_all()` | Use `.map()` and `.filter()` naturally |
| **Finding first match** | Cursor | Stop as soon as found, don't load rest |

---

## Detailed Comparison

### **Memory Usage**

```
get_all() with 10,000 records:
┌──────────────────────────────────┐
│   All 10,000 in memory           │
│   ~10MB (if 1KB each)            │
└──────────────────────────────────┘

Cursor with 10,000 records:
┌──────┐
│  1   │ Load, process, discard
├──────┤
│  2   │ Load, process, discard
├──────┤
│  3   │ Load, process, discard
└──────┘
Peak memory: ~1KB (one record at a time)
```

### **Performance**

**get_all() is faster for:**
- Small datasets (< 1000 records)
- When you need all data anyway
- Simple filtering/mapping operations

**Cursor is faster for:**
- Large datasets (> 1000 records)
- Early termination (find first match)
- Memory-constrained environments (mobile browsers)

### **Code Complexity**

**get_all() is simpler:**
```rust
// Easy: Get all, then filter
let notes = Note::list_all().await?;
let recent: Vec<_> = notes
    .into_iter()
    .filter(|n| n.timestamp > cutoff)
    .take(10)
    .collect();
```

**Cursor is more complex but powerful:**
```rust
// More control: Filter while iterating
let mut cursor = store.index("by_date")?
    .open_cursor_with_direction(IdbCursorDirection::Prev)?
    .await?;

let mut results = Vec::new();
while let Some(c) = cursor {
    let note: Note = from_value(c.value().unwrap())?;

    if note.timestamp > cutoff {
        results.push(note);
        if results.len() >= 10 {
            break;  // Stop early!
        }
    }

    cursor = c.continue_cursor()?.await?;
}
```

---

## Cursor Advanced Features

**1. Direction control (sorted results):**
```rust
// Newest first
let cursor = index.open_cursor_with_direction(IdbCursorDirection::Prev)?.await?;

// Oldest first
let cursor = index.open_cursor_with_direction(IdbCursorDirection::Next)?.await?;
```

**2. Skip records efficiently:**
```rust
// Skip first 100 records
let cursor = store.open_cursor()?.await?;
if let Some(c) = cursor {
    cursor = c.advance(100)?.await?;  // Skip to record 101
}
```

**3. Range queries with cursors:**
```rust
use web_sys::IdbKeyRange;

// Only records where key is between 100 and 200
let range = IdbKeyRange::bound(&100.into(), &200.into())?;
let cursor = store.open_cursor_with_range(&range)?.await?;
```

---

## Practical Examples

### Example 1: Small dataset - Use get_all()
```rust
// Task: Get all user preferences (< 50 records)
async fn get_all_preferences() -> Result<Vec<Preference>, JsValue> {
    let db = open_db().await?;
    let tx = db.transaction_on_one("preferences")?;
    let store = tx.object_store("preferences")?;

    let array = store.get_all()?.await?;
    let mut prefs = Vec::new();

    for i in 0..array.length() {
        if let Some(js_val) = array.get(i) {
            prefs.push(from_value(js_val)?);
        }
    }

    Ok(prefs)
}
```

### Example 2: Large dataset - Use cursor
```rust
// Task: Process 100,000 chat messages one by one
async fn process_all_messages() -> Result<u32, JsValue> {
    let db = open_db().await?;
    let tx = db.transaction_on_one("messages")?;
    let store = tx.object_store("messages")?;

    let mut cursor = store.open_cursor()?.await?;
    let mut count = 0;

    while let Some(c) = cursor {
        if let Some(js_val) = c.value() {
            let msg: Message = from_value(js_val)?;
            process_message(&msg).await?;  // Process one at a time
            count += 1;
        }

        cursor = c.continue_cursor()?.await?;
    }

    Ok(count)
}
```

### Example 3: Find first match - Use cursor
```rust
// Task: Find first message from specific user
async fn find_first_message_by_user(user_id: u32) -> Result<Option<Message>, JsValue> {
    let db = open_db().await?;
    let tx = db.transaction_on_one("messages")?;
    let store = tx.object_store("messages")?;
    let index = store.index("by_user")?;

    // Use cursor with key range (more efficient than get_all + filter)
    let range = IdbKeyRange::only(&user_id.into())?;
    let cursor = index.open_cursor_with_range(&range)?.await?;

    if let Some(c) = cursor {
        if let Some(js_val) = c.value() {
            return Ok(Some(from_value(js_val)?));
        }
    }

    Ok(None)
}
```

---

## Summary: Quick Decision Flowchart

```
How many records?
├─ < 1000 → Use get_all()
│           Simple, fast enough
│
└─ > 1000 → Do you need all of them?
            ├─ Yes → Use get_all() if memory allows
            │        Otherwise → Cursor
            │
            └─ No (filtering/searching) → Cursor
                                          Can stop early
```

## How dialect-coach Uses indexed_db_futures

### 1. Dependency Declaration (frontend/Cargo.toml:57)

```toml
indexed_db_futures = "0.5"
```

### 2. Persistence Service (frontend/src/services/persistence.rs)

The dialect-coach project has a persistence service for storing session data and chat history locally, allowing the application to work offline and restore state after page refreshes.

**Common Use Cases:**
- Store chat history
- Cache user preferences
- Offline message queue
- Session restoration

## Best Practices

1. **Version management** - Increment version for schema changes
2. **Transaction scope** - Keep transactions focused and short-lived
3. **Error handling** - Always handle database operation errors
4. **Serialization** - Use serde with `serde-wasm-bindgen` for type safety
5. **Indexes sparingly** - Only create indexes for frequent queries
6. **Transaction modes** - Use readonly when not modifying data

## Common Patterns

### Pattern 1: Transaction Template

```rust
async fn transaction_template() -> Result<(), JsValue> {
    let db = open_db().await?;

    let tx = db.transaction_on_one_with_mode(
        STORE_NAME,
        IdbTransactionMode::Readwrite
    )?;

    let store = tx.object_store(STORE_NAME)?;

    // Perform operations
    store.add_key_val(&key, &value)?.await?;

    // Commit by awaiting transaction
    tx.await.into_result()?;

    Ok(())
}
```

### Pattern 2: Cursor Iteration

```rust
async fn iterate_store() -> Result<Vec<Item>, JsValue> {
    let db = open_db().await?;
    let tx = db.transaction_on_one(STORE_NAME)?;
    let store = tx.object_store(STORE_NAME)?;

    let mut cursor = store.open_cursor()?.await?;
    let mut items = Vec::new();

    while let Some(c) = cursor {
        if let Some(value) = c.value() {
            let item: Item = from_value(value)?;
            items.push(item);
        }

        cursor = c.continue_cursor()?.await?;
    }

    Ok(items)
}
```

## Troubleshooting

### Issue: "VersionError" on database open

**Cause:** Version number decreased or schema mismatch

**Solution:** Always increment version for schema changes, never decrement.

### Issue: "ReadOnlyError" when writing

**Cause:** Transaction opened in readonly mode

**Solution:** Use `IdbTransactionMode::Readwrite`:

```rust
let tx = db.transaction_on_one_with_mode(
    STORE_NAME,
    IdbTransactionMode::Readwrite
)?;
```

### Issue: Data not persisting

**Cause:** Transaction not committed

**Solution:** Always await the transaction:

```rust
tx.await.into_result()?;
```

## Further Resources

- **Crate:** https://crates.io/crates/indexed_db_futures
- **Docs:** https://docs.rs/indexed_db_futures/latest/indexed_db_futures/
- **MDN IndexedDB:** https://developer.mozilla.org/en-US/docs/Web/API/IndexedDB_API
- **Examples:** https://github.com/Alorel/rust-indexed-db

## Summary

indexed_db_futures provides async IndexedDB access:

- **Async/await** API for all operations
- **Versioned databases** with upgrade handlers
- **Object stores** for structured data
- **Indexes** for efficient querying
- **Transactions** for atomic operations
- **Type-safe** with serde integration

IndexedDB enables building offline-capable web applications with persistent storage in the browser, perfect for PWAs and applications that need to work without a network connection.
