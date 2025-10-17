# Axum Tutorial: Modern Web Server Framework

## Overview

**Axum** is an ergonomic and modular web framework built on Tokio, Tower, and Hyper. It emphasizes type safety, composability, and leverages Rust's powerful type system for compile-time correctness.

**Version used in dialect-coach:** `0.8` with ws and macros features

**Why we use it:** Provides excellent ergonomics for building APIs with strong type safety, easy state management, and seamless WebSocket support.

## Dependencies

```toml
[dependencies]
axum = { version = "0.8", features = ["ws", "macros"] }
tokio = { version = "1", features = ["full"] }
tower = "0.5"
tower-http = { version = "0.6", features = ["cors", "trace"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

## The Tower Ecosystem

Before diving into Axum, understanding the Tower ecosystem is essential:

**Tower** is a library for building robust network clients and servers. It provides:
- **Service trait** - Core abstraction for async request/response
- **Layer trait** - Composable middleware
- **Utilities** - Timeouts, rate limiting, load balancing

**tower-http** extends Tower specifically for HTTP:
- HTTP-specific middleware (CORS, compression, tracing)
- Works with any Tower-compatible framework
- Provides the middleware Axum uses

**Axum** is a web framework built on Tower:
- Uses Tower's Service trait for routing
- Applies tower-http middleware via `.layer()`
- Benefits from entire Tower ecosystem

**The relationship:**
```
Tower (core traits: Service, Layer)
  ↓
tower-http (HTTP middleware: CORS, Tracing, etc.)
  ↓
Axum (web framework: routing, extractors, handlers)
  ↓
Your Application (handlers + state + middleware)
```

**Why this matters:**
1. **Composability** - Middleware from tower-http works seamlessly
2. **Type safety** - Service trait ensures compile-time correctness
3. **Ecosystem** - Access to all Tower utilities (timeouts, rate limits, etc.)
4. **Understanding** - Knowing Tower helps you understand Axum's design

**Key insight:** Axum isn't just a framework - it's a thin, ergonomic layer on top of Tower's powerful abstractions. When you write an Axum handler, you're creating a Tower Service. When you add middleware, you're wrapping Services with Layers.

## Core Concepts

### 1. Handlers

Functions that handle HTTP requests:

```rust
use axum::{response::IntoResponse, http::StatusCode};

async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}
```

### 2. Extractors

Type-safe request data extraction:

```rust
use axum::{
    extract::{Path, Query, Json, State},
    response::Json as JsonResponse,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct CreateUser {
    name: String,
    email: String,
}

async fn create_user(
    Json(payload): Json<CreateUser>
) -> impl IntoResponse {
    // Use payload
    (StatusCode::CREATED, Json(payload))
}
```

### 3. Router

Composable routing:

```rust
use axum::{Router, routing::{get, post}};

let app = Router::new()
    .route("/", get(index))
    .route("/users", post(create_user))
    .route("/users/:id", get(get_user));
```

### 4. State Management

Shared application state:

```rust
use std::sync::Arc;

#[derive(Clone)]
struct AppState {
    db: Arc<Database>,
}

async fn handler(
    State(state): State<AppState>
) -> impl IntoResponse {
    // Use state.db
    "OK"
}

let app = Router::new()
    .route("/", get(handler))
    .with_state(AppState { db: Arc::new(database) });
```

## Step-by-Step: Building a REST API with WebSockets

### Step 1: Basic Server

```rust
use axum::{
    Router,
    routing::get,
    response::IntoResponse,
    http::StatusCode,
};
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health_check));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .unwrap();

    axum::serve(listener, app)
        .await
        .unwrap();
}

async fn root() -> &'static str {
    "Hello, World!"
}

