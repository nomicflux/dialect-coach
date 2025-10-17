# Tower-HTTP Tutorial: HTTP Middleware for Rust

## Overview

**tower-http** provides HTTP-specific middleware for Tower services. It includes middleware for CORS, tracing/logging, compression, timeouts, and serving static files. Works seamlessly with Axum and other Tower-based frameworks.

**Version used in dialect-coach:** `0.6` with cors, fs, and trace features

**Why we use it:** Provides production-ready middleware as composable layers, reducing boilerplate for common HTTP concerns.

## Dependencies

```toml
[dependencies]
tower-http = { version = "0.6", features = ["cors", "fs", "trace"] }
tower = "0.5"
axum = "0.8"
tracing = "0.1"
tracing-subscriber = "0.3"
```

## The Tower Ecosystem

Before diving into tower-http, understanding the Tower ecosystem is essential:

**Tower** is a library for building robust network clients and servers. It provides:
- **Service trait** - Core abstraction for async request/response
- **Layer trait** - Composable middleware
- **Utilities** - Timeouts, rate limiting, load balancing

**tower-http** builds on Tower specifically for HTTP:
- HTTP-specific middleware (CORS, compression, tracing)
- Works with any Tower-compatible framework
- Used by Axum, Tonic, Hyper

**Axum** is a web framework built on Tower:
- Uses Tower's Service trait for routing
- Applies tower-http middleware via `.layer()`
- Benefits from entire Tower ecosystem

**The relationship:**
```
Tower (core traits)
  ↓
tower-http (HTTP middleware)
  ↓
Axum (web framework)
```

**Why this matters:** Understanding Tower helps you understand why Axum middleware works the way it does - it's all based on the Service trait and Layer composition.

## Core Concepts

### 1. Layers

Middleware is applied as layers on routers:

```rust
use tower_http::cors::CorsLayer;

let app = Router::new()
    .route("/", get(handler))
    .layer(CorsLayer::permissive());
```

### 2. Layer Ordering

Layers wrap in reverse order (outer to inner):

```rust
Router::new()
    .layer(layer_3)  // Runs third (innermost)
    .layer(layer_2)  // Runs second
    .layer(layer_1)  // Runs first (outermost)
```

## How Middleware Wrapping Works

**The wrapping mechanism:** Each layer wraps the previous layer, creating an "onion" of middleware around your handler.

**Visual representation:**
```
Request Flow →

Client → Layer 1 (CORS) → Layer 2 (Trace) → Layer 3 (Compression) → Handler
                ↓              ↓                    ↓
         [CORS check]    [Log request]      [Check Accept-Encoding]
                ↓              ↓                    ↓
         [Add headers]   [Start span]       [Pass through]
                                                    ↓
                                              Your handler runs
                                                    ↓
         [Add headers] ← [Log response] ← [Compress body] ← Response
                ↓              ↓                    ↓
Client ← Layer 1         Layer 2             Layer 3 ← Response

Response Flow ←
```

**Why reverse order matters:**
1. `.layer(CorsLayer)` applied last wraps outermost → runs first on requests
2. `.layer(TraceLayer)` applied first wraps innermost → runs last on requests
3. On response path, the order reverses

**Example:**
```rust
let app = Router::new()
    .route("/api", get(handler))
    .layer(CompressionLayer::new())  // Inner - compresses response
    .layer(TraceLayer::new())         // Middle - logs
    .layer(CorsLayer::permissive());  // Outer - handles CORS first

// Request: CORS → Trace → Compression → handler
// Response: handler → Compression → Trace → CORS
```

**Rule of thumb:** Apply layers in the order you want them to process responses, or reverse order for requests.

### 3. Tower Services

All middleware implements the Tower `Service` trait, enabling composition.

## Understanding CORS

**What is CORS?** Cross-Origin Resource Sharing is a security mechanism that restricts how web pages from one origin can interact with resources from another origin.

**The problem:**
- Your frontend runs on `http://localhost:5173` (Vite dev server)
- Your API runs on `http://localhost:3000` (Axum server)
- These are different origins! Browser blocks the request by default

**How CORS works:**
1. Browser sends a "preflight" OPTIONS request asking "Can I access this?"
2. Server responds with allowed origins, methods, headers
3. If allowed, browser sends the actual request
4. Server includes CORS headers in response

