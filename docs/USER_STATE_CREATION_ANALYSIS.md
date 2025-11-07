# UserState Creation Analysis

This document analyzes ALL production code locations where `UserState::new()` is called to create new UserState instances.

## Production Code Locations

### Frontend

**`frontend/src/app/callbacks.rs`**

1. **Line 204**: `on_user_create_response` - Creates new UserState for newly created user
   ```rust
   // Create new UserState for the user
   let new_state = UserState::new(user.id);
   user_state.dispatch(UserStateAction::ReplaceUserState(new_state.clone()));
   ```
   Context: Called when a new user account is successfully created. Creates a fresh UserState with default values.

2. **Line 234**: `on_user_signin_response` - Creates new UserState for signing in user
   ```rust
   // Create UserState - will be populated from backend via WebSocket
   let new_state = UserState::new(user.id);
   user_state.dispatch(UserStateAction::ReplaceUserState(new_state.clone()));
   ```
   Context: Called when a user successfully signs in. Creates a fresh UserState that will be populated from backend via WebSocket load.

## Summary

There are **2 production code locations** where `UserState::new()` is called:

1. User account creation (`on_user_create_response`)
2. User sign-in (`on_user_signin_response`)

Both locations are in `frontend/src/app/callbacks.rs` and handle user authentication flows.

## Default Values in `UserState::new()`

When a new UserState is created, it initializes with:
- `user_id`: Provided parameter
- `learning_items`: Empty Vec
- `conversation_history`: Empty Vec
- `tts_enabled`: false
- `selected_language`: Language::Spanish
- `selected_dialect`: Dialect::SpanishCuban
- `formality`: Formality::Casual
- `teaching_mode`: TeachingMode::Immersive
- `active_branch_id`: ID of initial branch
- `branches`: Vec with one initial branch
- `learning_goals`: Empty Vec
- `usage_stats`: UsageStats::default()

