# Gloo Tutorial: WASM Utilities for Modern Web Development

## Overview

**Gloo** is a modular toolkit of Rust crates for web development with WASM. It provides ergonomic wrappers around web APIs, including WebSockets, timers, storage, and more. It's designed to work seamlessly with Yew and other WASM frameworks.

**Versions used in dialect-coach:**
- `gloo` 0.11
- `gloo-net` 0.5
- `gloo-timers` 0.3

**Why we use it:** Provides higher-level, futures-based APIs that are more ergonomic than raw web-sys, with built-in error handling and async/await support.

## Dependencies

```toml
[dependencies]
gloo = "0.11"
gloo-net = "0.5"
gloo-timers = "0.3"
futures-channel = "0.3"
futures-util = "0.3"
```

## Core Modules

Gloo is organized into focused modules:

- **gloo-net** - HTTP requests and WebSockets
- **gloo-timers** - setTimeout/setInterval equivalents
- **gloo-console** - Console logging
- **gloo-storage** - localStorage/sessionStorage
- **gloo-events** - Event listeners
- **gloo-utils** - DOM utilities

## Core Concepts

### 1. Futures-Based WebSocket API

Gloo provides a futures-based WebSocket that integrates with async/await:

```rust
use gloo_net::websocket::{futures::WebSocket, Message};
use futures_util::{SinkExt, StreamExt};

async fn websocket_example() {
    let mut ws = WebSocket::open("ws://localhost:8080").unwrap();
    let (mut write, mut read) = ws.split();

    // Send message
    write.send(Message::Text("Hello".to_string())).await.unwrap();

    // Receive message
    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => println!("Received: {}", text),
            _ => {}
        }
    }
}
```

## WebSocket Split Pattern Explained

**Why split?** WebSockets are bidirectional—you send AND receive on the same connection. But async Rust needs separate tasks for concurrent operations.

**The split pattern:**
```rust
let ws = WebSocket::open("ws://localhost:8080")?;
let (mut write, mut read) = ws.split();
//   ^^^^^^^^^^  ^^^^^^^^^^
//   SplitSink   SplitStream
```

**What happens:**
1. **`SplitSink`** (write half) - Implements `Sink` trait, allows sending messages
2. **`SplitStream`** (read half) - Implements `Stream` trait, allows receiving messages
3. **Independent use** - Each half can be moved into different tasks

**Why this matters:**
- **No blocking** - Sending doesn't block receiving, receiving doesn't block sending
- **Concurrent operations** - Can spawn separate tasks for send/receive loops
- **Backpressure** - Each half can apply pressure independently
- **Clean separation** - Send logic separate from receive logic

**Common pattern:**
```rust
let (mut write, mut read) = ws.split();

// Spawn send task
spawn_local(async move {
    while let Some(msg) = tx.recv().await {
        let _ = write.send(msg).await;
    }
});

// Spawn receive task
spawn_local(async move {
    while let Some(msg) = read.next().await {
        // Handle incoming message
    }
});
```

**Without split:** You'd need complex state machines or locks to coordinate sending/receiving, defeating the purpose of async.

### 2. Timers with Callbacks

Simple timer API for delays and intervals:

```rust
use gloo_timers::callback::{Timeout, Interval};

// One-time delay
let timeout = Timeout::new(1000, move || {
    console::log!("Executed after 1 second");
});
timeout.forget(); // Keep alive

// Repeating interval
let interval = Interval::new(1000, move || {
    console::log!("Executes every second");
});
interval.forget();
```

### 3. Console Logging

Type-safe console logging:

```rust
use gloo_console::{log, warn, error};

log!("Simple message");
log!("Formatted:", 42, "items");
warn!("Warning message");
error!("Error occurred");
```

## Step-by-Step: Building a Real-Time Chat Client

Let's build a WebSocket-based chat with reconnection logic.

### Step 1: Basic WebSocket Connection