**CORS headers:**
- `Access-Control-Allow-Origin` - Which origins can access
- `Access-Control-Allow-Methods` - Which HTTP methods allowed
- `Access-Control-Allow-Headers` - Which headers allowed
- `Access-Control-Max-Age` - How long to cache preflight response

**Example:**
```rust
use tower_http::cors::{CorsLayer, Any};

// Development: Allow everything
let cors = CorsLayer::new()
    .allow_origin(Any)  // Any origin can access
    .allow_methods(Any)  // Any method (GET, POST, etc.)
    .allow_headers(Any);  // Any headers

// Production: Specific origin
let cors = CorsLayer::new()
    .allow_origin("https://myapp.com".parse::<HeaderValue>().unwrap())
    .allow_methods([Method::GET, Method::POST])
    .allow_headers([AUTHORIZATION, CONTENT_TYPE]);
```

**When you need CORS:** Any time your frontend and backend are on different domains/ports during development or production.

## Key Middleware

### 1. CORS Layer

Handle Cross-Origin Resource Sharing:

```rust
use tower_http::cors::{CorsLayer, Any};

// Permissive (development)
let cors = CorsLayer::permissive();

// Configured (production)
let cors = CorsLayer::new()
    .allow_origin(Any)
    .allow_methods(Any)
    .allow_headers(Any);

// Specific origins
let cors = CorsLayer::new()
    .allow_origin("https://example.com".parse::<HeaderValue>().unwrap())
    .allow_methods([Method::GET, Method::POST])
    .allow_headers([AUTHORIZATION, CONTENT_TYPE]);
```

### 2. Trace Layer

Request/response logging:

```rust
use tower_http::trace::TraceLayer;

let app = Router::new()
    .layer(TraceLayer::new_for_http());
```

### 3. ServeDir

Serve static files:

```rust
use tower_http::services::ServeDir;

let app = Router::new()
    .nest_service("/static", ServeDir::new("assets"));
```

### 4. Compression

Compress responses:

```rust
use tower_http::compression::CompressionLayer;

let app = Router::new()
    .layer(CompressionLayer::new());
```

## Step-by-Step: Building a File Server with Logging

### Step 1: Basic Server with Tracing

```rust
use axum::{Router, routing::get};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into())
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let app = Router::new()
        .route("/", get(|| async { "Hello!" }))
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    tracing::info!("Listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
```

### Step 2: Add CORS

```rust
use tower_http::cors::{CorsLayer, Any};

let app = Router::new()
    .route("/api/data", get(get_data))
    .layer(TraceLayer::new_for_http())
    .layer(
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
    );
```

### Step 3: Serve Static Files

```rust
use tower_http::services::ServeDir;

let app = Router::new()
    .route("/api/health", get(health_check))
    .nest_service("/static", ServeDir::new("public"))
    .nest_service("/assets", ServeDir::new("assets"))
    .layer(TraceLayer::new_for_http());
```

### Step 4: Add Timeout

```rust
use tower_http::timeout::TimeoutLayer;
use std::time::Duration;

let app = Router::new()
    .route("/api/slow", get(slow_handler))
    .layer(TimeoutLayer::new(Duration::from_secs(30)))
    .layer(TraceLayer::new_for_http());
```

### Step 5: Complete Production Setup

```rust
use axum::{Router, routing::get};
use tower_http::{
    trace::TraceLayer,
    cors::{CorsLayer, Any},
    compression::CompressionLayer,
    services::ServeDir,
    timeout::TimeoutLayer,
};
use std::time::Duration;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        // API routes
        .route("/api/v1/users", get(list_users))
        .route("/api/v1/health", get(health_check))

        // Static files
        .nest_service("/static", ServeDir::new("public"))

        // Middleware layers (outermost to innermost)
        .layer(CompressionLayer::new())             // Compress responses
        .layer(TimeoutLayer::new(Duration::from_secs(30)))  // Timeout requests
        .layer(TraceLayer::new_for_http())          // Log requests
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any)
        );

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();

    tracing::info!("Server running on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn list_users() -> &'static str {
    "Users list"
}

async fn health_check() -> &'static str {
    "OK"
}
```

## How dialect-coach Uses tower-http

### 1. CORS Configuration (main.rs:98-102)

Allow frontend access:

```rust
use tower_http::cors::{CorsLayer, Any};

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
```

**Pattern:** Permissive CORS during development. In production, specify exact origins.

### 2. Trace Layer (main.rs:96)

