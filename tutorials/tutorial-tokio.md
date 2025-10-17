# Tokio Tutorial: Asynchronous Runtime for Rust

## Overview

**Tokio** is an asynchronous runtime for Rust that provides the building blocks for writing network applications. It includes an async/await runtime, async I/O, timers, and synchronization primitives.

**Version used in dialect-coach:** `1.x` with "full" features

**Why we use it:** Industry-standard async runtime enabling high-performance concurrent applications with async/await syntax.

## Dependencies

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
# Or selective features:
tokio = { version = "1", features = ["rt-multi-thread", "macros", "sync", "time"] }
```

## Core Concepts

### 1. The Runtime

Tokio's runtime executes async tasks:

```rust
#[tokio::main]
async fn main() {
    println!("Hello from async!");
}

// Equivalent to:
fn main() {
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            println!("Hello from async!");
        })
}
```

## Understanding Async Runtimes

**What is a runtime?** An async runtime is an event loop that executes async tasks. Without a runtime, async functions can't run.

**How it works:**
1. **Event loop** - Continuously checks for work to do
2. **Task scheduler** - Decides which task to run next
3. **Reactor** - Monitors I/O events (network, files, timers)
4. **Executor** - Runs tasks when they're ready

**The execution model:**
```
Application Code
      ↓
   tokio::spawn(async task)
      ↓
Task Scheduler
      ↓
[Task Queue: task1, task2, task3, ...]
      ↓
Executor (runs tasks on worker threads)
      ↓
Reactor (wakes tasks when I/O ready)
      ↓
Event Loop (coordinates everything)
```

**What `#[tokio::main]` does:**
1. Creates a runtime with default configuration
2. Wraps your async main function
3. Runs `block_on` to execute it
4. Handles shutdown when main completes

**Cooperative scheduling:** Unlike OS threads, async tasks cooperate:
- Tasks yield at `.await` points
- Long-running computation blocks other tasks
- Solution: Use `tokio::task::spawn_blocking` for CPU-heavy work

## OS Threads vs Async Tasks

Understanding the difference is crucial:

| Feature | OS Threads | Async Tasks |
|---------|-----------|-------------|
| **Creation cost** | High (~2MB stack) | Low (~few KB) |
| **Context switch** | Kernel-level (expensive) | User-level (cheap) |
| **Scheduling** | Preemptive (OS decides) | Cooperative (at .await) |
| **Count** | ~1000s max | ~100,000s possible |
| **Blocking** | Blocks thread only | Blocks entire executor |
| **Use case** | CPU-bound work | I/O-bound work |

**When to use threads:**
```rust
// CPU-intensive work
tokio::task::spawn_blocking(|| {
    // Heavy computation that takes time
    expensive_calculation()
});
```

**When to use async tasks:**
```rust
// I/O-bound work
tokio::spawn(async {
    // Mostly waiting for I/O
    let data = fetch_from_network().await;
    let result = save_to_database(data).await;
});
```

**Key insight:** Async shines when you have many concurrent operations that spend most of their time waiting (network requests, database queries). Threads are better for CPU-intensive work that rarely waits.

### 2. Spawning Tasks

Run tasks concurrently:

```rust
use tokio::task;

#[tokio::main]
async fn main() {
    let task1 = task::spawn(async {
        // Do work
        "result1"
    });

    let task2 = task::spawn(async {
        // Do work concurrently
        "result2"
    });

    let (res1, res2) = tokio::join!(task1, task2);
    println!("{:?}, {:?}", res1, res2);
}
```

### 3. Channels

Communication between tasks:

```rust
use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    let (tx, mut rx) = mpsc::unbounded_channel();

    tokio::spawn(async move {
        tx.send("message").unwrap();
    });

    while let Some(msg) = rx.recv().await {
        println!("Received: {}", msg);
    }
}
```

### 4. Synchronization Primitives

Tokio provides async-aware synchronization:

- **Mutex** - Mutual exclusion
- **RwLock** - Reader-writer lock
- **Semaphore** - Limit concurrent access
- **Notify** - Wake tasks

```rust
use tokio::sync::Mutex;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let data = Arc::new(Mutex::new(0));

    let data_clone = data.clone();
    tokio::spawn(async move {
        let mut lock = data_clone.lock().await;
        *lock += 1;
    }).await.unwrap();

    println!("Value: {}", *data.lock().await);
}
```

## Synchronization Primitives Explained

**What are synchronization primitives?** Tools to coordinate access to shared resources between tasks/threads.

