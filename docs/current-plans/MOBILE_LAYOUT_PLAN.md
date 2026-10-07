# Mobile selection bar + phone layout — phased plan (PSP)

## Context
On Android, selecting text in a message never shows the Translate/Grammar buttons. The phone layout also spends a full row on Dashboard/New Conversation and hides Today's focus. An analysis (2026-10-06) established the causes in code and in the browser at 412px width. The user then chose a design for each point. This plan delivers those choices in three working phases.

Plan mode only allows this file, so **Phase 0 copies it to `docs/current-plans/MOBILE_LAYOUT_PLAN.md`**. That location is required by the project `.claude/CLAUDE.md` ("For architecture or multi-file changes: Create/update planning doc first") and PSP §4.

## User requirements and definitions — verbatim

**User message 1 (2026-10-06):**
> 1. I cannot selectt words on my phone to have the translate/grammar button show up - the normal Android selection for "Copy" / "Select All" comes up instaed.
> 2. View is cramped on the phone - "Dashboard" and "New Conversation" take up space when they should be on the header, "Sign Out" could be moved into dashboard (like a Logout setting), "Today's focus" and new items need to be visible even on the limited mobile space.

**User message 2:**
> Plan this out.
>
> 1. Selection buttons - fixed bar
> 2. Layout - approved
> 3. Focus: Open full list when tapped
> 4. Yes, plan