async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}
```

### Step 2: JSON API with State

```rust
use axum::{
    extract::{Path, State, Json},
    response::{IntoResponse, Json as JsonResponse},
    http::StatusCode,
    Router,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

#[derive(Clone)]
struct AppState {
    users: Arc<Mutex<HashMap<u32, User>>>,
    next_id: Arc<Mutex<u32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: u32,
    name: String,
    email: String,
}

#[derive(Deserialize)]
struct CreateUserRequest {
    name: String,
    email: String,
}

async fn create_user(
    State(state): State<AppState>,
    Json(req): Json<CreateUserRequest>,
) -> impl IntoResponse {
    let mut next_id = state.next_id.lock().unwrap();
    let id = *next_id;
    *next_id += 1;

    let user = User {
        id,
        name: req.name,
        email: req.email,
    };

    state.users.lock().unwrap().insert(id, user.clone());

    (StatusCode::CREATED, Json(user))
}

async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<u32>,
) -> Result<Json<User>, StatusCode> {
    state.users
        .lock()
        .unwrap()
        .get(&id)
        .cloned()
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

async fn list_users(
    State(state): State<AppState>,
) -> Json<Vec<User>> {
    let users: Vec<User> = state.users
        .lock()
        .unwrap()
        .values()
        .cloned()
        .collect();

    Json(users)
}

#[tokio::main]
async fn main() {
    let state = AppState {
        users: Arc::new(Mutex::new(HashMap::new())),
        next_id: Arc::new(Mutex::new(1)),
    };

    let app = Router::new()
        .route("/users", post(create_user))
        .route("/users", get(list_users))
        .route("/users/:id", get(get_user))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

### Step 3: WebSocket Handler

```rust
use axum::{
    extract::{ws::{WebSocket, WebSocketUpgrade, Message}, State},
    response::Response,
};
use futures_util::{SinkExt, StreamExt};

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();

    // Send welcome message
    let _ = sender
        .send(Message::Text("Welcome to the chat!".to_string()))
        .await;

    // Handle incoming messages
    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                println!("Received: {}", text);

                // Echo back
                if sender
                    .send(Message::Text(format!("Echo: {}", text)))
                    .await
                    .is_err()
                {
                    break;
                }
            }
            Ok(Message::Close(_)) => {
                println!("Client disconnected");
                break;
            }
            _ => {}
        }
    }
}

// Add to router:
// .route("/ws", get(websocket_handler))
```

## Understanding WebSocket Split Pattern

**What is `.split()`?** When you call `.split()` on a WebSocket, you separate it into two independent halves:
- **Sender (SinkExt)** - For sending messages to the client
- **Receiver (StreamExt)** - For receiving messages from the client

**Why split is necessary:**

```
Without split (problem):
┌──────────────────────────────┐
│      WebSocket               │
│  (unified send/receive)      │
└──────────────────────────────┘
         ↓
    Can't read while writing!
    Can't write while reading!
    = DEADLOCK RISK
```

```
With split (solution):
┌──────────────────────────────┐
│      WebSocket               │
└──────────────────────────────┘
         .split()
          ↙    ↘
     Sender    Receiver
    (writes)   (reads)
       ↓          ↓
   Send task  Receive task
   (spawned)  (spawned)
       ↓          ↓
   Independent execution!
```

**The problem without split:**
```rust
// BAD: Can't do this simultaneously
while let Some(msg) = socket.next().await {
    // Reading blocks sending!
    socket.send(response).await?;  // What if we need to send WITHOUT receiving first?
}
```

**The solution with split:**
```rust
let (mut sender, mut receiver) = socket.split();

// Now we can:
// 1. Read independently in one task
tokio::spawn(async move {
    while let Some(msg) = receiver.next().await {
        process(msg);
    }
});

// 2. Send independently in another task
tokio::spawn(async move {
    while let Some(data) = channel.recv().await {
        sender.send(data).await;
    }
});
```

**Common pattern in dialect-coach:**
```rust
let (mut sender, mut receiver) = socket.split();
let (tx, mut rx) = mpsc::unbounded_channel::<String>();

// Spawn send task
let send_task = tokio::spawn(async move {
    while let Some(msg) = rx.recv().await {
        if sender.send(Message::Text(msg)).await.is_err() {
            break;
        }
    }
});

// Receive task (in main function)
while let Some(Ok(Message::Text(text))) = receiver.next().await {
    // Process message, send response via tx.send()
}
```

**Key benefits:**
1. **Non-blocking bidirectional** - Send and receive simultaneously
2. **Channel-based sending** - Other tasks can send messages via channel
3. **Clean separation** - Reading logic separate from sending logic
4. **Prevents deadlocks** - No waiting on yourself

**When NOT to use split:**
- Simple request-response pattern (read → process → send → repeat)
- No need for concurrent operations
- Simpler logic without background tasks

### Step 4: Error Handling

```rust
use axum::{
    response::{IntoResponse, Response},
    http::StatusCode,
    Json,
};
use serde_json::json;

struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let error_msg = format!("{}", self.0);

        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": error_msg
            }))
        ).into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

// Use in handlers
async fn fallible_handler() -> Result<Json<User>, AppError> {
    let user = fetch_user().await?;  // ? converts errors to AppError
    Ok(Json(user))
}
```

### Step 5: Complete Chat Server (REST + WebSocket)

A complete example combining REST API for message history with WebSocket for real-time chat:

```rust
use axum::{
    extract::{ws::{WebSocket, WebSocketUpgrade, Message}, State, Path},
    response::{Response, IntoResponse, Json},
    http::StatusCode,
    Router,
    routing::{get, post},
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tower_http::cors::{CorsLayer, Any};
use tower_http::trace::TraceLayer;

// --- Data Models ---

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatMessage {
    id: u64,
    username: String,
    content: String,
    timestamp: u64,
}

#[derive(Deserialize)]
struct SendMessageRequest {
    username: String,
    content: String,
}

// --- Application State ---

type Clients = Arc<Mutex<HashMap<String, mpsc::UnboundedSender<String>>>>;

#[derive(Clone)]
struct AppState {
    messages: Arc<Mutex<Vec<ChatMessage>>>,
    next_id: Arc<Mutex<u64>>,
    clients: Clients,
}

impl AppState {
    fn new() -> Self {
        Self {
            messages: Arc::new(Mutex::new(Vec::new())),
            next_id: Arc::new(Mutex::new(1)),
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn add_message(&self, username: String, content: String) -> ChatMessage {
        let mut next_id = self.next_id.lock().unwrap();
        let id = *next_id;
        *next_id += 1;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let message = ChatMessage {
            id,
            username,
            content,
            timestamp,
        };

        self.messages.lock().unwrap().push(message.clone());
        message
    }

    fn broadcast(&self, message: &ChatMessage) {
        let json = serde_json::to_string(message).unwrap();
        let clients = self.clients.lock().unwrap();

        for (username, tx) in clients.iter() {
            if tx.send(json.clone()).is_err() {
                println!("Failed to send to {}", username);
            }
        }
    }
}

// --- REST API Handlers ---

async fn get_messages(State(state): State<AppState>) -> Json<Vec<ChatMessage>> {
    let messages = state.messages.lock().unwrap().clone();
    Json(messages)
}

async fn get_recent_messages(
    State(state): State<AppState>,
    Path(limit): Path<usize>,
) -> Json<Vec<ChatMessage>> {
    let messages = state.messages.lock().unwrap();
    let recent: Vec<_> = messages
        .iter()
        .rev()
        .take(limit)
        .cloned()
        .collect();
    Json(recent)
}

async fn post_message(
    State(state): State<AppState>,
    Json(req): Json<SendMessageRequest>,
) -> (StatusCode, Json<ChatMessage>) {
    let message = state.add_message(req.username, req.content);
    state.broadcast(&message);
    (StatusCode::CREATED, Json(message))
}

async fn health() -> &'static str {
    "OK"
}

// --- WebSocket Handler ---

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
    ws.on_upgrade(move |socket| handle_websocket(socket, state))
}

async fn handle_websocket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    // Generate unique client ID
    let client_id = uuid::Uuid::new_v4().to_string();
    let username = format!("user_{}", &client_id[..8]);

    // Register client
    state.clients.lock().unwrap().insert(username.clone(), tx);
    println!("Client connected: {}", username);

    // Send welcome message
    let welcome = serde_json::json!({
        "type": "welcome",
        "username": username,
        "message": "Connected to chat server"
    });
    let _ = sender.send(Message::Text(serde_json::to_string(&welcome).unwrap())).await;

    // Spawn send task
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    // Clone username for cleanup
    let username_clone = username.clone();
    let state_clone = state.clone();

    // Receive task
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    #[derive(Deserialize)]
                    struct ClientMessage {
                        content: String,
                    }

                    if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                        let message = state_clone.add_message(
                            username_clone.clone(),
                            client_msg.content,
                        );
                        state_clone.broadcast(&message);
                    }
                }
                Message::Close(_) => {
                    println!("Client {} disconnected", username_clone);
                    break;
                }
                _ => {}
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = (&mut send_task) => {
            recv_task.abort();
        }
        _ = (&mut recv_task) => {
            send_task.abort();
        }
    }

    // Cleanup: remove client
    state.clients.lock().unwrap().remove(&username);
    println!("Client {} removed", username);
}

// --- Main Server ---

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let state = AppState::new();

    // Build router
    let app = Router::new()
        // REST API
        .route("/health", get(health))
        .route("/messages", get(get_messages).post(post_message))
        .route("/messages/recent/:limit", get(get_recent_messages))
        // WebSocket
        .route("/ws", get(websocket_handler))
        // Middleware
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any)
        )
        .with_state(state);

    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Chat server listening on {}", addr);
    println!("REST API: http://localhost:3000/messages");
    println!("WebSocket: ws://localhost:3000/ws");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .unwrap();

    axum::serve(listener, app)
        .await
        .unwrap();
}
```

**How to test this:**

1. **Start server:**
```bash
cargo run
```

2. **Test REST API:**
```bash
# Get messages
curl http://localhost:3000/messages

# Post message
curl -X POST http://localhost:3000/messages \
  -H "Content-Type: application/json" \
  -d '{"username": "alice", "content": "Hello!"}'

# Get recent 5 messages
curl http://localhost:3000/messages/recent/5
```

3. **Test WebSocket (using websocat):**
```bash
# Install websocat first: cargo install websocat

# Connect
websocat ws://localhost:3000/ws

# Send message (type and press Enter)
{"content": "Hello from WebSocket!"}
```

**What this example demonstrates:**
- ✅ REST API for message history (GET/POST)
- ✅ WebSocket for real-time chat
- ✅ Shared state between REST and WS
- ✅ Broadcasting to all connected clients
- ✅ Proper split pattern for WebSocket
- ✅ Graceful cleanup with tokio::select!
- ✅ CORS and tracing middleware
- ✅ Type-safe extractors and responses

## How dialect-coach Uses Axum

### 1. Main Server Setup (backend/src/main.rs:93-115)

Complete application with state and middleware:

```rust
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize services
    let qdrant = Arc::new(QdrantService::from_env().await?);
    let embeddings = Arc::new(EmbeddingService::new()?);
    let agent = Arc::new(AgentService::from_env(qdrant.clone(), embeddings.clone())?);

    let state = AppState {
        qdrant,
        agent,
        embeddings,
        session_histories: Arc::new(Mutex::new(HashMap::new())),
    };

    // Build router
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/ws", get(websocket_handler))
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(state);

    // Start server
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
```

**Pattern:** Centralized state, middleware composition, graceful error handling.

### 2. Application State (main.rs:23-29)

Type-safe shared state:

```rust
#[derive(Clone)]
pub struct AppState {
    pub qdrant: Arc<QdrantService>,
    pub agent: Arc<AgentService>,
    pub embeddings: Arc<EmbeddingService>,
    pub session_histories: Arc<Mutex<HashMap<Uuid, Vec<String>>>>,
}
```

**Why Arc:** Allows cheap cloning for sharing across async tasks. State is extracted in handlers via `State` extractor.

### 3. WebSocket Handler (websocket.rs:64-73)

WebSocket upgrade pattern:

```rust
pub async fn websocket_handler(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> Response {
    eprintln!("[WEBSOCKET] Upgrade request received!");
    tracing::info!("WebSocket upgrade request received");
    let response = ws.on_upgrade(move |socket| handle_socket(socket, state));
    eprintln!("[WEBSOCKET] Returning upgrade response");
    response
}
```

**Pattern:** Accept `WebSocketUpgrade`, call `.on_upgrade()` with handler, return response.

### 4. WebSocket Socket Handling (websocket.rs:76-260)

Full bidirectional communication:

```rust
async fn handle_socket(socket: WebSocket, state: AppState) {
    let connection_id = Uuid::new_v4();
    tracing::info!("WebSocket connection established: {}", connection_id);

    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    // Spawn send task
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Err(e) = sender.send(Message::Text(msg.into())).await {
                tracing::error!("Failed to send message: {}", e);
                break;
            }
        }
    });

    // Handle incoming messages
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                // Parse and handle message
                match serde_json::from_str::<Message>(&text) {
                    Ok(parsed_msg) => {
                        // Process message...
                    }
                    Err(e) => {
                        tracing::error!("Failed to parse: {}", e);
                    }
                }
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = (&mut send_task) => {
            recv_task.abort();
        }
        _ = (&mut recv_task) => {
            drop(tx);  // Close channel
            let _ = send_task.await;
        }
    }
}
```

**Pattern:** Split socket, spawn separate send/receive tasks, use `tokio::select!` for graceful shutdown.

### 5. Health Check Endpoint (main.rs:121-124)

Simple response handler:

```rust
async fn health_check() -> impl IntoResponse {
    eprintln!("[HEALTH] Handler called");
    (StatusCode::OK, "OK")
}
```

**Pattern:** Return tuple of `(StatusCode, Body)` which implements `IntoResponse`.

## Common Patterns

### Pattern 1: Nested Routers

```rust
let api_routes = Router::new()
    .route("/users", get(list_users).post(create_user))
    .route("/users/:id", get(get_user).put(update_user).delete(delete_user));

let app = Router::new()
    .nest("/api/v1", api_routes)
    .route("/health", get(health_check));
```

### Pattern 2: Middleware Layers

```rust
use tower_http::{cors::CorsLayer, trace::TraceLayer};

let app = Router::new()
    .route("/", get(handler))
    .layer(TraceLayer::new_for_http())
    .layer(CorsLayer::permissive());
```

### Pattern 3: Custom Extractor

```rust
struct AuthUser {
    id: u32,
    name: String,
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or((StatusCode::UNAUTHORIZED, "Missing auth"))?;

        // Validate token and extract user
        Ok(AuthUser { id: 1, name: "User".to_string() })
    }
}

async fn protected(user: AuthUser) -> String {
    format!("Hello, {}!", user.name)
}
```

## Best Practices from dialect-coach

1. **Use Arc for shared state** - Cheap cloning across async tasks
2. **Separate send/receive tasks** - Don't block WebSocket read on writes
3. **tokio::select! for shutdown** - Graceful task coordination
4. **Layered middleware** - Composable request/response processing
5. **Strong typing** - Use extractors for compile-time safety
6. **Centralized error handling** - Implement `IntoResponse` for error types
7. **Logging at boundaries** - Log at entry/exit points for debugging

## Troubleshooting

### Issue: "State not found" error

**Cause:** Forgot to call `.with_state()`

**Solution:** Add state to router:

```rust
let app = Router::new()
    .route("/", get(handler))
    .with_state(app_state);
```

### Issue: WebSocket immediately closes

**Cause:** Handler returning too early

**Solution:** Keep handler alive until connection closes:

```rust
while let Some(msg) = receiver.next().await {
    // Process message
}
```

### Issue: "Cannot share Arc<Mutex> across threads"

**Cause:** Using non-Send types in async context

**Solution:** Use `tokio::sync::Mutex` instead of `std::sync::Mutex` for async:

```rust
use tokio::sync::Mutex;
Arc<Mutex<T>>  // Works with async
```

## Further Resources

- **Official Docs:** https://docs.rs/axum/latest/axum/
- **GitHub:** https://github.com/tokio-rs/axum
- **Examples:** https://github.com/tokio-rs/axum/tree/main/examples
- **Book (community):** https://github.com/programatik29/axum-tutorial

## Summary

Axum provides modern web server capabilities:

- **Type-safe extractors** for request data
- **Composable routing** with nested routers
- **WebSocket support** with upgrade pattern
- **State management** via Arc and extractors
- **Middleware layers** from Tower ecosystem
- **Error handling** with `IntoResponse`

The dialect-coach project demonstrates production patterns including WebSocket communication, shared state management, middleware composition, and graceful task coordination with `tokio::select!`.
