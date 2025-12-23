# Docker + Postgres Migration Plan

## Overview
Migrate Dialect Coach from local Sled persistence to Docker-based deployment with Postgres, enabling multi-instance scaling and production deployment.

## User Decisions
- **Containers**: Separate (nginx for frontend, backend container)
- **Database**: Docker Compose with Postgres (full local stack)
- **URL Config**: Relative URLs + nginx proxy (no rebuild per environment)

---

## Phase 1: PostgresPersistence Implementation

**Goal**: Create Postgres backend for `UserPersistence` trait

**Subagent**: kiss-code-generator

### Files to Create
- `backend/src/persistence/postgres.rs` - PostgresPersistence implementation

### Files to Modify
- `backend/Cargo.toml` - Add sqlx with postgres feature
- `backend/src/persistence/mod.rs` - Export PostgresPersistence, add factory function

### Schema (4 tables with JSONB for complex data)
```sql
CREATE TABLE users (
    id UUID PRIMARY KEY,
    username TEXT UNIQUE NOT NULL,
    email TEXT,
    password_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE user_states (
    user_id UUID PRIMARY KEY REFERENCES users(id),
    version TEXT NOT NULL,
    data JSONB NOT NULL,
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE usage_stats (
    user_id UUID PRIMARY KEY REFERENCES users(id),
    data JSONB NOT NULL,
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE invite_codes (
    code TEXT PRIMARY KEY,
    created_date BIGINT NOT NULL,
    used_by UUID REFERENCES users(id),
    expiration BIGINT
);
```

### Implementation Details
- Use `sqlx` with compile-time query checking
- Keep versioned data wrapper for user_states (matches Sled pattern)
- Factory function selects Sled vs Postgres based on `DATABASE_URL` env var
- Connection pooling via sqlx's PgPool

### Code Style Checklist
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] Tests for PostgresPersistence

### Deliverables
- PostgresPersistence passing all UserPersistence trait methods
- Unit tests using sqlx test fixtures
- `cargo test` passes, `cargo clippy` clean

---

## Phase 2: Backend Docker Preparation

**Goal**: Make backend container-ready

**Status**: ✅ COMPLETE

**Issues Fixed**:
- Fixed Phase 1 compilation error at line 251 in postgres.rs: Added `.map(|id| id.to_string())` conversion
- Fixed uuid feature in Cargo.toml: Removed non-existent "sqlx" feature from workspace uuid dependency
- Added "uuid" feature to sqlx in backend/Cargo.toml for UUID type support

**Subagent**: kiss-code-generator

### Files to Modify
- `backend/src/main.rs` - Change bind address from `127.0.0.1` to `0.0.0.0` (ALREADY DONE)
- `backend/src/persistence/mod.rs` - Add `create_persistence()` factory and export PostgresPersistence

### Key Changes

**main.rs line 257** - Change:
```rust
// FROM
let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
// TO
let addr = std::env::var("BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0:3000".to_string());
let listener = tokio::net::TcpListener::bind(&addr).await?;
```

**persistence/mod.rs** - Add persistence factory:
```rust
pub async fn create_persistence() -> Result<Arc<dyn UserPersistence>> {
    if let Ok(database_url) = std::env::var("DATABASE_URL") {
        let pool = PgPool::connect(&database_url).await?;
        Ok(Arc::new(PostgresPersistence::new(pool)))
    } else {
        let db_path = get_db_path();
        Ok(Arc::new(SledPersistence::new(&db_path)?))
    }
}
```

### Code Style Checklist
- [x] Functions <20 lines
- [x] No dead code
- [x] Existing tests still pass

### Deliverables
- [x] Backend binds to configurable address (main.rs already had this)
- [x] Persistence auto-selects based on DATABASE_URL (factory function added to mod.rs)
- [x] `cargo test` passes - 100% (319 tests passed)
- [x] `cargo clippy` clean - zero warnings