**User answers during planning** (questions are assistant-authored; answers are the user's):
- Q: "When you have a plan step or goal, what should the phone strip show?" → **"Keep switching"** (option text: "The phone strip shows the daily focus line plus whichever view is current (plan step, goal or review items), and tapping still switches.")
- Q: "Which size should the phone strip be?" (after the correction that an always-visible strip does not fit in the 50px saved) → **"One review row"** (option text: "Show one review row on phones instead of two, giving back about 30px of conversation. \"N to review\" still opens the full list.")
- Q: how should the functions that read the page or render components be tested (the suite runs outside a browser)? → **"Add basic frontend test scaffolding as needed, use manual browser checks for glue"**
- Q: which deletions are approved (old mouse-up selection code and props; floating Dashboard/New Conversation row and styles; both header Sign Out buttons; Today's focus open/close toggle styles; merging the header's two signed-in layouts)? → **"Approve all"**

**Desktop clarification (conversation context, 2026-10-06):**
> Yes, it should show on desktop, but make sure you understand what is happening first.
>
> I'm not asking whether exactly the same panel is showing, I'm asking whether the content shows anywhere

After the distinction was explained—underlying items are available in Learning, but their designation as today's selected practice/fix/review targets is not shown elsewhere—the user confirmed:
> Ok. That is the change that needs to be made, specifically.

Resolved requirement: desktop must visibly identify today's selected practice/fix/review targets. Availability of the underlying learning items in Dashboard → Learning does not satisfy this requirement.

**Assistant text the user's answers approved** (assistant-authored, quoted from the analysis):
- Decision 1, "fixed bar": "a small Translate | Grammar bar fixed just above the message box. It can never land off-screen, under the selection handles, or under Android's bar."
- Decision 2, "Layout - approved": "approve the one-row header, Log out under Dashboard → Settings → Account, and Today's focus always open on phones?" Proposal: "**Header, one row:** ☰ (Dashboard) and ➕ (New Conversation) as icon buttons on the left; status dot, Lvl and streak on the right. Sign Out moves to Dashboard → Settings → Account." / "**Today's focus, always open:** a compact strip showing the daily focus line plus the top review items, using part of the 50px saved. The count should match what's shown, or tapping it should open the full list." (The count is settled by Decision 3. Strip content is settled by "Keep switching", and size by "One review row".) / "**New items:** pin the pop-up just above the message box (or under the header) so it can't land under the buttons."
- Decision 3: "open the full review list when tapped".
- Fix approach for #1: "Use the existing selection-change listener … to show the buttons as well as hide them, and only when the selection is inside that message. Wait about 300ms after the selection stops changing … Remove the `onmouseup` trigger, since this one path then covers both mouse and touch."

**Standing rules that bind this work** (quoted):
- User `~/.claude/CLAUDE.md`: "Functions under 20 lines, under 10 where possible. This includes functions you modify." / "Write a function → write a test." / "Delete code only with the user's explicit permission, including code your change made obsolete." / "Use the project's standard test suite; no ad-hoc test files."
- Project `.claude/CLAUDE.md`: "Checking a UI feature requires visual confirmation. No exceptions." / "Modify the existing function in place … Update ALL callers in the same phase … Delete any code made obsolete by the changes" / "100% test pass rate required".

## Agent-authored design decisions (each tied to the sources above)
- **D-a, one implementation for all widths.** The bar replaces the floating buttons on desktop too, and Log out leaves the header on desktop too. Basis: "One implementation per behavior". None of the approved choices is width-restricted.
- **D-b, the bar and the new-items list are rendered inside the conversation area.** Both go inside `.chat`, which is `position: relative` and a flex column (`chat.css`).
  - The bar sits in flow after `.chat-scroll`, so it lands directly above the message box ("fixed just above the message box") and the transcript shrinks rather than being covered.
  - The new-items list is pinned `absolute` to the top-right of `.chat`, i.e. under the header (approved alternative). On phones that is below the focus strip, so it cannot cover the header buttons or "N to review".
  - The `.chat-canvas` grid is unchanged, so the auto-placed `.chat` and `.message-undo-notification` keep their cells.
- **D-c, Log out also closes the drawer.** `sign_out` leaves `ui_state.drawer_open` set. Logging out from inside the drawer would otherwise reopen the drawer at the next sign-in. The renewal-refused caller of `sign_out` (`websocket_hooks.rs:182`) is unchanged, as today.
- **D-d, the "full list" is Dashboard → Learning tab.** `Learning` (`utility_sidebar/learning.rs`) lists every learning item for the active dialect.
- **D-e, phone strip = title row ("TODAY'S FOCUS" + "N to review" button) + daily focus line + the current view, switching on tap ("Keep switching") with one review row ("One review row").** Quests keep their two-item cap. On desktop, the panel must visibly identify today's selected practice/fix/review targets; the title row stays hidden and the phone row caps do not apply.
- **D-f, deletions.** The user answered "Approve all" for the listed deletions, including merging the header's two signed-in layouts. Each phase lists its deletions; nothing outside those lists is deleted.
- **D-g, testing** ("Add basic frontend test scaffolding as needed, use manual browser checks for glue").
  - Frontend tests run natively (`cargo test`, rlib): web-sys DOM calls need wasm, and yew has only `csr`.
  - Logic is extracted into pure functions with unit tests in each module's `#[cfg(test)]`.
  - The scaffolding is a new `#[cfg(test)] pub mod test_fixtures` (`frontend/src/test_fixtures.rs`, declared in `lib.rs`). It holds builders for exactly the values those tests need: an agent `Message` (via `Message::user_message`/`MessageMetadata::new` or the agent constructor in `shared/src/models/message.rs`), a `LearningItem` (`LearningItem::new`), a `Quest`, and a `UserState` with an active `LanguagePlan` step. Each builder is added in the phase whose test first uses it.
  - Glue (DOM reads, Yew render helpers, hooks, callback wiring) is checked manually in the browser at each phase gate, with screenshots.
- **Out of scope:** message-box height.

## Current behavior (traced, with evidence)
- **Selection.**
  - `MessageBubble` (`components/message_bubble.rs`) sets `selection_state` only on `onmouseup` of `.message-content`. Its `selectionchange` effect only clears it.
  - It renders `TranslateSelectionButton` (alias of `SelectionActionButtons`, `translate_selection_button.rs`; re-exported at `components/mod.rs:46`; used only by `message_bubble.rs`) at an inline `position: fixed; left: rect.right; top: rect.bottom`.
  - Props flow: `MainContent.on_selection_translate_click` → `ChatWindow.on_selection_translate` / `translate_loading` → `MessageBubble.on_selection_translate` / `is_translate_loading`.
  - Android fires only `touchstart selectstart selectionchange` on long-press, and only `selectionchange` on handle drags ([gist](https://gist.github.com/robertknight/ad9f1bca8280b3fa325817aa3ed9935c)).
  - Probes at 412px: a no-mouse programmatic selection rendered 0 buttons. A mouse double-click rendered them. For the first Arabic word the buttons spanned x=353–531.
  - Desktop probe: mousedown and click on a `<button>` left the selection intact (`sc false`, `md false`, `click false`).
- **Header.**
  - `Header` (`components/header.rs`) renders brand (grid `center`) and `user-section` (`right`), with Sign Out in both signed-in branches. The comment at `:69` is stale.
  - `app.rs` renders `.dashboard-buttons-container` with `DashboardButton` and `NewConversationButton` inside `.app-main`. `dashboard_button.css` fixes it on desktop and makes it a ≤900px row.
  - At 412px: header 62px, pill row 50px, and the header's left 169px are empty.
- **Sign out.** `on_signout_click(app_state, session)` → `sign_out` (cookie, `DestroySession`, `Logout`, `ClearUser`). There is a second `sign_out` caller at `websocket_hooks.rs:182`. `UIStateAction::SetDrawerOpen(bool)` exists.
- **Settings and drawer.**
  - `Settings` (`utility_sidebar/settings.rs`, ~195-line fn, manual `PartialEq`) takes `user`, `is_admin`, `dispatch`.
  - `StudyDrawerContent` (derived `PartialEq`) passes them.
  - `MainContent` opens a specific tab with `drawer_active_tab.set(..)` + `SetDrawerOpen(true)` (OpenUsage shortcut).
- **Today's focus.**
  - `DynamicIsland` starts in `ViewState::Plan`; the effective mode is the first available of Plan/Goal/Items. A click cycles modes.
  - Content sits in `<details class="island-disclosure">` with `<summary>` "Today's focus" + `"{review_count} to review"`, where `review_count = available_items.len()`.
  - At ≤900px it is collapsed by default, with a cap of 2 review rows and 2 quests via `MOBILE_REVIEW_LIMIT = 2` (used by both `render_items` and `render_quests`). CSS `.island-item:nth-of-type(n + 4), .island-quest-item:nth-of-type(n + 4) { display:none }` is **unscoped**.
  - Measured at 412px: 60px collapsed, 192px open.
  - Desktop diagnosis (2026-10-06): the same `<details>` starts closed, while desktop CSS hides its `<summary>`. The child panel's `display: flex` does not reveal it. Opening the disclosure at phone width and returning to desktop reveals the content; closing it at phone width and returning to desktop hides it again.
  - Content access: Dashboard → Learning contains the underlying review and quest items (verified live). Dashboard → Plan renders learning goals and the active plan step (traced in source; the inspected account has neither). Today's selected practice/fix/review designation is rendered only in the focus panel; the separate `QuestList` component has no render caller.
- **New items.**
  - `LearningItemsFlash` is rendered last in `MainContent` (inside `.app-main`). CSS is `position: fixed; right` with no `top`/`bottom`. A probe at 412px landed at y=62 over New Conversation.
  - Its rows have class `island-item`, so at ≤900px the unscoped `nth-of-type(n+4)` rule hides the 4th and later new items.
- **Grid.** `.chat-canvas` areas: desktop `"focus chat" "focus composer"`, ≤900px `"focus" "chat" "composer"`. `.chat` and `.message-undo-notification` have no area and are auto-placed (`grid-area: chat` is on `.chat-scroll`, which is not a grid child). This plan does not change the grid.
- **Serving.** localhost:8080/3000 are Colima forwards of `docker compose`. `bin/startup` = `docker buildx bake && docker compose up`. The frontend uses same-origin URLs (`get_base_url`).
- **Baseline (2026-10-06).** `cargo test --workspace` exit 0: 638 passed, 0 failed, 4 ignored. `cargo test -p dialect-coach-frontend`: 84 passed. `cargo clippy -p dialect-coach-frontend --all-targets`: 0 warnings.

## Model and behavior

**Selection bar state** (single instance, owned by the bar):
`Option<BarSelection { message_id: Uuid, text: String }>`
- **Set or clear:** 300ms after the last `selectionchange`, read the document selection. It is `Some` only if it is non-collapsed, the text is non-blank, and the anchor and focus nodes resolve to the same `[data-message-id]` (on `.message-content`).
- **Clear on action:** clicking an action also clears it.
- **Context** for the API = `message_context(messages, id)` = that message's `get_content()`, as today.
- **Disabled** while `translate_loading` contains `message_id`.

| Case | Operation | Result | Source |
|---|---|---|---|
| Android long-press | long-press a word | ~300ms later the bar shows above the message box; Android's Copy bar also shows | #1, Decision 1 |
| Handle drag | extend the selection | bar text updates after the drag stops | #1 |
| Tap Translate / Grammar | tap | modal opens with selected text + message content; bar hides | preserves current modal |
| Desktop double-click | double-click a word | bar above the message box | D-a |
| Across two messages | drag across bubbles | no bar | "only when the selection is inside that message" |
| Outside messages | select in composer or drawer | no bar | same |
| Deselect | tap empty space | bar hides after 300ms | Decision 1 |

**Header (all widths):**
- Left: `.header-actions` with ☰ Dashboard and ➕ New Conversation.
- Center: brand (hidden ≤640 as now).
- Right: status, Lvl, streak, username (hidden ≤900 as now).
- No Sign Out. At ≤900px the labels are hidden, and `aria-label` keeps the names.

**Log out:** Settings → last section "Account" → "Log out" (`.signout-button` style). Click → `SetDrawerOpen(false)` then `sign_out` → the sign-in screen, as after today's Sign Out.

**Today's focus (≤900px):**
- Always rendered: title row "TODAY'S FOCUS" plus, when N>0, a `<button>` "N to review", then the panel.
- The panel shows the daily focus line plus the current view (plan/goal/items). Tapping the panel still switches views.
- Items view shows 1 review row plus "+N more" (`MOBILE_REVIEW_LIMIT = 1`). Quests stay capped at 2 (`MOBILE_QUEST_LIMIT = 2`).
- "N to review" → drawer opens on Learning.
- Desktop: the title row stays hidden, and the panel is visible without opening a disclosure at another width. Its Daily Focus section identifies today's selected practice/fix/review targets whenever there are incomplete quests. All rendered rows show; the panel still cycles plan/goal/items.

**New items:** `position: absolute; top: var(--s-2); right: var(--s-2); max-height: calc(100% - var(--s-4))` inside `.chat`. All items show at every width once the island caps are scoped to `.dynamic-island`. Dismiss is unchanged.

**Expected phone budget at 412×860 (verify):** today, with focus open, the conversation is 408px. Removing the 50px pill row and about 30px for the second review row gives ≈ 490–500px. Today, with focus hidden, it is 548px.

## Phases

### Phase 0: Plan into the repo and validation
1. Copy this file to `docs/current-plans/MOBILE_LAYOUT_PLAN.md` and create `docs/current-plans/MOBILE_LAYOUT_IMPLEMENTATION_STATUS.md` (phase table).
2. Then (PSP §6) **the user** runs plan validation (canonically `/verify-plan`) on the saved plan, reviews the report, and authorizes Phase 1. No phase work starts before that.

### Phase 1: Selection action bar (#1, Decision 1)
**Outcome:** selecting text inside a message, by touch or mouse, shows the Translate | Grammar bar directly above the message box at every width. The actions behave as today.

**Changes (author: me, inline):**
- `components/translate_selection_button.rs`: `SelectionActionButtons` is rewritten in place as the bar.
  - Props: `messages: Arc<Vec<Message>>`, `translate_loading: HashSet<Uuid>`, `on_action: Callback<(Uuid, SelectionAction, String, String)>`.
  - **Delete** the old props `selected_text`, `message_context`, `position`, `is_loading` and the inline fixed style.
  - Functions, each <20 lines:
    - `use_bar_selection()`: hook; one `gloo::events::EventListener` on `selectionchange`, restarting a 300ms `gloo::timers::callback::Timeout` held in `use_mut_ref`.
    - `read_selection() -> Option<BarSelection>`: DOM glue.
    - `message_id_at(node) -> Option<String>`: `parent_element`/`closest("[data-message-id]")`.
    - `selection_target(text, anchor_id, focus_id) -> Option<BarSelection>`: pure, **tested** (blank, mismatched, bad uuid, valid).
    - `message_context(messages, id) -> Option<String>`: pure, **tested**.
    - `render_action_button(..)` and the component fn.
  - The alias `pub use … as TranslateSelectionButton` stays; `MainContent` becomes its user.
- `components/message_bubble.rs`:
  - Add `data-message-id` on `.message-content`.
  - **Delete** `TextSelection`, `get_text_selection`, `selection_state`, `on_mouseup`, the `selectionchange` effect, `on_selection_action`, the button render, the props `is_translate_loading`/`on_selection_translate`, and the imports that become unused (`TranslateSelectionButton`, `SelectionAction`, `web_sys::{MouseEvent, window}`, `wasm_bindgen::prelude`).
  - Split `message_bubble` (<20 lines): extract `render_bubble(props)` and `render_bot_header(props)`.
- `components/chat_window.rs`:
  - **Delete** props `translate_loading`, `on_selection_translate`, the per-message `is_translate_loading`, and the `SelectionAction` import.
  - Add `#[prop_or_default] children: Html`, rendered after `.chat-scroll` inside `.chat`.
  - Split `chat_window`: `use_auto_scroll(ref, count)` and `render_chat_body(props, active)`.
  - Split `render_message_with_rope`: extract `render_bubble(..)`.
- `components/main_content.rs`: stop passing the two removed `ChatWindow` props. Pass `<TranslateSelectionButton messages translate_loading on_action={on_selection_translate_click}/>` as a `ChatWindow` child, and import `TranslateSelectionButton`. Split `main_content` (~440 lines) into functions under 20 lines, behavior-preserving:
  - `use_main_memos(us)` → filtered items/goals; `use_history_memos(us)` → branches/messages.
  - Pure `current_step_title(us) -> Option<String>` and `dialect_quests(us) -> Vec<Quest>`, **tested** using `UserState::new(Uuid::new_v4())`, as the existing tests in `app_state/user/tests.rs` do.
  - `use_selection_modal(..)`: modal state, close, save, and action. The save logic goes into pure `learning_action_for(result, element, meaning, context) -> LearningAction`, **tested**. The action logic goes into `async fn run_selection_action(..) -> Result<SelectionResult, String>` plus `apply_selection_result(..)`.
  - `use_goal_callbacks`, `use_branch_callbacks`, `use_learning_item_callbacks`, `use_drawer_callbacks`.
  - `use_keyboard_shortcuts(ctx)` with `handle_shortcut(action, &ctx)`: one line per arm calling small fns. The existing `FocusGoalInput` arm is kept unchanged.
  - `render_chat_canvas(..)`, `render_island(..)`, `render_drawer(..)`; the existing `render_modal` stays.
- `styles/components/chat.css`:
  - `.selection-action-bar { display: flex; justify-content: center; gap: var(--s-2); padding: var(--s-2) var(--chat-gutter) 0; flex: 0 0 auto; }`.
  - Add `flex: 1 1 auto` to `.chat-scroll`. Today it has no flex size, so in a short conversation an in-flow bar would sit right under the last message instead of above the message box. With `flex: 1` the scroller fills `.chat` and the bar stays at the bottom.
- `frontend/src/test_fixtures.rs` (new, `#[cfg(test)]`) plus its `lib.rs` declaration, with the builders this phase's tests use: agent `Message`, `UserState` with a plan step, `Quest`.

**Boundary:** the old mouseup path is gone; the bar is live through `MainContent` → `ChatWindow` children. No unused props or imports remain.

**Verification:**
- `cargo test --workspace` (all green: 638 + new), `cargo clippy -p dialect-coach-frontend --all-targets` (0 warnings).
- `bin/startup` in the background, then Chrome with a 412px same-origin iframe plus desktop, screenshots each:
  - (a) Programmatic no-mouse selection → bar fully on-screen above the composer; the transcript shrinks and is not covered. Repeat in a branch with only 1–2 messages: the bar still sits directly above the composer.
  - (b) First Arabic word → bar on-screen.
  - (c) Selection across two messages → no bar.
  - (d) Grammar → modal opens with the selected text.
  - (e) Desktop double-click → bar.
  - (f) Shortcuts (Ctrl+Shift+[ toggles the drawer), the Translate/Grammar save to learning items, drawer tabs, and delete/undo message all still work (behavior-preserving split).
- Android long-press is confirmed by the user on a phone and reported as pending until then.

### Phase 2: One-row header + Log out in Settings (#2: "Dashboard"/"New Conversation" on header; "Sign Out" into dashboard)
**Changes:**
- `components/header.rs`: add `.header-actions` with `<DashboardButton/>` and `<NewConversationButton/>`, importing both. Remove both Sign Out buttons and the `on_signout_click` import. Fix the stale `:69` comment. Split `header` (<20 lines):
  - `use_signin_callback`
  - `render_status`
  - `render_signed_in(username, Option<stats>)`: the two branches now differ only by stats, so they merge.
  - `render_sign_in_form` with `input_setter(state) -> Callback<InputEvent>`
  - `render_error_banner`
- `app.rs`: **delete** the `.dashboard-buttons-container` block and the two imports. Split `app`: `use_session()`, `use_error_auto_dismiss(app_state)`, `render_main_view(..)`.
- `components/dashboard_button.rs`, `new_conversation_button.rs`: label in `<span class="dashboard-label">`, `aria-label` on the button, and the fns kept under 20 lines (extract the click callback).
- `styles/components/dashboard_button.css`:
  - **Delete** the fixed `.dashboard-float-container` rule, the `.dashboard-buttons-container` rules, the `.dashboard-float-container { position: static }` override, and the ≤900px row block. (The already-empty `.btn-new-conversation` rule is untouched; it isn't made obsolete by this change.)
  - Add `.header-actions { grid-area: left; justify-self: start; display: flex; gap: var(--s-2); }`.
  - ≤900px: `.dashboard-label { display: none }` with icon padding.
- `styles/layout.css`: update the `.error-banner` comment (no more fixed buttons).
- `app/callbacks.rs`: `on_signout_click(app_state, session, ui_state)` in place: dispatch `SetDrawerOpen(false)`, then `sign_out`.
- `utility_sidebar/settings.rs`:
  - Prop `on_log_out: Callback<MouseEvent>`, added to the manual `PartialEq`.
  - Split `settings` into <20-line render fns: language section (language select, dialect select, existing experimental toggle), conversation section (formality, teaching mode, gender, level), display options (arabic script, japanese script), and new `render_account_section(on_log_out)`.
- `components/study_drawer_content.rs`: prop `on_log_out` → `Settings`. Split the fn into `render_tab_bar` and `render_tab_content` with per-tab helpers.
- `components/main_content.rs`: pass `on_log_out={on_signout_click(app_state.clone(), session.clone(), ui_state.clone())}` (import).

**Verification:**
- Tests + clippy.
- `bin/startup`, screenshots at 412px and desktop: one header row with ☰ ➕ on the left; no Sign Out; nothing overlapping.
- Dashboard → Settings → Account → Log out → sign-in screen with the drawer closed. I do not sign back in (the agent may not enter passwords); the user confirms that the next sign-in shows the chat without the drawer.

### Phase 3: Today's focus always visible, count opens full list, new items pinned (#2 "Today's focus" and new items; Decision 3; "Keep switching"; "One review row")
**Desktop outcome:** today's selected practice/fix/review targets are visibly identified beside the conversation without opening Dashboard or first expanding the phone disclosure. Existing access to the underlying items in Learning is insufficient.

**Changes:**
- `components/dynamic_island.rs`:
  - Replace `<details>/<summary>` with `<div class="island-summary">` (label plus a `<button class="island-summary-count">` when `review_count > 0`) followed by the always-rendered `.dynamic-island`.
  - New prop `on_open_review_list: Callback<()>`.
  - `MOBILE_REVIEW_LIMIT = 1`; new `MOBILE_QUEST_LIMIT = 2` used by `render_quests`.
  - Update the summary comment.
  - Split `dynamic_island` and the modified `render_quests` (<20 lines):
    - pure `quest_item_ids(quests)`, `available_items(items, ids)`, `effective_mode(state, available)`, all **tested**
    - `use_cycle_click(..)`, `render_summary(..)`, `render_island_body(..)`
- `components/main_content.rs`: `on_open_review_list` (`use_callback`: `drawer_active_tab.set(DrawerTab::Learning)` + `SetDrawerOpen(true)`) passed to `DynamicIsland`. Move `LearningItemsFlash` from the canvas siblings into the `ChatWindow` children.
- `styles/components/dynamic_island.css`:
  - **Delete** the `.island-disclosure > .dynamic-island` rule, the `::-webkit-details-marker` and `::after` chevron rules, the `[open]` selectors, and `cursor`/`list-style` on `.island-summary`.
  - At ≤900px, the existing flattening of the summary's bottom corners and the panel's top corners becomes unconditional.
  - Scope the caps to `.dynamic-island`: `.dynamic-island .island-item:nth-of-type(n + 3)` (1 row after the header) and `.island-quest-item:nth-of-type(n + 4)`.
  - Rewrite the stale comments: "the disclosure is inert here", "Collapsed by default …", "Only the first two review rows and two quests are shown" (`dynamic_island.css`), and "Rows past the second are hidden on a narrow screen" (`render_items` in `dynamic_island.rs`).
- `test_fixtures.rs`: add the `LearningItem` builder used by the `available_items` tests.
- `styles/components/learning_items_flash.css`: `position: absolute; top: var(--s-2); right: var(--s-2); max-height: calc(100% - var(--s-4));` and a comment.

**Verification:**
- Tests + clippy.
- `bin/startup`, then screenshots at 412px:
  - Focus content is visible without tapping; measure header, strip and conversation heights against the budget.
  - Tapping the panel switches views; Items shows 1 row + "+N more".
  - "N to review" opens the drawer on Learning.
  - 4 stand-in flash rows, inserted into `.chat` the way `LearningItemsFlash` renders, are all visible at the conversation's top-right and overlap neither header buttons, strip, nor composer.
- Desktop: load directly at desktop width with incomplete quests and verify that Daily Focus visibly identifies the selected practice/fix/review targets beside the conversation, without first expanding anything at phone width. Switching plan/goal/items keeps those targets visible. Dashboard → Learning still provides their underlying learning content. Capture a screenshot of the actual targets, not merely the panel container.
- Desktop new items now sit at the top-right of the conversation column, directly under the header, all visible (previously at the top-right of the page body).

## Phase gate (every phase, PSP §8)
1. `cargo test --workspace` green.
2. Clippy 0 warnings; no dead code; every modified fn under 20 lines.
3. `bin/startup`, then visual browser checks with screenshots.
4. Diff reviewed against this plan and the verbatim block.
5. Status doc updated.
6. Commit `Phase N (mobile layout) complete`.
7. Stop and report; wait for authorization.

## Requirement → behavior → phase → check
| Requirement | Behavior | Phase | Check that fails if absent |
|---|---|---|---|
| #1 buttons on phone selection | `selectionchange`-driven bar | 1 | no-mouse selection shows no bar; Android long-press |
| Decision 1 "fixed bar" | in-flow bar above the composer | 1 | bar off-screen, floating, or covering text |
| "Dashboard"/"New Conversation" on header | `.header-actions` | 2 | second row at 412px |
| "Sign Out" into dashboard (Logout setting) | Settings → Account → Log out | 2 | Sign Out in header / no Account section |
| "Today's focus" visible | disclosure removed | 3 | only a title row at 412px |
| Desktop identifies today's selected practice/fix/review targets | visible Daily Focus content beside the conversation | 3 | targets absent on direct desktop load, or only their underlying items accessible in Learning |
| Decision 3 open full list | count → Learning tab | 3 | drawer closed or wrong tab |
| "Keep switching" | tap cycles modes on phone | 3 | tap does nothing |
| "One review row" | limit 1 + scoped CSS | 3 | 2 rows shown |
| New items visible | flash in `.chat`, caps scoped | 3 | 4th stand-in hidden, or flash over buttons |
| Functions <20 lines incl. modified | splits listed per phase | 1–3 | line count per modified fn |
| Write a function → test; "Add basic frontend test scaffolding as needed, use manual browser checks for glue" | pure fns unit-tested with `test_fixtures`; glue browser-checked (D-g) | 1–3 | missing `#[test]` for a listed pure fn; missing screenshot for a glue path |
| "Approve all" (deletions) | only the listed deletions | 1–3 | diff deletes anything unlisted |
| 100% tests | full suite | all | non-zero exit |
| Preserve modal, shortcuts, drawer, undo | behavior-preserving splits | 1–3 | checks 1(f), 2, 3 |