```rust
use gloo_net::websocket::{futures::WebSocket, Message as WsMessage};
use wasm_bindgen_futures::spawn_local;
use futures_util::{SinkExt, StreamExt};

pub fn connect_websocket(url: &str) {
    let url = url.to_string();

    spawn_local(async move {
        match WebSocket::open(&url) {
            Ok(ws) => {
                console::log!("Connected!");
                handle_websocket(ws).await;
            }
            Err(e) => {
                console::error!("Failed to connect:", format!("{:?}", e));
            }
        }
    });
}

async fn handle_websocket(ws: WebSocket) {
    let (mut write, mut read) = ws.split();

    // Send initial message
    if let Err(e) = write.send(WsMessage::Text("Hello Server!".to_string())).await {
        console::error!("Send failed:", format!("{:?}", e));
        return;
    }

    // Read messages
    while let Some(msg) = read.next().await {
        match msg {
            Ok(WsMessage::Text(text)) => {
                console::log!("Received:", text);
            }
            Ok(WsMessage::Bytes(data)) => {
                console::log!("Received binary:", data.len(), "bytes");
            }
            Err(e) => {
                console::error!("Error:", format!("{:?}", e));
                break;
            }
        }
    }

    console::log!("Connection closed");
}
```

### Step 2: WebSocket with Message Channel

```rust
use futures_channel::mpsc::{unbounded, UnboundedSender, UnboundedReceiver};

pub struct ChatClient {
    tx: UnboundedSender<String>,
}

impl ChatClient {
    pub fn connect(url: &str) -> Self {
        let (tx, rx) = unbounded::<String>();

        let url = url.to_string();
        spawn_local(async move {
            if let Ok(ws) = WebSocket::open(&url) {
                Self::run_websocket(ws, rx).await;
            }
        });

        Self { tx }
    }

    async fn run_websocket(ws: WebSocket, mut rx: UnboundedReceiver<String>) {
        let (mut write, mut read) = ws.split();

        // Spawn send task
        spawn_local(async move {
            while let Some(msg) = rx.next().await {
                if let Err(e) = write.send(WsMessage::Text(msg)).await {
                    console::error!("Send error:", format!("{:?}", e));
                    break;
                }
            }
        });

        // Read loop
        while let Some(msg) = read.next().await {
            if let Ok(WsMessage::Text(text)) = msg {
                console::log!("Received:", text);
            }
        }
    }

    pub fn send(&self, message: String) {
        let _ = self.tx.unbounded_send(message);
    }
}
```

### Step 3: Reconnection with Exponential Backoff

