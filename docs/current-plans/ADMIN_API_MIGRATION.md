# Admin API Migration - Implementation Plan

**Date Started**: 2025-11-08

## Problem Statement

The admin CLI tool (`backend/src/bin/admin.rs`) and backend server (`backend/src/main.rs`) both open separate Sled database connections to the same DB file. Sled does not support concurrent access from multiple processes, causing a locking conflict. Administrators must shut down the backend server to use admin tools.

## Solution Approach

Migrate admin functionality to backend API endpoints. Convert admin CLI to a thin HTTP client that calls these endpoints. This ensures a single Sled connection (in the backend) and enables remote administration.

## Explicitly Rejected

- Database proxy/coordinator process (adds unnecessary complexity)
- SQLite migration (out of scope)
- Allowing concurrent Sled access (not supported by Sled)

## API Endpoint Specifications

### Base Path
All admin invite endpoints: `/admin/api/invites`

### Authentication
**REQUIRED**: All admin endpoints must verify admin authentication token in `Authorization` header.
- Header format: `Authorization: Bearer <admin-token>`
- Admin token configured via environment variable `ADMIN_TOKEN` (backend startup)
- 401 Unauthorized if missing/invalid
- Security critical: prevents unauthorized invite code manipulation

### Endpoints

#### 1. Create Invite Code
- **Path**: `POST /admin/api/invites`
- **Auth**: Required
- **Request Body**:
  ```json
  {
    "expires_days": 7  // optional, defaults to no expiration if omitted
  }
  ```
- **Response** (201 Created):
  ```json
  {
    "code": "ABC123XYZ",
    "created_at": 1699564800,
    "expires_at": 1700169600  // null if no expiration
  }
  ```
- **Errors**:
  - 401: Invalid/missing auth
  - 500: Database error

#### 2. List Invite Codes
- **Path**: `GET /admin/api/invites`
- **Auth**: Required
- **Response** (200 OK):
  ```json
  {
    "invites": [
      {
        "code": "ABC123XYZ",
        "created_at": 1699564800,
        "expires_at": 1700169600,
        "used_at": null,
        "status": "active"  // "active" | "used" | "expired"
      }
    ]
  }
  ```
- **Errors**:
  - 401: Invalid/missing auth
  - 500: Database error

#### 3. Delete Invite Code
- **Path**: `DELETE /admin/api/invites/{code}`
- **Auth**: Required
- **Response** (204 No Content): Empty body on success
- **Errors**:
  - 401: Invalid/missing auth
  - 404: Code not found
  - 500: Database error

## Shared Types (in `shared/src/models/`)

Create `shared/src/models/admin.rs`:

```rust
// Request/Response types for admin API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInviteRequest {
    pub expires_days: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InviteResponse {
    pub code: String,
    pub created_at: i64,
    pub expires_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InviteListItem {
    pub code: String,
    pub created_at: i64,
    pub expires_at: Option<i64>,
    pub used_at: Option<i64>,
    pub status: String,  // "active" | "used" | "expired"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InviteListResponse {
    pub invites: Vec<InviteListItem>,
}
```

## Files to Modify/Create

### Phase 1: API Endpoint Implementation
- **Create**: `backend/src/routes/admin_invites.rs` (new module)
- **Modify**: `backend/src/routes/mod.rs` (add admin_invites module)
- **Modify**: `backend/src/main.rs` (mount new routes)
- **Create**: `shared/src/models/admin.rs` (request/response types)
- **Modify**: `shared/src/models/mod.rs` (expose admin module)

### Phase 2: CLI HTTP Client Implementation
- **Modify**: `backend/src/bin/admin.rs` (convert to HTTP client)
- **Modify**: `backend/Cargo.toml` (add reqwest dependency for admin binary)

### Phase 3: Integration Testing
- **Create**: `backend/tests/admin_api_tests.rs` (API endpoint tests)
- **Modify**: `backend/src/bin/admin.rs` (add error handling tests)

## Architecture Notes