### Files Actually Modified
1. `/Users/demouser/Code/dialect-coach/Cargo.toml` - Removed invalid "sqlx" feature from uuid dependency
2. `/Users/demouser/Code/dialect-coach/backend/Cargo.toml` - Added "uuid" feature to sqlx
3. `/Users/demouser/Code/dialect-coach/backend/src/persistence/postgres.rs` - Fixed line 251 UUID conversion
4. `/Users/demouser/Code/dialect-coach/backend/src/persistence/mod.rs` - Added `create_persistence()` factory and exports (added by Phase 1)

---

## Phase 3: Frontend Relative URLs

**Status**: ✅ COMPLETE (already implemented in commit ccf20f57)

**Goal**: Replace hardcoded localhost URLs with relative paths

**Subagent**: kiss-code-generator

### Files to Modify
- `frontend/src/app/app_state/app.rs` (lines 71-82)

### Key Changes
```rust
// FROM
WebSocketService::new("ws://localhost:3000/ws"),
UserStateWebSocketService::new("ws://localhost:3000/ws/user_state"),
UserWebSocketService::new("ws://localhost:3000/ws/user"),
CloudTtsService::new("http://localhost:3000"),
TranslationService::new("http://localhost:3000"),
EnrichmentService::new("http://localhost:3000"),

// TO (detect protocol from window.location)
WebSocketService::new(&get_ws_url("/ws")),
UserStateWebSocketService::new(&get_ws_url("/ws/user_state")),
UserWebSocketService::new(&get_ws_url("/ws/user")),
CloudTtsService::new(&get_base_url()),
TranslationService::new(&get_base_url()),
EnrichmentService::new(&get_base_url()),
```

### Helper Functions (new file or in app.rs)
```rust
fn get_base_url() -> String {
    web_sys::window()
        .and_then(|w| w.location().origin().ok())
        .unwrap_or_else(|| "http://localhost:3000".to_string())
}

fn get_ws_url(path: &str) -> String {
    let base = get_base_url();
    let ws_protocol = if base.starts_with("https") { "wss" } else { "ws" };
    let host = base.trim_start_matches("http://").trim_start_matches("https://");
    format!("{}://{}{}", ws_protocol, host, path)
}
```

### Code Style Checklist
- [x] Functions <20 lines (get_base_url: 4 lines, get_ws_url: 7 lines)
- [x] Helper functions are pure (no side effects)
- [x] Works in both dev (trunk serve) and production

### Deliverables
- [x] Frontend uses relative URLs (via get_base_url() and get_ws_url() helpers)
- [x] WebSocket protocol auto-detects (ws/wss based on https detection)
- [x] `trunk build` succeeds
- [x] `cargo test --all` passes - 100% (319 tests passed)
- [x] `cargo clippy` clean - zero warnings

### Implementation Details
- Helper functions added to frontend/src/app/app_state/app.rs (lines 201-214)
- AppState::default() now generates URLs dynamically (lines 67-70)
- All six services use relative URLs: WebSocket, UserState WS, User WS, CloudTTS, Translation, Enrichment

---

## Phase 4: Docker Configuration

**Status**: ✅ COMPLETE

**Goal**: Create Dockerfiles and docker-compose.yml

**Subagent**: modular-builder

### Files to Create

**`backend/Dockerfile`**