```rust
use gloo_timers::callback::Timeout;
use std::rc::Rc;
use std::cell::RefCell;

pub struct ReconnectingChat {
    url: String,
    tx: Rc<RefCell<Option<UnboundedSender<String>>>>,
    reconnect_delay: Rc<RefCell<u32>>,
    max_delay: u32,
}

impl ReconnectingChat {
    pub fn new(url: &str) -> Self {
        let chat = Self {
            url: url.to_string(),
            tx: Rc::new(RefCell::new(None)),
            reconnect_delay: Rc::new(RefCell::new(1000)),
            max_delay: 30000,
        };

        chat.connect();
        chat
    }

    fn connect(&self) {
        let url = self.url.clone();
        let tx_ref = self.tx.clone();
        let reconnect_delay = self.reconnect_delay.clone();
        let max_delay = self.max_delay;
        let self_url = self.url.clone();

        spawn_local(async move {
            match WebSocket::open(&url) {
                Ok(ws) => {
                    console::log!("Connected to", &url);

                    // Reset delay on successful connection
                    *reconnect_delay.borrow_mut() = 1000;

                    let (tx, rx) = unbounded::<String>();
                    *tx_ref.borrow_mut() = Some(tx);

                    Self::run_websocket_with_reconnect(
                        ws, rx, self_url, tx_ref.clone(),
                        reconnect_delay.clone(), max_delay
                    ).await;
                }
                Err(e) => {
                    console::error!("Connection failed:", format!("{:?}", e));
                    Self::schedule_reconnect(
                        self_url, tx_ref, reconnect_delay, max_delay
                    );
                }
            }
        });
    }

    async fn run_websocket_with_reconnect(
        ws: WebSocket,
        mut rx: UnboundedReceiver<String>,
        url: String,
        tx_ref: Rc<RefCell<Option<UnboundedSender<String>>>>,
        reconnect_delay: Rc<RefCell<u32>>,
        max_delay: u32,
    ) {
        let (mut write, mut read) = ws.split();

        // Send task
        spawn_local(async move {
            while let Some(msg) = rx.next().await {
                if write.send(WsMessage::Text(msg)).await.is_err() {
                    break;
                }
            }
        });

        // Read messages
        while let Some(msg) = read.next().await {
            if let Ok(WsMessage::Text(text)) = msg {
                console::log!("Message:", text);
            }
        }

        // Connection closed - schedule reconnect
        console::log!("Connection closed, reconnecting...");
        Self::schedule_reconnect(url, tx_ref, reconnect_delay, max_delay);
    }

    fn schedule_reconnect(
        url: String,
        tx_ref: Rc<RefCell<Option<UnboundedSender<String>>>>,
        reconnect_delay: Rc<RefCell<u32>>,
        max_delay: u32,
    ) {
        let delay = *reconnect_delay.borrow();
        console::log!("Reconnecting in", delay, "ms");

        // Exponential backoff
        let new_delay = (delay * 2).min(max_delay);
        *reconnect_delay.borrow_mut() = new_delay;

        // Schedule reconnection
        let url_clone = url.clone();
        let tx_ref_clone = tx_ref.clone();
        let reconnect_delay_clone = reconnect_delay.clone();

        Timeout::new(delay, move || {
            console::log!("Attempting to reconnect...");

            // Actually reconnect
            spawn_local(async move {
                match WebSocket::open(&url_clone) {
                    Ok(ws) => {
                        console::log!("Reconnected successfully!");

                        // Reset delay on successful connection
                        *reconnect_delay_clone.borrow_mut() = 1000;

                        let (tx, rx) = unbounded::<String>();
                        *tx_ref_clone.borrow_mut() = Some(tx);

                        // Run the WebSocket with same reconnection logic
                        Self::run_websocket_with_reconnect(
                            ws, rx, url_clone, tx_ref_clone.clone(),
                            reconnect_delay_clone.clone(), max_delay
                        ).await;
                    }
                    Err(e) => {
                        console::error!("Reconnect failed:", format!("{:?}", e));
                        // Try again with backoff
                        Self::schedule_reconnect(
                            url_clone, tx_ref_clone, reconnect_delay_clone, max_delay
                        );
                    }
                }
            });
        }).forget();
    }

    pub fn send(&self, message: String) {
        if let Some(tx) = self.tx.borrow().as_ref() {
            let _ = tx.unbounded_send(message);
        }
    }
}
```

### Step 4: Using Timers for Heartbeat

```rust
use gloo_timers::callback::Interval;

pub struct HeartbeatChat {
    tx: UnboundedSender<String>,
    _heartbeat: Interval,
}

impl HeartbeatChat {
    pub fn new(url: &str) -> Self {
        let (tx, rx) = unbounded::<String>();

        // Start WebSocket
        let url = url.to_string();
        spawn_local(async move {
            if let Ok(ws) = WebSocket::open(&url) {
                Self::run_websocket(ws, rx).await;
            }
        });

        // Send heartbeat every 30 seconds
        let tx_heartbeat = tx.clone();
        let heartbeat = Interval::new(30000, move || {
            let _ = tx_heartbeat.unbounded_send("ping".to_string());
        });

        Self {
            tx,
            _heartbeat: heartbeat,
        }
    }

    async fn run_websocket(ws: WebSocket, mut rx: UnboundedReceiver<String>) {
        let (mut write, mut read) = ws.split();

        spawn_local(async move {
            while let Some(msg) = rx.next().await {
                let _ = write.send(WsMessage::Text(msg)).await;
            }
        });

        while let Some(msg) = read.next().await {
            if let Ok(WsMessage::Text(text)) = msg {
                if text == "pong" {
                    console::log!("Heartbeat received");
                } else {
                    console::log!("Message:", text);
                }
            }
        }
    }

    pub fn send(&self, message: String) {
        let _ = self.tx.unbounded_send(message);
    }
}
```