**Semaphore - Limiting Concurrency:**
```rust
use tokio::sync::Semaphore;
use std::sync::Arc;

// What it does: Limits how many tasks can access a resource simultaneously
// Use case: Rate limiting, connection pooling, resource management

let semaphore = Arc::new(Semaphore::new(3));  // Allow 3 concurrent

for i in 0..10 {
    let permit = semaphore.clone();
    tokio::spawn(async move {
        // This will block if 3 tasks already hold permits
        let _permit = permit.acquire().await.unwrap();
        println!("Task {} running (max 3 concurrent)", i);
        expensive_operation().await;
        // Permit automatically released when _permit is dropped
    });
}
```

**Why semaphores matter:**
- Prevent overwhelming external services (e.g., only 5 concurrent DB connections)
- Control resource usage (e.g., limit parallel file operations)
- Implement rate limiting

**Mutex - Exclusive Access:**
```rust
// What it does: Ensures only one task modifies data at a time
// Use case: Protecting shared state

let counter = Arc::new(Mutex::new(0));

// Multiple tasks can safely increment
for _ in 0..100 {
    let counter = counter.clone();
    tokio::spawn(async move {
        let mut lock = counter.lock().await;  // Wait for exclusive access
        *lock += 1;
        // Lock released when `lock` is dropped
    });
}
```

**Notify - Wake Waiting Tasks:**
```rust
use tokio::sync::Notify;

// What it does: Signal to waiting tasks that something happened
// Use case: Producer-consumer patterns, event notifications

let notify = Arc::new(Notify::new());

// Task 1: Wait for notification
let notify_clone = notify.clone();
tokio::spawn(async move {
    notify_clone.notified().await;  // Blocks until notified
    println!("Got notification!");
});

// Task 2: Send notification
tokio::spawn(async move {
    tokio::time::sleep(Duration::from_secs(1)).await;
    notify.notify_one();  // Wake one waiting task
});
```

## How `tokio::select!` Works

**What is select?** A macro that waits on multiple async operations simultaneously and proceeds with whichever completes first.

**The polling mechanism:**
```rust
tokio::select! {
    result1 = async_operation_1() => {
        // This branch runs if operation_1 completes first
    }
    result2 = async_operation_2() => {
        // This branch runs if operation_2 completes first
    }
}
```

**How it works internally:**
1. Polls all futures simultaneously (not sequentially!)
2. Whichever future completes first, its branch executes
3. Other futures are dropped (cancelled)

**Common use cases:**

**1. Timeout pattern:**
```rust
use tokio::time::{sleep, Duration, timeout};

// Run with timeout
tokio::select! {
    result = long_operation() => {
        println!("Completed: {:?}", result);
    }
    _ = sleep(Duration::from_secs(5)) => {
        println!("Timed out after 5 seconds!");
    }
}
```

**2. Cancellation pattern:**
```rust
use tokio::sync::mpsc;

let (tx, mut rx) = mpsc::unbounded_channel::<()>();

tokio::select! {
    _ = long_running_task() => {
        println!("Task completed");
    }
    _ = rx.recv() => {
        println!("Task cancelled via channel");
    }
}
```

**3. Racing multiple operations:**
```rust
tokio::select! {
    data = fetch_from_cache() => {
        println!("Got from cache: {:?}", data);
    }
    data = fetch_from_database() => {
        println!("Got from database: {:?}", data);
    }
    data = fetch_from_api() => {
        println!("Got from API: {:?}", data);
    }
}
// First to complete wins, others are cancelled
```

**Important: Cancellation safety** - Operations in select! are cancelled if they don't complete first. Make sure your async code handles cancellation properly (no half-written files, incomplete transactions, etc.).

## Step-by-Step: Building a Concurrent Web Scraper

### Step 1: Basic Async Function

```rust
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() {
    println!("Starting...");
    work().await;
    println!("Done!");
}

async fn work() {
    println!("Working...");
    sleep(Duration::from_secs(1)).await;
    println!("Work complete!");
}
```

### Step 2: Spawning Multiple Tasks

```rust
use tokio::task;

#[tokio::main]
async fn main() {
    let mut handles = vec![];

    for i in 0..10 {
        let handle = task::spawn(async move {
            fetch_url(i).await
        });
        handles.push(handle);
    }

    for handle in handles {
        match handle.await {
            Ok(result) => println!("Got: {:?}", result),
            Err(e) => eprintln!("Task failed: {}", e),
        }
    }
}

async fn fetch_url(id: usize) -> String {
    sleep(Duration::from_millis(100)).await;
    format!("Result from task {}", id)
}
```

### Step 3: Using Channels for Results

```rust
use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    let (tx, mut rx) = mpsc::unbounded_channel();

    // Spawn 10 workers
    for i in 0..10 {
        let tx = tx.clone();
        tokio::spawn(async move {
            let result = fetch_url(i).await;
            tx.send((i, result)).unwrap();
        });
    }

    // Drop original sender so rx can close
    drop(tx);

    // Collect results
    while let Some((id, result)) = rx.recv().await {
        println!("Task {} completed: {}", id, result);
    }
}
```

