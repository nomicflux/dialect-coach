# Mobile Layout — Implementation Status

Plan: [MOBILE_LAYOUT_PLAN.md](MOBILE_LAYOUT_PLAN.md)

| Phase | Scope | Status | Checks run | Notes |
|---|---|---|---|---|
| 0 | Plan into the repo; user runs plan validation | Plan saved; user authorized Phase 1 on 2026-10-06 | Plan-validation result not reported | Desktop daily-target visibility clarified in the plan |
| 1 | Selection action bar | Complete, including subsequently authorized Undo repair | Initial checks: `cargo check`; `cargo test --workspace`: 646 passed, 0 failed, 4 ignored; both clippy gates clean; `bin/startup` rebuilt and served the app. Undo repair checks below. | Fork-specific Undo repaired after user authorization. Android long-press remains pending user confirmation. |
| 2 | One-row header + Log out in Settings | Not started | — | — |
| 3 | Today's focus always visible, count opens full list, new items pinned | Not started | — | — |

Baseline (2026-10-06): `cargo test --workspace` exit 0, 638 passed, 0 failed, 4 ignored. `cargo clippy -p dialect-coach-frontend --all-targets`: 0 warnings.

## Phase 1 browser checks (2026-10-06)

- A same-origin iframe at 412×860 shows the bar after programmatic selection without mouse events, including in a two-message branch. Transcript height changes from 548.1px to 505.7px; the 42.4px bar sits directly above the composer without covering messages.
- Selecting the first Arabic word shows the bar on screen. Selecting across two message IDs hides it. Composer selection also hides it. Desktop double-click at 1280×860 shows the bar.
- Grammar opens with the selected text. Grammar and Translate results save to Learning with their content visible there. All five drawer tabs render; Ctrl+Shift+[ opens and closes the drawer. Original-conversation delete/undo restores the exact message ID and text.
- Additional fork check failed: deleting the shared second message and undoing restores it to the original six-message branch, but leaves the new fork at one message. `restore_message_to_branch` in `frontend/src/app/app_state/user/helpers.rs` stops after the first matching branch. This helper, its reducer, and the delete/undo callbacks are unchanged by Phase 1. The original six-message conversation is restored and active. A scope decision was requested before changing branch restoration.
- Verification created one fork and saved two learning items (the greeting translation and a grammar explanation). The temporary iframe was removed; the viewport override was reset.
- Existing warnings observed: Trunk's deprecated `address` configuration; backend image platform mismatch; existing Redis orphan; keyboard listener calling `preventDefault` from a passive listener. The drawer shortcut still toggles correctly. No new Rust compiler or clippy warnings.

Screenshots are saved locally under `/private/tmp/dialect-coach-phase1-*.jpg`, including `phone-short-branch`, `phone-first-word`, `phone-cross-message`, `phone-translation`, `phone-saved-translation`, `phone-saved-grammar`, `phone-plan`, `phone-settings`, `phone-usage`, `phone-deleted`, `phone-undo` (failed fork restoration), `desktop-doubleclick`, and `desktop-undo` (successful original-conversation restoration).

## Undo repair authorized after Phase 1

User instruction (conversation context): "You should fix the undo. Undo restores the message to where it was. We don't check the branch after the fact, we restore it to the branch it was deleted from."

Capture the full message, history position, and each branch membership/position before deletion removes them. Carry that record through the existing UI undo stack and `MessageAction::UndoDelete`. Restore those recorded locations directly, preserving messages added afterward and leaving unrelated branches alone. Replace the parent-based search in place. Verify shared fork messages, root/interior/leaf positions, branch switching, subsequent additions, and successive undo operations through the standard tests and the live delete/undo UI. Phases 2 and 3 remain unstarted.

Complete. `cargo check` passed. `cargo test --workspace`: 651 passed, 0 failed, 4 existing ignored tests. `cargo clippy --all` and `cargo clippy -p dialect-coach-frontend --all-targets`: no warnings. Logs: `/private/tmp/dialect-coach-undo-{check,tests-final,clippy,clippy-frontend}.log`.

`bin/startup` rebuilt and served the repair. In a fresh two-message fork, deleted the shared agent message, switched to the original conversation, and clicked Undo. The original six-message conversation and the two-message fork both matched their recorded pre-delete IDs, text, and order exactly. The restored fork also matched after page reload. Phone layout checked at 412×860; screenshots: `/private/tmp/dialect-coach-undo-restored-fork-phone.png` and `/private/tmp/dialect-coach-undo-restored-fork.png`. The temporary viewport override was reset, and the original conversation is active with the drawer closed. Verification created one additional fork; all deleted messages were restored.