### Step 5: Production-Ready Chat Client

Let's combine everything into a complete, production-ready chat implementation:

```rust
use gloo_net::websocket::{futures::WebSocket, Message as WsMessage};
use gloo_timers::callback::{Timeout, Interval};
use gloo_console as console;
use futures_channel::mpsc::{unbounded, UnboundedSender, UnboundedReceiver};
use futures_util::{SinkExt, StreamExt};
use wasm_bindgen_futures::spawn_local;
use std::rc::Rc;
use std::cell::RefCell;

pub struct ProductionChat {
    url: String,
    tx: Rc<RefCell<Option<UnboundedSender<String>>>>,
    reconnect_delay: Rc<RefCell<u32>>,
    max_delay: u32,
    _heartbeat: Rc<RefCell<Option<Interval>>>,
    on_message: Rc<dyn Fn(String)>,
    on_connect: Rc<dyn Fn()>,
    on_disconnect: Rc<dyn Fn()>,
}

impl ProductionChat {
    pub fn new(
        url: &str,
        on_message: impl Fn(String) + 'static,
        on_connect: impl Fn() + 'static,
        on_disconnect: impl Fn() + 'static,
    ) -> Self {
        let chat = Self {
            url: url.to_string(),
            tx: Rc::new(RefCell::new(None)),
            reconnect_delay: Rc::new(RefCell::new(1000)),
            max_delay: 30000,
            _heartbeat: Rc::new(RefCell::new(None)),
            on_message: Rc::new(on_message),
            on_connect: Rc::new(on_connect),
            on_disconnect: Rc::new(on_disconnect),
        };

        chat.connect();
        chat
    }

    fn connect(&self) {
        let url = self.url.clone();
        let tx_ref = self.tx.clone();
        let reconnect_delay = self.reconnect_delay.clone();
        let max_delay = self.max_delay;
        let self_url = self.url.clone();
        let heartbeat_ref = self._heartbeat.clone();
        let on_connect = self.on_connect.clone();
        let on_disconnect = self.on_disconnect.clone();
        let on_message = self.on_message.clone();

        spawn_local(async move {
            match WebSocket::open(&url) {
                Ok(ws) => {
                    console::log!("Connected successfully");

                    // Trigger connect callback
                    on_connect();

                    // Reset delay on successful connection
                    *reconnect_delay.borrow_mut() = 1000;

                    // Create channel for outgoing messages
                    let (tx, rx) = unbounded::<String>();
                    *tx_ref.borrow_mut() = Some(tx.clone());

                    // Start heartbeat
                    let tx_heartbeat = tx.clone();
                    let heartbeat = Interval::new(30000, move || {
                        console::log!("Sending heartbeat");
                        let _ = tx_heartbeat.unbounded_send("ping".to_string());
                    });
                    *heartbeat_ref.borrow_mut() = Some(heartbeat);

                    // Run WebSocket with handlers
                    Self::run_websocket_with_handlers(
                        ws, rx, on_message.clone()
                    ).await;

                    // Connection closed
                    console::log!("Connection closed");
                    on_disconnect();

                    // Clear heartbeat
                    *heartbeat_ref.borrow_mut() = None;

                    // Schedule reconnection
                    Self::schedule_reconnect(
                        self_url, tx_ref, reconnect_delay, max_delay,
                        heartbeat_ref, on_connect, on_disconnect, on_message
                    );
                }
                Err(e) => {
                    console::error!("Connection failed:", format!("{:?}", e));
                    on_disconnect();

                    // Schedule reconnection
                    Self::schedule_reconnect(
                        self_url, tx_ref, reconnect_delay, max_delay,
                        heartbeat_ref, on_connect, on_disconnect, on_message
                    );
                }
            }
        });
    }

    async fn run_websocket_with_handlers(
        ws: WebSocket,
        mut rx: UnboundedReceiver<String>,
        on_message: Rc<dyn Fn(String)>,
    ) {
        let (mut write, mut read) = ws.split();

        // Spawn send task
        spawn_local(async move {
            while let Some(msg) = rx.next().await {
                if let Err(e) = write.send(WsMessage::Text(msg)).await {
                    console::error!("Send error:", format!("{:?}", e));
                    break;
                }
            }
        });

        // Read messages
        while let Some(msg) = read.next().await {
            match msg {
                Ok(WsMessage::Text(text)) => {
                    if text == "pong" {
                        console::log!("Heartbeat acknowledged");
                    } else {
                        console::log!("Received message");
                        on_message(text);
                    }
                }
                Ok(WsMessage::Bytes(data)) => {
                    console::warn!("Received unexpected binary data:", data.len());
                }
                Err(e) => {
                    console::error!("WebSocket error:", format!("{:?}", e));
                    break;
                }
            }
        }
    }

    fn schedule_reconnect(
        url: String,
        tx_ref: Rc<RefCell<Option<UnboundedSender<String>>>>,
        reconnect_delay: Rc<RefCell<u32>>,
        max_delay: u32,
        heartbeat_ref: Rc<RefCell<Option<Interval>>>,
        on_connect: Rc<dyn Fn()>,
        on_disconnect: Rc<dyn Fn()>,
        on_message: Rc<dyn Fn(String)>,
    ) {
        let delay = *reconnect_delay.borrow();
        console::log!("Reconnecting in", delay, "ms");

        // Exponential backoff
        let new_delay = (delay * 2).min(max_delay);
        *reconnect_delay.borrow_mut() = new_delay;

        Timeout::new(delay, move || {
            console::log!("Attempting reconnection...");

            spawn_local(async move {
                match WebSocket::open(&url) {
                    Ok(ws) => {
                        console::log!("Reconnected!");
                        on_connect();

                        // Reset delay
                        *reconnect_delay.borrow_mut() = 1000;

                        let (tx, rx) = unbounded::<String>();
                        *tx_ref.borrow_mut() = Some(tx.clone());

                        // Restart heartbeat
                        let tx_heartbeat = tx.clone();
                        let heartbeat = Interval::new(30000, move || {
                            let _ = tx_heartbeat.unbounded_send("ping".to_string());
                        });
                        *heartbeat_ref.borrow_mut() = Some(heartbeat);

                        // Run connection
                        Self::run_websocket_with_handlers(ws, rx, on_message.clone()).await;

                        // Disconnected again
                        on_disconnect();
                        *heartbeat_ref.borrow_mut() = None;

                        // Reconnect again
                        Self::schedule_reconnect(
                            url, tx_ref, reconnect_delay, max_delay,
                            heartbeat_ref, on_connect, on_disconnect, on_message
                        );
                    }
                    Err(e) => {
                        console::error!("Reconnect failed:", format!("{:?}", e));
                        Self::schedule_reconnect(
                            url, tx_ref, reconnect_delay, max_delay,
                            heartbeat_ref, on_connect, on_disconnect, on_message
                        );
                    }
                }
            });
        }).forget();
    }

    pub fn send(&self, message: String) {
        if let Some(tx) = self.tx.borrow().as_ref() {
            if let Err(e) = tx.unbounded_send(message) {
                console::error!("Failed to queue message:", format!("{:?}", e));
            }
        } else {
            console::warn!("Not connected, message dropped");
        }
    }

    pub fn is_connected(&self) -> bool {
        self.tx.borrow().is_some()
    }
}

// Usage example
pub fn create_chat() {
    let chat = ProductionChat::new(
        "ws://localhost:8080/chat",
        |message| {
            console::log!("New message:", message);
            // Update UI with new message
        },
        || {
            console::log!("Connected!");
            // Show "connected" indicator in UI
        },
        || {
            console::log!("Disconnected!");
            // Show "connecting..." indicator in UI
        },
    );

    // Send messages
    chat.send("Hello from Rust!".to_string());
}
```

