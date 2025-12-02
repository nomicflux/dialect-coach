# Authentication & Session Management

## Overview

Dialect Coach uses JWT (JSON Web Token) based authentication with password hashing for secure user sessions. Sessions persist across page reloads using browser cookies with a 24-hour expiration.

## Architecture

### Backend (Rust/Axum)
- **Password Hashing**: bcrypt with cost factor 12
- **JWT Generation**: HS256 algorithm with 24-hour expiration
- **Session Validation**: Stateless JWT validation
- **Storage**: User records with password hashes in Sled database

### Frontend (Rust/Yew WASM)
- **Cookie Management**: JS-accessible cookies with `SameSite=Lax`
- **Session Restoration**: Automatic validation on page load
- **WebSocket Communication**: All auth operations over WebSocket

## Authentication Flow

### 1. User Registration
```
Frontend                WebSocket              Backend
   |                       |                      |
   |-- CreateUser -------->|------- Hash -------->|
   |    (credentials,      |     Password         |
   |     password)         |                      |
   |                       |<--- (User, JWT) -----|
   |<-- Store Cookie ------|                      |
```

**Messages**:
- `UserMessage::CreateUser { username, email, credentials, password }`
- `UserMessage::CreateUserResponse(Result<(User, String), String>)`

### 2. Sign In
```
Frontend                WebSocket              Backend
   |                       |                      |
   |-- SignIn ------------>|--- Verify Password ->|
   |    (username,         |                      |
   |     password)         |                      |
   |                       |<--- (User, JWT) -----|
   |<-- Store Cookie ------|                      |
```

**Messages**:
- `UserMessage::SignIn { username, password }`
- `UserMessage::SignInResponse(Result<(User, String), String>)`

### 3. Session Restoration (Page Reload)
```
Frontend                WebSocket              Backend
   |                       |                      |
   |-- Read Cookie ------->|                      |
   |-- ValidateSession --->|--- Validate JWT ---->|
   |    (token)            |    Extract user_id   |
   |                       |    Load User ------->|
   |                       |<------ User ---------|
   |<-- Set User State ----|                      |
```

**Messages**:
- `UserMessage::ValidateSession { token }`
- `UserMessage::ValidateSessionResponse(Result<User, String>)`

### 4. Logout
```
Frontend
   |
   |-- Clear Cookie
   |-- Clear User State
   |-- Clear Session
```

**Actions**:
- `AppStateAction::DestroySession`
- `UserStateAction::ClearUserState`
- `AppStateAction::ClearUser`

## Security Features

### Password Security
- **Hashing**: bcrypt algorithm (industry standard)
- **Cost Factor**: 12 (balances security vs performance)
- **Salt**: Automatically generated per password
- **Storage**: Only hashes stored, never plaintext

### JWT Security
- **Algorithm**: HS256 (HMAC with SHA-256)
- **Secret**: Configurable via `JWT_SECRET` environment variable
- **Expiration**: 24 hours (86400 seconds)
- **Claims**: Contains `sub` (user_id) and `exp` (expiration)

### Cookie Security
- **Name**: `dialect_coach_session`
- **Max-Age**: 86400 seconds (24 hours)
- **Path**: `/` (entire application)
- **SameSite**: `Lax` (CSRF protection)
- **HttpOnly**: No (allows JS access for WebSocket auth)
- **Secure**: Recommended for production (HTTPS only)

## Environment Configuration

### Required Variables

```bash
# JWT secret key for token signing/verification
# Must be at least 32 characters for security
JWT_SECRET=your-secret-key-here-minimum-32-characters-required
```

Add to `.env` file in project root.

## Implementation Files

### Backend
- `backend/src/crypto/jwt.rs` - JWT generation and validation
- `backend/src/crypto/password.rs` - Password hashing and verification
- `backend/src/auth_service.rs` - Authentication service interface
- `backend/src/websocket/user.rs` - WebSocket auth handlers
- `backend/src/persistence/mod.rs` - User storage with passwords

### Frontend
- `frontend/src/utils/cookies.rs` - Cookie management utilities
- `frontend/src/app/callbacks.rs` - Auth response callbacks
- `frontend/src/app/websocket_hooks.rs` - Session restoration logic
- `frontend/src/components/header.rs` - Login/logout UI
- `frontend/src/services/user_websocket.rs` - Auth WebSocket service

## Testing

### Integration Tests
Run authentication integration tests:
```bash
cargo test --package dialect-coach-backend --test auth_integration_tests
```

Tests cover:
- JWT generation and validation
- Password hashing and verification
- Invalid token handling
- Password verification success/failure

### Manual Testing Checklist
1. **Registration**: Create account → verify cookie set
2. **Page Reload**: Refresh page → verify user still logged in
3. **Expiration**: Wait 24+ hours → verify auto-logout
4. **Sign In**: Sign in → verify cookie set
5. **Logout**: Click sign out → verify cookie cleared
6. **Invalid Session**: Manipulate cookie → verify clear on validation failure

## Error Handling

### Common Errors

**"Invalid password"**
- User exists but password doesn't match
- Check password was entered correctly

**"Invalid token" / "Session validation failed"**
- JWT expired (24 hours passed)
- JWT secret changed
- Cookie was manually modified
- Solution: Clear cookie and sign in again

**"User not found"**
- Username doesn't exist in database
- Check username spelling

**"Unauthorized: invite code invalid"**
- Invite code doesn't exist or already used
- Get a valid invite code from admin

## Future Enhancements

Potential improvements (not currently implemented):
- [ ] Refresh tokens for extended sessions
- [ ] Rate limiting on auth endpoints
- [ ] Password reset flow
- [ ] Multi-factor authentication
- [ ] Session revocation/blacklist
- [ ] HttpOnly cookies (requires HTTP endpoint for initial auth)
