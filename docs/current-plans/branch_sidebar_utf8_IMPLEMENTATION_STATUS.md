# UTF-8 Branch Sidebar Implementation Status

## Agreements Made
- 2025-11-09: "Implement the plan as specified, it is attached for your reference. Do NOT edit the plan file itself."
- 2025-11-09: "Arabic in particular is a huge portion of this app, and eventually Japanese and Chinese will be as well, so everything should be UTF-8 compatible."

## Explicitly Rejected
- 2025-11-09: "DO NOT RUN `cargo doc --open`. EVER." (Reason: opens a browser window the environment cannot use.)

## Implementation Details
- Perform UTF-8 audit of `frontend/src` to identify byte-index slicing (`[..n]`, `String::truncate`, manual byte math`).
- Replace branch preview truncation in `frontend/src/components/branch_sidebar.rs` with character-aware helper returning ellipsis only when over limit.
- Add tests covering ASCII, Arabic, Japanese, and emoji inputs to verify helper behavior.
- Verify fixes via `cargo test -p dialect-coach-frontend` and manual branch flow with multi-byte content.

## Issues Encountered
- 2025-11-09: Branch sidebar panic reported due to `format!("{}...", &text[..max_len])` slicing mid-codepoint for Arabic text during branch creation.