**Backend Changes**:
- Move invite code generation logic to backend
- Move status calculation (active/used/expired) to backend
- Admin routes require authentication middleware
- Use existing `UserPersistence` trait methods

**CLI Changes**:
- Becomes thin HTTP client
- Reads admin token from env var `ADMIN_TOKEN`
- Reads backend URL from env var `BACKEND_URL` (default: http://localhost:8080)
- Formats responses for terminal display
- No direct database access

**Authentication Strategy**:
- Simple bearer token for v1 (sufficient for admin tools)
- Token configured at backend startup via `ADMIN_TOKEN` env var
- Future: Could migrate to proper admin user accounts

## Implementation Phases

(See detailed phase breakdown below)

---

# Phase 1: Backend API Endpoints

**Subagent**: modular-builder (multi-file, cross-crate work)

**Code Style Checklist** (CLAUDE.md):
- [ ] Functions <20 lines
- [ ] Helper functions for complex logic
- [ ] Pure functions where possible
- [ ] No defensive coding - known types in/out
- [ ] No TODOs - complete functionality
- [ ] No dead code or future-proofing
- [ ] Tests for all new functions

**Context Files**:
- `backend/src/routes/mod.rs`
- `backend/src/routes/admin.rs` (existing admin routes)
- `backend/src/main.rs`
- `backend/src/auth_service.rs` (for auth pattern reference)
- `shared/src/models/mod.rs`
- `shared/src/models/invite_code.rs`
- `backend/src/bin/admin.rs` (for code generation logic to extract)

**Deliverables**:
1. New module `backend/src/routes/admin_invites.rs` with:
   - `create_invite` handler (POST /admin/api/invites)
   - `list_invites` handler (GET /admin/api/invites)
   - `delete_invite` handler (DELETE /admin/api/invites/:code)
   - `verify_admin_token` middleware/helper
   - `generate_invite_code` helper (moved from CLI)
   - `calculate_invite_status` helper (moved from CLI)

2. New module `shared/src/models/admin.rs` with:
   - `CreateInviteRequest` struct
   - `InviteResponse` struct
   - `InviteListItem` struct
   - `InviteListResponse` struct

3. Updated `backend/src/main.rs`:
   - Mount admin invite routes at `/admin/api/invites`
   - Load `ADMIN_TOKEN` env var at startup (panic if not set)

4. Updated `backend/Cargo.toml`:
   - Add any needed dependencies

**Instructions for modular-builder**:
- Extract `generate_code()` from admin.rs and move to admin_invites.rs as `generate_invite_code()`
- Extract status calculation logic and create `calculate_invite_status(invite: &InviteCode) -> String`
- Create `verify_admin_token(headers: &HeaderMap, expected_token: &str) -> Result<(), StatusCode>` helper
- Use Axum extractors: `State`, `Json`, `Path`, `HeaderMap`
- Return proper HTTP status codes (201, 200, 204, 401, 404, 500)
- Backend must panic at startup if `ADMIN_TOKEN` env var not set (admin security critical)
- All handlers must call `verify_admin_token` first

**End of Phase**:
- Run: `cargo test` - must have 100% pass rate
- Run: `cargo check` - must compile cleanly
- Update: `docs/current-plans/ADMIN_API_MIGRATION_STATUS.md` with Phase 1 completion
- **STOP and wait for explicit approval**

---

# Phase 2: CLI HTTP Client Conversion

**Subagent**: kiss-code-generator (single file modification)

**Code Style Checklist** (CLAUDE.md):
- [ ] Functions <20 lines
- [ ] Helper functions for complex logic
- [ ] Pure functions where possible
- [ ] No defensive coding - known types in/out
- [ ] No TODOs - complete functionality
- [ ] No dead code or future-proofing
- [ ] Tests for all new functions

**Context Files**:
- `backend/src/bin/admin.rs`
- `shared/src/models/admin.rs` (the new request/response types)
- `backend/Cargo.toml`

**Deliverables**:
1. Modified `backend/src/bin/admin.rs`:
   - Remove direct Sled database code
   - Remove `generate_code()` function (now in backend)
   - Remove status calculation logic (now in backend)
   - Add HTTP client using `reqwest`
   - Read `ADMIN_TOKEN` from env var (panic if not set)
   - Read `BACKEND_URL` from env var (default: http://localhost:8080)
   - Implement commands:
     - `generate-invite [--expires-days N]` -> POST request
     - `list-invites` -> GET request
     - `delete-invite <code>` -> DELETE request
   - Format HTTP responses for terminal display
   - Handle HTTP errors gracefully (print error message, exit with code 1)

2. Updated `backend/Cargo.toml`:
   - Add `reqwest` with features `["blocking", "json"]` to admin binary dependencies

**Instructions for kiss-code-generator**:
- Create helper function `get_admin_client() -> (reqwest::blocking::Client, String, String)` that returns (client, backend_url, admin_token)
- Create helper function `format_invite_response(invite: &InviteListItem) -> String` for terminal display
- Use `reqwest::blocking::Client` for synchronous HTTP (CLI doesn't need async)
- Add `Authorization: Bearer {token}` header to all requests
- Map HTTP status codes to user-friendly error messages
- Preserve existing CLI argument parsing structure

**End of Phase**:
- Run: `cargo test` - must have 100% pass rate
- Run: `cargo check` - must compile cleanly
- Run: `cd backend && cargo build --bin admin` - must build successfully
- Update: `docs/current-plans/ADMIN_API_MIGRATION_STATUS.md` with Phase 2 completion
- **STOP and wait for explicit approval**

---

# Phase 3: Integration Testing

**Subagent**: kiss-code-generator (test file creation)

**Code Style Checklist** (CLAUDE.md):
- [ ] Functions <20 lines
- [ ] Helper functions for complex logic
- [ ] Test expected behavior, not edge cases
- [ ] No TODOs - complete tests

**Context Files**:
- `backend/src/routes/admin_invites.rs`
- `backend/tests/` (existing test structure)
- `shared/src/models/admin.rs`

**Deliverables**:
1. New file `backend/tests/admin_api_tests.rs`:
   - Test create invite (valid request, missing auth, invalid expires_days)
   - Test list invites (valid request, missing auth)
   - Test delete invite (valid request, missing auth, not found)
   - Test authentication (invalid token, missing header)
   - Use test helper to start backend with test DB
   - Use `reqwest::blocking` for HTTP requests in tests

**Instructions for kiss-code-generator**:
- Study existing test structure in `backend/tests/`
- Create test helper `setup_test_backend() -> (TestBackend, String)` that returns backend instance and admin token
- Write simple, focused tests for expected behavior
- Test both success and failure cases
- Ensure tests clean up (drop test DB)

**End of Phase**:
- Run: `cargo test` - must have 100% pass rate (all crates)
- Update: `docs/current-plans/ADMIN_API_MIGRATION_STATUS.md` with Phase 3 completion and final status
- **STOP and wait for explicit approval**

---

# Success Criteria

- [ ] Backend server runs with single Sled connection
- [ ] Admin CLI can be used while backend is running
- [ ] All three admin commands work via API
- [ ] Admin endpoints require authentication
- [ ] All tests pass (100% success rate)
- [ ] No dead code from old admin CLI implementation
- [ ] Documentation updated

# Testing Strategy

**Unit Tests**:
- `verify_admin_token` function (valid/invalid tokens)
- `generate_invite_code` function (uniqueness, format)
- `calculate_invite_status` function (active/used/expired logic)

**Integration Tests**:
- Full request/response cycle for each endpoint
- Authentication failure scenarios
- Error handling (404, 500)

**Manual Testing**:
- Start backend with `ADMIN_TOKEN=test123`
- Run CLI with `ADMIN_TOKEN=test123 BACKEND_URL=http://localhost:8080`
- Test all three commands end-to-end
- Verify concurrent backend + CLI usage works

# Issues Encountered

(To be filled during implementation)
