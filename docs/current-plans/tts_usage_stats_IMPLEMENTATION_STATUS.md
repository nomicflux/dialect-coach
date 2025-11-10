<!-- Tracking doc for TTS usage stats sync work -->

# TTS Usage Stats Implementation Status

## Agreements Made
- 2025-11-10: "tts in @websocket.rs and the frontend does not update usage stats like running the agents does. Find out how it is different, and fix it to send back real-time stats in the same way."

## Explicitly Rejected
- None noted yet.

## Implementation Details
- Mirror websocket usage updates: reuse shared `user_state_connections` senders and emit `UserStateMessage::UsageStatsUpdate` after TTS usage persistence.
- Extend `backend/src/tts_handler.rs::TtsState` with shared connection map required for real-time updates.
- Ensure TTS rate-limit and persistence calls still succeed while broadcasting updates to frontend.
- Implemented `notify_usage_stats_update` in `backend/src/tts_handler.rs` sending serialized `UserStateMessage::UsageStatsUpdate` to the active connection when TTS usage stats persist successfully.
- Shared `user_state_connections` between `AppState` and `tts_handler::TtsState` via `backend/src/main.rs` wiring so TTS can reach the same frontend channel.
- Added regression coverage in `backend/src/tts_handler.rs` ensuring TTS usage tracking emits `UserStateMessage::UsageStatsUpdate` and persists refreshed stats.

## Issues Encountered
- None yet.