**What this demonstrates:**
- **Complete reconnection** - Actual working reconnection with exponential backoff
- **Heartbeat** - Automatic ping/pong to keep connection alive
- **Callbacks** - `on_message`, `on_connect`, `on_disconnect` for UI integration
- **Error handling** - Graceful handling of all error cases
- **Resource cleanup** - Proper cleanup of timers and channels
- **Production patterns** - Everything you need for a real application

**Key improvements over basic examples:**
1. **Automatic reconnection** that actually works
2. **Heartbeat mechanism** to detect dead connections
3. **Callback pattern** for easy UI integration
4. **State tracking** (`is_connected()`)
5. **Proper error messages** for debugging

## How dialect-coach Uses Gloo

### 1. WebSocket Imports (websocket.rs:2-3)

Using gloo for futures-based WebSocket:

```rust
use gloo_net::websocket::{Message as WsMessage, futures::WebSocket};
use futures_util::{SinkExt, StreamExt};
```

**Why:** `gloo_net::websocket::futures` provides async WebSocket that integrates naturally with Rust's async/await.

### 2. WebSocket Split Pattern (websocket.rs:160)

Separating read and write:

```rust
let (mut write, mut read) = ws.split();
```

**Pattern:** Split WebSocket into sink (write) and stream (read) for bidirectional communication in separate tasks.

