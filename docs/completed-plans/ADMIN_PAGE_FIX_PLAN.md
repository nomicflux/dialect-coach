# Admin Page Fix Plan

**Date**: 2025-10-31
**Problem**: Current implementation uses a separate WASM binary with broken asset paths. Need to simplify to a single HTML file served directly by the backend.

---

## What Was Built (WRONG)

1. Created separate WASM binary (`admin-page`) in frontend
2. Tried to serve from `/admin` but assets use absolute paths pointing to `/`
3. Created complex build setup that doesn't match existing app structure
4. Used nested router instead of simple routes in main.rs

**Result**: Blank page at `http://localhost:3000/admin` with all assets returning 404

---

## What Should Be Built (CORRECT)

Simple HTML file served directly from backend with embedded JavaScript.

---

## Files to DELETE

1. `backend/src/admin/routes.rs` - unnecessary router module
2. `frontend/src/components/admin_page.rs` - Yew component
3. `frontend/src/admin_main.rs` - separate binary entry point
4. `frontend/admin.html` - Trunk template

---

## Files to CREATE

### `backend/static/admin.html`

Single self-contained HTML file containing:
- Embedded CSS for dashboard layout
- Embedded JavaScript that:
  - Fetches `/admin/api/status` on page load
  - Displays Anthropic stats (note field)
  - Displays ElevenLabs stats (tier, characters used/limit/remaining, reset date)
  - Displays QDrant stats (points, vectors, segments, memory)
  - Displays dialect counts in a table
  - Shows "Service unavailable" for null values
  - Refresh button that re-fetches data
  - Number formatting (commas)
  - Byte formatting (KB/MB/GB)

---

## Files to MODIFY

### `backend/src/admin/mod.rs`

Remove line:
```rust
pub mod routes;
```

### `backend/src/main.rs`

**Remove this line** (around line 130):
```rust
app = app.nest("/admin", admin::routes::create_router());
```

**Add these imports** at the top with other imports:
```rust
use crate::admin::{anthropic_monitor, elevenlabs_monitor, qdrant_monitor};
```

**Add these two handler functions** before `async fn main()`:

```rust
async fn serve_admin_html() -> Result<Html<String>, StatusCode> {
    match std::fs::read_to_string("static/admin.html") {
        Ok(content) => Ok(Html(content)),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

async fn get_admin_status(State(state): State<AppState>) -> Json<admin::types::AdminStatusResponse> {
    use chrono::Utc;

    let timestamp = Utc::now().to_rfc3339();
    let anthropic = anthropic_monitor::get_anthropic_stats();

    let elevenlabs = match std::env::var("ELEVEN_LABS_API_KEY") {
        Ok(api_key) => elevenlabs_monitor::get_elevenlabs_usage(&api_key).await.ok(),
        Err(_) => None,
    };

    let qdrant = qdrant_monitor::get_qdrant_stats(&state.qdrant).await.ok();

    Json(admin::types::AdminStatusResponse {
        timestamp,
        anthropic,
        elevenlabs,
        qdrant,
    })
}
```

**Add these two routes** to the router (around line 110, with other routes):
```rust
.route("/admin", get(serve_admin_html))
.route("/admin/api/status", get(get_admin_status))
```

### `frontend/Cargo.toml`

**Remove these lines**:
```toml
[[bin]]
name = "admin-page"
path = "src/admin_main.rs"
```

### `frontend/src/components/mod.rs`

**Remove these lines**:
```rust
pub mod admin_page;
pub use admin_page::AdminPage;
```

---

## Backend Monitor Code (KEEP AS IS)

The following modules are correct and should NOT be changed:
- `backend/src/admin/types.rs`
- `backend/src/admin/anthropic_monitor.rs`
- `backend/src/admin/elevenlabs_monitor.rs`
- `backend/src/admin/qdrant_monitor.rs`

---

## Result After Fix

1. Navigate to `http://localhost:3000/admin`
2. See admin dashboard with data loaded automatically
3. Refresh button updates data
4. No build step required
5. No WASM compilation
6. Single HTML file

---

## Testing Checklist

- [ ] Page loads at `http://localhost:3000/admin`
- [ ] Anthropic section shows placeholder note
- [ ] ElevenLabs section shows usage stats OR "Service unavailable"
- [ ] QDrant section shows collection stats OR "Service unavailable"
- [ ] Dialect counts table displays all 16 dialects
- [ ] Refresh button updates timestamp
- [ ] No console errors