### Step 4: Rate Limiting with Semaphore

```rust
use tokio::sync::Semaphore;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    // Limit to 3 concurrent requests
    let semaphore = Arc::new(Semaphore::new(3));
    let mut handles = vec![];

    for i in 0..10 {
        let permit = semaphore.clone();
        let handle = tokio::spawn(async move {
            let _permit = permit.acquire().await.unwrap();
            println!("Task {} running", i);
            fetch_url(i).await;
            println!("Task {} done", i);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }
}
```

### Step 5: Select for Racing Futures

```rust
use tokio::time::{sleep, Duration, timeout};

#[tokio::main]
async fn main() {
    let result = timeout(
        Duration::from_secs(5),
        slow_operation()
    ).await;

    match result {
        Ok(value) => println!("Completed: {}", value),
        Err(_) => println!("Timed out!"),
    }
}

async fn slow_operation() -> String {
    sleep(Duration::from_secs(10)).await;
    "Done".to_string()
}

// Manual select example
async fn race_tasks() {
    tokio::select! {
        result1 = task1() => {
            println!("Task 1 won: {}", result1);
        }
        result2 = task2() => {
            println!("Task 2 won: {}", result2);
        }
    }
}
```

## How dialect-coach Uses Tokio

### 1. Main Runtime (main.rs:31, corpus-processor/src/main.rs:68)

Application entry point:

```rust
#[tokio::main]
async fn main() -> Result<()> {
    // Load environment
    dotenvy::dotenv().ok();

    // Initialize services (async operations)
    let qdrant = QdrantService::from_env().await?;
    let agent = AgentService::from_env(qdrant.clone())?;

    // Start server
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
```

**Pattern:** `#[tokio::main]` transforms async main into synchronous entry point.

### 2. Spawning Tasks (websocket.rs:85-96, 104-241)

Independent send/receive tasks:

```rust
// Spawn send task
let mut send_task = tokio::spawn(async move {
    while let Some(msg) = rx.recv().await {
        eprintln!("[WEBSOCKET] Sending message");
        if let Err(e) = sender.send(WsMessage::Text(msg.into())).await {
            eprintln!("[WEBSOCKET] Failed to send: {}", e);
            tracing::error!("Failed to send message: {}", e);
            break;
        }
    }
    eprintln!("[WEBSOCKET] Send task exiting");
});

// Spawn receive task
let mut recv_task = tokio::spawn(async move {
    while let Some(Ok(msg)) = receiver.next().await {
        if let WsMessage::Text(text) = msg {
            // Process message...
        }
    }
});
```

**Pattern:** Spawn independent tasks for concurrent operations. Each task runs until completion or error.

### 3. Task Coordination with select! (websocket.rs:244-256)

Graceful shutdown when either task completes:

```rust
// Wait for either task to finish
tokio::select! {
    _ = (&mut send_task) => {
        eprintln!("[WEBSOCKET] send_task finished first");
        recv_task.abort();
    }
    _ = (&mut recv_task) => {
        eprintln!("[WEBSOCKET] recv_task finished");
        drop(tx);  // Signal send_task to finish
        let _ = send_task.await;
    }
}
```

**Pattern:** Use `tokio::select!` to wait for first completion and cleanup remaining tasks.

### 4. Shared State with Mutex (main.rs:16, 28)

Thread-safe shared state:

```rust
use tokio::sync::Mutex;

pub struct AppState {
    pub session_histories: Arc<Mutex<HashMap<Uuid, Vec<String>>>>,
}

// Usage in handler:
let mut histories = state.session_histories.lock().await;
let history = histories.entry(session_id).or_insert_with(Vec::new);
history.push(format!("User: {}", message));
```

**Pattern:** Use `tokio::sync::Mutex` for async-friendly locking. Always await lock acquisition.

### 5. Channels for Message Passing (websocket.rs:82)

Decoupled communication:

```rust
let (tx, mut rx) = mpsc::unbounded_channel::<String>();

// Producer
tx.send(message).unwrap();

// Consumer
while let Some(msg) = rx.recv().await {
    // Process message
}
```

**Pattern:** Use unbounded channels for flexible message passing. Bounded channels when backpressure is needed.

### 6. TcpListener Binding (main.rs:112-115)

Async network I/O:

```rust
let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
let listener = tokio::net::TcpListener::bind(addr).await?;
tracing::info!("Server listening on {}", addr);
axum::serve(listener, app).await?;
```

**Pattern:** Bind listener asynchronously, then serve indefinitely.

