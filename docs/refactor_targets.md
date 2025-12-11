# Refactor Targets

## Modules > 200 lines

| File | Lines |
|------|-------|
| `backend/src/agent_service/response.rs` | 1522 |
| `frontend/src/components/utility_sidebar/learning.rs` | 1051 |
| `shared/src/models/agent.rs` | 956 |
| `shared/src/models/user_state.rs` | 918 |
| `shared/src/models/message.rs` | 851 |
| `shared/src/models/dialect.rs` | 810 |
| `backend/src/websocket/agents.rs` | 769 |
| `backend/src/agent_service/learning.rs` | 713 |
| `backend/src/agent_service/analysis.rs` | 603 |
| `backend/src/agent_service/retry.rs` | 593 |
| `frontend/src/components/plan/step_editor.rs` | 564 |
| `backend/src/auth_service.rs` | 525 |
| `backend/src/websocket/user_state.rs` | 518 |
| `backend/src/agent_service/enrichment.rs` | 475 |
| `shared/src/models/plan/import.rs` | 466 |
| `backend/src/tts_handler.rs` | 454 |
| `frontend/src/components/message_bubble.rs` | 436 |
| `backend/src/persistence/in_memory.rs` | 408 |
| `backend/src/agent_service/provider.rs` | 396 |
| `frontend/src/services/websocket.rs` | 380 |
| `backend/src/agent_service.rs` | 374 |
| `shared/src/models/partial_learning_item.rs` | 368 |
| `frontend/src/components/main_content.rs` | 367 |
| `backend/src/qdrant_service.rs` | 345 |
| `backend/src/test_utils.rs` | 332 |
| `frontend/src/app/user_state/callbacks.rs` | 314 |
| `frontend/src/services/speech.rs` | 312 |
| `backend/src/usage_tracker.rs` | 289 |
| `backend/src/bin/admin.rs` | 282 |
| `backend/src/translation_handler.rs` | 273 |
| `backend/src/main.rs` | 270 |
| `shared/src/models/gamification.rs` | 264 |
| `frontend/src/app/websocket_hooks.rs` | 255 |
| `frontend/src/components/plan/create.rs` | 252 |
| `backend/src/tts_service/azure_tts_provider.rs` | 238 |
| `backend/src/persistence/sled.rs` | 238 |
| `backend/src/rate_limiter/service.rs` | 238 |
| `backend/src/admin_invites.rs` | 229 |
| `backend/src/agent_service/util.rs` | 225 |
| `frontend/src/components/utility_sidebar/mod.rs` | 222 |
| `frontend/src/services/user_state_websocket.rs` | 219 |
| `frontend/src/keyboard_shortcuts.rs` | 218 |
| `backend/src/websocket/user.rs` | 210 |
| `frontend/src/components/utility_sidebar/settings.rs` | 209 |
| `shared/src/models/plan/mod.rs` | 205 |
| `shared/src/models/usage_stats.rs` | 201 |

## Enums/Structs > 7 elements

| Type | File | Line | Count |
|------|------|------|-------|
| `Dialect` enum | `shared/src/models/dialect.rs` | 10 | 30 variants |
| `UserState` struct | `shared/src/models/user_state.rs` | 79 | 17 fields |
| `UserMessageWithContext` struct | `shared/src/models/message.rs` | 196 | 11 fields |
| `ConversationContext` struct | `shared/src/models/message.rs` | 25 | 10 fields |

## Functions > 20 lines

| Function | File | Line | Lines |
|----------|------|------|-------|
| `HydratedLanguagePlan::try_into_plan()` | `shared/src/models/plan/import.rs` | 214 | 106 |
| `build_system_content()` | `backend/src/agent_service/response.rs` | 439 | 100+ |
| `ImportLanguagePlan::hydrate()` | `shared/src/models/plan/import.rs` | 129 | 80 |
| `run_agents_with_analysis()` | `backend/src/websocket/agents.rs` | 151 | 49 |
| `get_tts_voices()` | `shared/src/models/dialect.rs` | 432 | 44 |
| `Dialect::language()` | `shared/src/models/dialect.rs` | 83 | 40 |
| `Dialect::name()` | `shared/src/models/dialect.rs` | 165 | 36 |
| `Dialect::id()` | `shared/src/models/dialect.rs` | 125 | 35 |
| `Dialect::from_id()` | `shared/src/models/dialect.rs` | 321 | 34 |
| `Dialect::all()` | `shared/src/models/dialect.rs` | 284 | 32 |
| `calculate_streaks()` | `shared/src/models/gamification.rs` | 134 | 27 |
| `UserState::get_active_branch_messages()` | `shared/src/models/user_state.rs` | 281 | 26 |
| `UserState::migrate_branch_message_ids()` | `shared/src/models/user_state.rs` | 339 | 24 |
| `MistakeCategory::fmt()` | `shared/src/models/agent.rs` | 20 | 21 |
| `UserState::rebuild_branches_from_history()` | `shared/src/models/user_state.rs` | 365 | 20 |
| `UserState::build_action_context()` | `shared/src/models/user_state.rs` | 195 | 20 |
| `count_daily_messages()` | `shared/src/models/gamification.rs` | 101 | 20 |
