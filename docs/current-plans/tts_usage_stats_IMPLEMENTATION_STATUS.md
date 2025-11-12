<!-- Tracking doc for TTS usage stats sync work -->

# TTS Usage Stats Implementation Status

## Agreements Made
- 2025-11-10: "tts in @websocket.rs and the frontend does not update usage stats like running the agents does. Find out how it is different, and fix it to send back real-time stats in the same way."
- 2025-11-10: "Next, usage stats sidebar needs these things: - It should start collapsed - 'Usage Statistics' and the arrow should not be black text on a dark background"

## Explicitly Rejected
- None noted yet.

## Implementation Details
- Mirror websocket usage updates: reuse shared `user_state_connections` senders and emit `UserStateMessage::UsageStatsUpdate` after TTS usage persistence.
- Extend `backend/src/tts_handler.rs::TtsState` with shared connection map required for real-time updates.
- Ensure TTS rate-limit and persistence calls still succeed while broadcasting updates to frontend.
- Implemented `notify_usage_stats_update` in `backend/src/tts_handler.rs` sending serialized `UserStateMessage::UsageStatsUpdate` to the active connection when TTS usage stats persist successfully.
- Shared `user_state_connections` between `AppState` and `tts_handler::TtsState` via `backend/src/main.rs` wiring so TTS can reach the same frontend channel.
- Added regression coverage in `backend/src/tts_handler.rs` ensuring TTS usage tracking emits `UserStateMessage::UsageStatsUpdate` and persists refreshed stats.
- Usage footer now defaults to collapsed via `frontend/src/app/app_state.rs` (initial `usage_footer_collapsed` set to `true`).
- Toggle button styling in `frontend/styles/components/usage_footer.css` now relies on existing design tokens (`--surface`, `--surface-muted`, `--ink`, `--ink-soft`) so the collapsed bar renders with proper contrast using the shared palette.
- `frontend/index.html` now links `styles/components/usage_footer.css` so the footer styles load in the frontend bundle.

## Issues Encountered
- None yet.