## Common Patterns

### Pattern 1: Concurrent HTTP Requests

```rust
use tokio::task::JoinSet;

async fn fetch_many(urls: Vec<String>) -> Vec<String> {
    let mut set = JoinSet::new();

    for url in urls {
        set.spawn(async move {
            fetch(url).await
        });
    }

    let mut results = vec![];
    while let Some(res) = set.join_next().await {
        if let Ok(data) = res {
            results.push(data);
        }
    }

    results
}
```

### Pattern 2: Timeout with Fallback

```rust
use tokio::time::{timeout, Duration};

async fn fetch_with_fallback(url: &str) -> String {
    match timeout(Duration::from_secs(5), fetch(url)).await {
        Ok(result) => result,
        Err(_) => "Default value".to_string(),
    }
}
```

### Pattern 3: Retry with Exponential Backoff

```rust
async fn retry_with_backoff<F, Fut, T>(mut f: F, max_attempts: u32) -> Option<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Option<T>>,
{
    let mut delay = Duration::from_millis(100);

    for _ in 0..max_attempts {
        if let Some(result) = f().await {
            return Some(result);
        }

        sleep(delay).await;
        delay *= 2;
    }

    None
}
```

### Pattern 4: Broadcasting to Multiple Receivers

```rust
use tokio::sync::broadcast;

#[tokio::main]
async fn main() {
    let (tx, _rx) = broadcast::channel(16);

    for i in 0..3 {
        let mut rx = tx.subscribe();
        tokio::spawn(async move {
            while let Ok(msg) = rx.recv().await {
                println!("Receiver {} got: {}", i, msg);
            }
        });
    }

    tx.send("Hello".to_string()).unwrap();
    sleep(Duration::from_millis(100)).await;
}
```

## Best Practices from dialect-coach

1. **Spawn independent tasks** - Don't block one task waiting for another
2. **Use tokio::select! for coordination** - Gracefully handle multiple futures
3. **Drop channel senders** - Signal completion to receivers
4. **Abort tasks on shutdown** - Clean up spawned tasks
5. **tokio::sync primitives** - Use tokio's Mutex/RwLock, not std's
6. **Error propagation** - Use `?` with `Result` in async functions
7. **Resource cleanup** - Always handle task join errors

## Troubleshooting

### Issue: "Cannot start a runtime from within a runtime"

**Cause:** Calling `block_on` or creating runtime inside async context

**Solution:** Use `tokio::spawn` instead:

```rust
// Wrong
async fn bad() {
    tokio::runtime::Runtime::new().unwrap().block_on(async { });
}

// Correct
async fn good() {
    tokio::spawn(async { }).await.unwrap();
}
```

### Issue: Deadlock with Mutex

**Cause:** Holding lock across `.await` point

**Solution:** Drop lock before awaiting:

```rust
// Wrong - potential deadlock
{
    let lock = mutex.lock().await;
    some_async_fn().await;  // Still holding lock!
}

// Correct
{
    let value = {
        let lock = mutex.lock().await;
        lock.clone()  // Copy what you need
    };  // Lock dropped here
    some_async_fn().await;
}
```

### Issue: Task never completes

**Cause:** Not awaiting spawned task

**Solution:** Join or await task handle:

```rust
let handle = tokio::spawn(async { });
handle.await.unwrap();  // Wait for completion
```

### Issue: "Send bound not satisfied"

**Cause:** Trying to send non-Send type across tasks

**Solution:** Use Send-safe types or avoid sending:

```rust
// Wrong: Rc is not Send
let rc = Rc::new(5);
tokio::spawn(async move {
    println!("{}", rc);  // Error!
});

// Correct: Arc is Send
let arc = Arc::new(5);
tokio::spawn(async move {
    println!("{}", arc);  // OK
});
```

## Further Resources

- **Official Tutorial:** https://tokio.rs/tokio/tutorial
- **API Docs:** https://docs.rs/tokio/latest/tokio/
- **GitHub:** https://github.com/tokio-rs/tokio
- **Examples:** https://github.com/tokio-rs/tokio/tree/master/examples
- **Async Book:** https://rust-lang.github.io/async-book/

## Summary

Tokio provides async runtime capabilities:

- **#[tokio::main]** for async entry point
- **tokio::spawn** for concurrent tasks
- **tokio::select!** for racing futures
- **Channels** (mpsc, broadcast, oneshot) for communication
- **tokio::sync** primitives (Mutex, RwLock, Semaphore)
- **Timers** for delays and timeouts
- **Async I/O** for network and file operations

The dialect-coach project demonstrates production patterns including task spawning for WebSocket communication, select-based task coordination, channel-based message passing, and shared state management with async mutexes.