### 3. Reconnection Timeout (websocket.rs:117-120, 251-256)

Using gloo-timers for delayed reconnection:

```rust
use gloo_timers::callback::Timeout;

// Schedule reconnection with exponential backoff
let timeout = Timeout::new(delay, move || {
    info!("Triggering reconnection from app layer");
    ws_clone.borrow_mut().reconnect();
});

*reconnection_timeout.borrow_mut() = Some(timeout);
```

**Pattern:** Store `Timeout` in state to keep it alive. It will auto-cancel when dropped.

### 4. Channel-Based Message Sending (websocket.rs:82, 180-188)

Using futures channels for async message queue:

```rust
use futures_channel::mpsc;

// Create unbounded channel
let (tx, mut rx) = mpsc::unbounded::<String>();

// Spawn send task
spawn_local(async move {
    while let Some(text) = rx.next().await {
        if let Err(e) = write.send(WsMessage::Text(text)).await {
            error!("Failed to send message: {:?}", e);
            break;
        }
    }
});
```

**Pattern:** Decouple sending from connection management. Application can queue messages without blocking.

### 5. Stream Processing (websocket.rs:205-229)

Reading messages from WebSocket stream:

```rust
while let Some(msg) = read.next().await {
    match msg {
        Ok(WsMessage::Text(text)) => {
            info!("Received WebSocket message: {} bytes", text.len());
            match serde_json::from_str::<Message>(&text) {
                Ok(parsed_msg) => {
                    on_message.emit(parsed_msg);
                }
                Err(e) => {
                    error!("Failed to parse message JSON: {}", e);
                }
            }
        }
        Ok(WsMessage::Bytes(bytes)) => {
            warn!("Received unexpected binary message");
        }
        Err(e) => {
            error!("WebSocket error: {:?}", e);
            break;
        }
    }
}
```

**Pattern:** Use `StreamExt::next()` to consume messages asynchronously until stream closes or errors.

## Common Patterns

### Pattern 1: Basic Timer

```rust
use gloo_timers::callback::Timeout;

// Execute once after delay
let timeout = Timeout::new(1000, || {
    console::log!("Delayed execution");
});

// Auto-cancels when dropped, or:
timeout.forget();  // Keep alive
```

### Pattern 2: Repeating Timer