> **Note**: The backend uses pdfium for PDF parsing. Local dev uses `backend/lib/libpdfium.dylib` (macOS via `build.rs`), but Docker needs the Linux version (`libpdfium.so`). Both from [bblanchon/pdfium-binaries](https://github.com/bblanchon/pdfium-binaries) version chromium/7568 (145.0.7568.0).

```dockerfile
# Build stage
FROM rust:1.75-slim as builder
WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y pkg-config libssl-dev curl

# Download pdfium for Linux - same version as macOS (chromium/7568)
# Source: https://github.com/bblanchon/pdfium-binaries/releases/tag/chromium%2F7568
RUN curl -L -o /tmp/pdfium.tgz \
    "https://github.com/bblanchon/pdfium-binaries/releases/download/chromium%2F7568/pdfium-linux-x64.tgz" \
    && mkdir -p /app/backend/lib \
    && tar -xzf /tmp/pdfium.tgz -C /tmp \
    && cp /tmp/lib/libpdfium.so /app/backend/lib/

COPY . .
RUN cargo build --release -p dialect-coach-backend

# Runtime stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

# Copy binary and pdfium library
COPY --from=builder /app/target/release/dialect-coach-backend /usr/local/bin/
COPY --from=builder /app/backend/lib/libpdfium.so /usr/local/lib/

# Ensure library is found at runtime
ENV LD_LIBRARY_PATH=/usr/local/lib

EXPOSE 3000
CMD ["dialect-coach-backend"]
```

**`backend/build.rs` update needed** - Make platform-aware for Linux builds:
```rust
// Current: copies libpdfium.dylib (macOS only)
// Needs update to also handle libpdfium.so for Linux Docker builds
```

**`frontend/Dockerfile`**
```dockerfile
# Build stage
FROM rust:1.75-slim as builder
RUN rustup target add wasm32-unknown-unknown
RUN cargo install trunk
WORKDIR /app
COPY . .
RUN cd frontend && trunk build --release

# Serve stage
FROM nginx:alpine
COPY --from=builder /app/frontend/dist /usr/share/nginx/html
COPY frontend/nginx.conf /etc/nginx/conf.d/default.conf
EXPOSE 80
```

**`frontend/nginx.conf`**
```nginx
server {
    listen 80;
    root /usr/share/nginx/html;
    index index.html;

    # SPA routing - serve index.html for all non-file requests
    location / {
        try_files $uri $uri/ /index.html;
    }

    # WebSocket proxy
    location /ws {
        proxy_pass http://backend:3000;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_read_timeout 86400;
    }

    # API proxy
    location /api {
        proxy_pass http://backend:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }

    # Admin proxy
    location /admin {
        proxy_pass http://backend:3000;
        proxy_set_header Host $host;
    }

    # Health check proxy
    location /health {
        proxy_pass http://backend:3000;
    }
}
```

**`docker-compose.yml`** (project root)
```yaml
version: '3.8'

services:
  postgres:
    image: postgres:16-alpine
    environment:
      POSTGRES_USER: dialect_coach
      POSTGRES_PASSWORD: dialect_coach_dev
      POSTGRES_DB: dialect_coach
    volumes:
      - postgres_data:/var/lib/postgresql/data
      - ./backend/sql/init.sql:/docker-entrypoint-initdb.d/init.sql
    ports:
      - "5432:5432"
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U dialect_coach"]
      interval: 5s
      timeout: 5s
      retries: 5

  backend:
    build:
      context: .
      dockerfile: backend/Dockerfile
    environment:
      DATABASE_URL: postgres://dialect_coach:dialect_coach_dev@postgres:5432/dialect_coach
      BIND_ADDRESS: 0.0.0.0:3000
      JWT_SECRET: ${JWT_SECRET}
      ANTHROPIC_API_KEY: ${ANTHROPIC_API_KEY}
      QDRANT_URL: ${QDRANT_URL}
      QDRANT_API_KEY: ${QDRANT_API_KEY}
      # Add other env vars as needed
    depends_on:
      postgres:
        condition: service_healthy
    ports:
      - "3000:3000"

  frontend:
    build:
      context: .
      dockerfile: frontend/Dockerfile
    ports:
      - "8080:80"
    depends_on:
      - backend

volumes:
  postgres_data:
```

**`backend/sql/init.sql`** - Schema initialization
```sql
-- Users table
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY,
    username TEXT UNIQUE NOT NULL,
    email TEXT,
    password_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- User states (JSONB for flexibility)
CREATE TABLE IF NOT EXISTS user_states (
    user_id UUID PRIMARY KEY REFERENCES users(id),
    version TEXT NOT NULL,
    data JSONB NOT NULL,
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Usage statistics
CREATE TABLE IF NOT EXISTS usage_stats (
    user_id UUID PRIMARY KEY REFERENCES users(id),
    data JSONB NOT NULL,
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Invite codes
CREATE TABLE IF NOT EXISTS invite_codes (
    code TEXT PRIMARY KEY,
    created_date BIGINT NOT NULL,
    used_by UUID REFERENCES users(id),
    expiration BIGINT
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
CREATE INDEX IF NOT EXISTS idx_invite_codes_used_by ON invite_codes(used_by);
```

**`.dockerignore`** (project root)
```
target/
frontend/dist/
data/
*.db
.env
.git/
```

### Deliverables
- [x] All 6 Docker files created (backend/Dockerfile, frontend/Dockerfile, frontend/nginx.conf, docker-compose.yml, backend/sql/init.sql, .dockerignore)
- [x] backend/build.rs verified as platform-aware (already handles both macOS and Linux)
- [x] `cargo clippy` clean - zero warnings
- [ ] `docker-compose up --build` verification (ready but not tested yet)
- [ ] Frontend accessible at http://localhost:8080 (ready but not tested yet)
- [ ] Backend accessible at http://localhost:3000 (ready but not tested yet)
- [ ] Postgres data persists across restarts (ready but not tested yet)

### Files Created
1. `/Users/demouser/Code/dialect-coach/backend/Dockerfile` - Multi-stage build with pdfium for Linux
2. `/Users/demouser/Code/dialect-coach/frontend/Dockerfile` - Trunk build + nginx serve
3. `/Users/demouser/Code/dialect-coach/frontend/nginx.conf` - Proxy config for WebSockets and API
4. `/Users/demouser/Code/dialect-coach/docker-compose.yml` - Full stack orchestration
5. `/Users/demouser/Code/dialect-coach/backend/sql/init.sql` - Postgres schema initialization
6. `/Users/demouser/Code/dialect-coach/.dockerignore` - Docker build exclusions

### Files Verified (No Changes Needed)
1. `/Users/demouser/Code/dialect-coach/backend/build.rs` - Already platform-aware for macOS/Linux

---

## Phase 5: Integration Testing & Cleanup

**Goal**: Verify full stack works, clean up

**Subagent**: bug-hunter (if issues), kiss-code-generator (for fixes)

### Testing Checklist
- [ ] `docker-compose up --build` succeeds
- [ ] Can create user via invite code
- [ ] WebSocket chat works through nginx proxy
- [ ] User state persists across backend restarts
- [ ] TTS/translation endpoints work
- [ ] Frontend hot-reload still works with `trunk serve` (non-Docker dev)

### Files to Update
- `README.md` or `docs/` - Add Docker deployment instructions
- `.env.example` - Document required env vars for Docker

### Cleanup
- Remove any dead code
- `cargo clippy` clean across all crates
- `cargo test` passes

---

## Summary of All Changes

### New Files
| File | Purpose |
|------|---------|
| `backend/src/persistence/postgres.rs` | PostgresPersistence implementation |
| `backend/Dockerfile` | Backend container build |
| `frontend/Dockerfile` | Frontend container build |
| `frontend/nginx.conf` | nginx config with proxy rules |
| `docker-compose.yml` | Full stack orchestration |
| `backend/sql/init.sql` | Postgres schema |
| `.dockerignore` | Docker build exclusions |

### Modified Files
| File | Change |
|------|--------|
| `backend/Cargo.toml` | Add sqlx postgres dependency |
| `backend/src/persistence/mod.rs` | Export postgres, add factory function |
| `backend/src/main.rs` | Configurable bind address, use persistence factory |
| `backend/build.rs` | Platform-aware pdfium library copying (dylib vs so) |
| `frontend/src/app/app_state/app.rs` | Relative URLs instead of localhost |

### Preserved Behavior
- Sled continues to work for local non-Docker development
- Versioning system unchanged
- All existing tests pass
- UserPersistence trait interface unchanged