Request logging:

```rust
use tower_http::trace::TraceLayer;

let app = Router::new()
    .route("/health", get(health_check))
    .layer(TraceLayer::new_for_http())
    .layer(cors_layer);
```

**Pattern:** Add tracing layer before CORS so all requests are logged.

### 3. Tracing Subscriber Setup (main.rs:41-53)

Configure logging output:

```rust
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

tracing_subscriber::registry()
    .with(
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| {
                eprintln!("[MAIN] Using default log level: debug");
                "debug".into()
            }),
    )
    .with(
        tracing_subscriber::fmt::layer()
            .with_writer(std::io::stderr)
    )
    .init();
```

**Pattern:** Environment-based log level (RUST_LOG env var), output to stderr.

### 4. Layer Composition (main.rs:93-103)

Stacking middleware:

```rust
Router::new()
    .route("/health", get(health_check))
    .route("/ws", get(websocket_handler))
    .layer(TraceLayer::new_for_http())  // Inner: logs requests
    .layer(CorsLayer::new()             // Outer: handles CORS
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
    )
    .with_state(state);
```

**Pattern:** Order matters! Trace layer inside CORS so CORS headers are in logs.

## Common Patterns

### Pattern 1: Development vs Production CORS

```rust
fn cors_layer() -> CorsLayer {
    if cfg!(debug_assertions) {
        // Development: permissive
        CorsLayer::permissive()
    } else {
        // Production: specific origins
        CorsLayer::new()
            .allow_origin("https://myapp.com".parse::<HeaderValue>().unwrap())
            .allow_methods([Method::GET, Method::POST])
    }
}
```

### Pattern 2: Custom Trace Spans

```rust
use tower_http::trace::TraceLayer;
use tracing::Level;

let trace = TraceLayer::new_for_http()
    .make_span_with(|request: &Request<_>| {
        tracing::span!(
            Level::INFO,
            "http_request",
            method = %request.method(),
            uri = %request.uri(),
        )
    });
```

### Pattern 3: Conditional Middleware

```rust
use tower::ServiceBuilder;

let mut layers = ServiceBuilder::new();

if enable_compression {
    layers = layers.layer(CompressionLayer::new());
}

if enable_tracing {
    layers = layers.layer(TraceLayer::new_for_http());
}

let app = Router::new().layer(layers);
```

## Best Practices from dialect-coach

1. **Trace before CORS** - Log all requests including CORS preflight
2. **Environment-based config** - Use env vars for CORS origins
3. **Stderr for logs** - Separate logs from application output
4. **Middleware order** - Outer layers run first (request) and last (response)
5. **Permissive in dev** - Strict CORS in production
6. **Structured logging** - Use tracing, not println!

## Troubleshooting

### Issue: CORS errors in browser

**Cause:** Wrong CORS configuration or ordering

**Solution:** Check CORS layer is outermost and allows your origin:

```rust
.layer(CorsLayer::new()
    .allow_origin("http://localhost:5173".parse::<HeaderValue>().unwrap())
    .allow_methods(Any)
    .allow_headers(Any)
)
```

### Issue: Logs not appearing

**Cause:** Tracing subscriber not initialized

**Solution:** Initialize before building app:

```rust
tracing_subscriber::fmt::init();
```

### Issue: Static files return 404

**Cause:** Wrong path or ServeDir misconfigured

**Solution:** Verify path exists and use correct nesting:

```rust
.nest_service("/static", ServeDir::new("public"))
// Serves files from ./public at http://localhost/static/*
```

## Further Resources

- **Tower HTTP Docs:** https://docs.rs/tower-http/latest/tower_http/
- **Tower Guide:** https://github.com/tower-rs/tower/blob/master/guides/building-a-middleware-from-scratch.md
- **Tracing:** https://docs.rs/tracing/latest/tracing/
- **Examples:** https://github.com/tower-rs/tower-http/tree/master/examples

## Summary

tower-http provides HTTP middleware:

- **CorsLayer** - Cross-origin resource sharing
- **TraceLayer** - Request/response logging
- **ServeDir** - Static file serving
- **CompressionLayer** - Response compression
- **TimeoutLayer** - Request timeouts
- **Composable** - Stack layers for complex behavior

The dialect-coach project demonstrates production middleware setup: tracing for observability, permissive CORS for development, proper layer ordering, and environment-based configuration.