```rust
use gloo_timers::callback::Interval;
use std::rc::Rc;
use std::cell::RefCell;

let count = Rc::new(RefCell::new(0));
let count_clone = count.clone();

let interval = Interval::new(1000, move || {
    *count_clone.borrow_mut() += 1;
    console::log!("Count:", *count_clone.borrow());
});

// Stop after 10 iterations
if *count.borrow() >= 10 {
    drop(interval);  // Stops the interval
}
```

### Pattern 3: WebSocket with Error Recovery

```rust
async fn resilient_websocket(url: &str) {
    loop {
        match WebSocket::open(url) {
            Ok(ws) => {
                if let Err(e) = handle_connection(ws).await {
                    console::error!("Connection error:", format!("{:?}", e));
                }
            }
            Err(e) => {
                console::error!("Failed to connect:", format!("{:?}", e));
            }
        }

        // Wait before reconnecting
        gloo_timers::future::sleep(Duration::from_secs(5)).await;
    }
}

async fn handle_connection(ws: WebSocket) -> Result<(), JsValue> {
    let (mut write, mut read) = ws.split();

    while let Some(msg) = read.next().await {
        // Process message
    }

    Ok(())
}
```

### Pattern 4: Async Timer (Future-based)

```rust
use gloo_timers::future::sleep;
use std::time::Duration;

async fn delayed_work() {
    console::log!("Starting...");

    sleep(Duration::from_millis(1000)).await;

    console::log!("After 1 second");

    sleep(Duration::from_millis(2000)).await;

    console::log!("After 3 seconds total");
}
```

## Best Practices from dialect-coach

1. **Use futures-based WebSocket** - Better than callback-based for async/await code
2. **Split WebSocket early** - Separate read/write into different tasks
3. **Channel for sending** - Decouple message sending from connection state
4. **Store timeouts** - Keep `Timeout`/`Interval` in state or they'll cancel
5. **Handle both text and binary** - Always match on message types
6. **Graceful reconnection** - Use exponential backoff, don't hammer server
7. **Cleanup on drop** - Timers auto-cancel when dropped

## Troubleshooting

### Issue: Timer executes immediately

**Cause:** Forgot to store timer or call `.forget()`

**Solution:** Store timer in struct or call `.forget()`:

```rust
let timeout = Timeout::new(1000, || {});
timeout.forget();  // Keeps timer alive
```

### Issue: WebSocket connection immediately closes

**Cause:** Not reading from stream, causing backpressure

**Solution:** Always consume the read stream:

```rust
while let Some(msg) = read.next().await {
    // Handle message
}
```

### Issue: Messages not sending

**Cause:** Write half of WebSocket dropped

**Solution:** Keep write task alive:

```rust
spawn_local(async move {
    while let Some(msg) = rx.next().await {
        let _ = write.send(msg).await;
    }
    // Keep task alive until channel closes
});
```

### Issue: "Use of moved value"

**Cause:** WebSocket consumed by split

**Solution:** Split returns separate sink/stream:

```rust
let ws = WebSocket::open(url)?;
let (write, read) = ws.split();  // ws is consumed
// Use write and read separately
```

## Further Resources

- **GitHub:** https://github.com/rustwasm/gloo
- **gloo-net docs:** https://docs.rs/gloo-net/latest/gloo_net/
- **gloo-timers docs:** https://docs.rs/gloo-timers/latest/gloo_timers/
- **Examples:** https://github.com/rustwasm/gloo/tree/master/examples

## Summary

Gloo provides ergonomic WASM utilities:

- **gloo-net** for futures-based WebSockets and HTTP
- **gloo-timers** for setTimeout/setInterval equivalents
- **Split pattern** for bidirectional WebSocket communication
- **futures-channel** for async message passing
- **Auto-cleanup** when timers are dropped
- **Async/await integration** throughout

The dialect-coach project demonstrates production patterns for resilient WebSocket connections with reconnection logic, channel-based message queuing, and proper lifetime management of timers and async tasks.
