# Selective Text Translation Implementation Plan

## Overview

Replace the current "translate entire message" button with a text selection-based translation system. Users can select any text within messages (both agent and user) and a floating "Translate" button will appear to translate just the selected text (with full message context).

## User Requirements (2025-12-02)

User stated:
> "Right now, we send entire messages to a translation helper, and have it provide saveable translation items. This does not work for larger messages, especially since we want to be able to translate words and phrases both."

User wants:
- Select specific text within messages
- Translate only selected text (with full message as context)
- Save translation as learning item

**UI Choices:**
- "Button appears on text selection" (like Medium's highlight menu)
- "No, selection only" - Remove existing translate button
- "Both agent and user messages" - Support selection on all messages

## Current System Analysis

**Current Flow:**
1. User clicks "Translate" button on message bubble → `on_translate` callback fires
2. Entire message content sent to `/api/translate` endpoint
3. Backend segments entire message into phrases using AI
4. Modal displays all segmented phrases
5. User selects phrases to save as learning items

**Files Involved:**
- `frontend/src/components/message_bubble.rs` - Has "Translate" button (will remove)
- `frontend/src/components/translation_modal.rs` - Displays phrase list (keep)
- `frontend/src/services/translation.rs` - HTTP client for `/api/translate`
- `backend/src/translation_handler.rs` - Translation endpoint handler
- `shared/src/models/message.rs` - `TranslateRequest`, `TranslateResponse` types

**Current API:**
```rust
// Request
struct TranslateRequest {
    phrase: String,           // Currently: entire message
    dialect: String,
    formality: Option<String>,
}

// Response
struct TranslateResponse {
    original_sentence: String,
    segmented_phrases: Vec<PhraseTranslation>,
    success: bool,
    error: Option<String>,
}
```

## Proposed Architecture

### New Flow

1. User selects text within a message bubble
2. Floating "Translate" button appears near selection
3. User clicks button → selected text + full message sent to API
4. Backend translates selected text using full message as context
5. Modal displays translation result
6. User saves translation with full message as context

### API Changes

```rust
// Updated TranslateRequest (in shared/src/models/message.rs)
struct TranslateRequest {
    phrase: String,              // Selected text to translate
    context: Option<String>,     // NEW: Full message for context
    dialect: String,
    formality: Option<String>,
}
```

**Backend prompt changes:**
- When `context` is provided: "Translate '{phrase}' in the context of '{context}'"
- When `context` is None: "Translate '{phrase}'" (backward compatible)

### Frontend Components

**New Component: `TranslateSelectionButton`**
- Floating button positioned near text selection
- Props:
  - `selected_text: String`
  - `message_context: String`
  - `on_translate: Callback<(String, String)>` - (selected_text, context)
  - `position: (f64, f64)` - x, y coordinates for positioning
- Appears only when text is selected
- Disappears when selection is cleared

**Updated Component: `MessageBubble`**
- Remove "Translate" button
- Add selection detection on message text
- Track selection state: `selected_text: Option<String>`
- Render `TranslateSelectionButton` when selection exists
- Calculate button position based on selection coordinates

**Keep: `TranslationModal`**
- No changes needed - already displays phrases from API response
- Already handles saving with context

### Text Selection Implementation

**Selection Detection:**
- Listen to `mouseup` event on message content div
- Call `window().get_selection()` via `web_sys`
- Check if selection is non-empty and within message bounds
- Extract selected text string
- Calculate selection bounding box for button positioning

**Selection Clearing:**
- Listen to `mousedown` outside message
- Listen to `selectionchange` event
- Clear selection state when selection becomes empty

## Implementation Phases

### Phase 1: Backend API Context Support

**Agent: modular-builder**

**Code Style Checklist:**
- [ ] Functions <20 lines
- [ ] Helper functions for complex logic
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No dead code

**Deliverables:**
1. Update `TranslateRequest` in `shared/src/models/message.rs` to add `context: Option<String>` field
2. Update `translate_phrase()` in `backend/src/translation_handler.rs` to use context in AI prompt when provided
3. Test that translation works with and without context

**Files to Modify:**
- `shared/src/models/message.rs` - Add context field to TranslateRequest
- `frontend/src/services/translation.rs` - Update TranslateRequest struct definition (duplicate)
- `backend/src/translation_handler.rs` - Update prompt generation to use context

**Detailed Tasks:**

1. **Update shared TranslateRequest type:**
   ```rust
   // In shared/src/models/message.rs
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct TranslateRequest {
       pub phrase: String,
       pub context: Option<String>,  // ADD THIS
       pub dialect: String,
       pub formality: Option<String>,
   }
   ```

2. **Update frontend TranslateRequest:**
   ```rust
   // In frontend/src/services/translation.rs
   #[derive(Serialize)]
   struct TranslateRequest {
       phrase: String,
       context: Option<String>,  // ADD THIS
       dialect: String,
       formality: Option<String>,
   }
   ```

3. **Update backend translation prompt:**
   ```rust
   // In backend/src/translation_handler.rs, translate_phrase() function

   // Current prompt (around line 63):
   let prompt = format!(
       "Translate the following phrase into {} ({}): \"{}\"",
       dialect_name, formality_desc, phrase
   );

   // New prompt with context support:
   let prompt = if let Some(ctx) = &request.context {
       format!(
           "Translate the phrase \"{}\" into {} ({}), in the context of this sentence: \"{}\"",
           phrase, dialect_name, formality_desc, ctx
       )
   } else {
       format!(
           "Translate the following phrase into {} ({}): \"{}\"",
           dialect_name, formality_desc, phrase
       )
   };
   ```

4. **Update translation service method signature:**
   ```rust
   // In frontend/src/services/translation.rs
   pub async fn translate_phrase(
       &self,
       phrase: &str,
       context: Option<String>,  // ADD THIS
       dialect: Dialect,
       formality: Option<Formality>,
   ) -> Result<TranslateResponse, String>
   ```

**Acceptance Criteria:**
- `cargo check` passes
- Translation API accepts context field
- Backend uses context in prompt when provided
- Backward compatible (works without context)

**Phase End:**
- Run full test suite → 100% pass
- Run `cargo clippy` → clean
- Update status doc with Phase 1 complete
- `git add . && git commit -m "Phase 1 (backend context support) complete"`

---

### Phase 2: Text Selection Detection in MessageBubble

**Agent: modular-builder**

**Code Style Checklist:**
- [ ] Functions <20 lines
- [ ] Helper functions for complex logic
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No dead code

**Deliverables:**
1. Add text selection detection to `MessageBubble` component
2. Track selected text and selection position in component state
3. Handle selection clearing on deselection
4. Remove existing "Translate" button from message actions

**Files to Modify:**
- `frontend/src/components/message_bubble.rs` - Add selection handling, remove translate button

**Detailed Tasks:**

1. **Add selection state to MessageBubble:**
   ```rust
   // In message_bubble.rs, add to function component state
   let selection_state = use_state(|| None::<TextSelection>);

   #[derive(Clone, PartialEq)]
   struct TextSelection {
       text: String,
       position: (f64, f64),  // x, y coordinates for button
   }
   ```

2. **Add mouseup handler to message content:**
   ```rust
   let on_mouseup = {
       let selection_state = selection_state.clone();
       Callback::from(move |_: MouseEvent| {
           if let Some(selection) = get_text_selection() {
               selection_state.set(Some(selection));
           }
       })
   };
   ```

3. **Create get_text_selection() helper:**
   ```rust
   fn get_text_selection() -> Option<TextSelection> {
       use web_sys::window;

       let window = window()?;
       let selection = window.get_selection().ok()??;

       if selection.is_collapsed() {
           return None;
       }

       let text = selection.to_string().as_string()?;
       if text.trim().is_empty() {
           return None;
       }

       let range = selection.get_range_at(0).ok()?;
       let rect = range.get_bounding_client_rect();

       Some(TextSelection {
           text,
           position: (rect.right(), rect.bottom()),
       })
   }
   ```

4. **Add selection clearing on click outside:**
   ```rust
   // Use effect to add document-level click listener
   use_effect_with((), {
       let selection_state = selection_state.clone();
       move |_| {
           let clear_selection = Callback::from(move |_: MouseEvent| {
               selection_state.set(None);
           });

           // Add listener to document
           let document = web_sys::window()?.document()?;
           let listener = EventListener::new(&document, "mousedown", clear_selection);

           // Cleanup
           move || drop(listener)
       }
   });
   ```

5. **Remove existing translate button:**
   - Delete "Translate" button from action buttons section
   - Remove `on_translate` prop from MessageBubbleProps
   - Remove translate click handler

**Acceptance Criteria:**
- `cargo check` passes
- Text selection is detected on mouseup
- Selection state is cleared on click outside
- Selection coordinates are captured
- Old translate button is removed

**Phase End:**
- Run full test suite → 100% pass
- Run `cargo clippy` → clean
- Update status doc with Phase 2 complete
- `git add . && git commit -m "Phase 2 (text selection detection) complete"`

---

### Phase 3: Floating Translate Button Component

**Agent: modular-builder**

**Code Style Checklist:**
- [ ] Functions <20 lines
- [ ] Helper functions for complex logic
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No dead code

**Deliverables:**
1. Create new `TranslateSelectionButton` component
2. Position button absolutely near text selection
3. Style button to match existing UI
4. Wire up click handler to emit selected text + context

**Files to Create:**
- `frontend/src/components/translate_selection_button.rs`

**Files to Modify:**
- `frontend/src/components/mod.rs` - Add new module
- `frontend/src/components/message_bubble.rs` - Render button when selection exists

**Detailed Tasks:**

1. **Create TranslateSelectionButton component:**
   ```rust
   // In frontend/src/components/translate_selection_button.rs

   use yew::prelude::*;

   #[derive(Properties, PartialEq)]
   pub struct Props {
       pub selected_text: String,
       pub message_context: String,
       pub position: (f64, f64),
       pub on_translate: Callback<(String, String)>,
   }

   #[function_component(TranslateSelectionButton)]
   pub fn translate_selection_button(props: &Props) -> Html {
       let on_click = {
           let on_translate = props.on_translate.clone();
           let selected_text = props.selected_text.clone();
           let context = props.message_context.clone();

           Callback::from(move |e: MouseEvent| {
               e.stop_propagation();
               on_translate.emit((selected_text.clone(), context.clone()));
           })
       };

       let style = format!(
           "position: absolute; left: {}px; top: {}px; z-index: 1000;",
           props.position.0, props.position.1
       );

       html! {
           <button
               class="translate-selection-btn"
               style={style}
               onclick={on_click}
           >
               {"Translate"}
           </button>
       }
   }
   ```

2. **Add module to mod.rs:**
   ```rust
   // In frontend/src/components/mod.rs
   pub mod translate_selection_button;
   pub use translate_selection_button::TranslateSelectionButton;
   ```

3. **Render button in MessageBubble:**
   ```rust
   // In message_bubble.rs, in the html! macro

   {
       if let Some(selection) = &*selection_state {
           html! {
               <TranslateSelectionButton
                   selected_text={selection.text.clone()}
                   message_context={props.message.as_str().to_string()}
                   position={selection.position}
                   on_translate={on_selection_translate}
               />
           }
       } else {
           html! {}
       }
   }
   ```

4. **Add CSS styling:**
   ```css
   /* In frontend/styles/main.css or appropriate stylesheet */

   .translate-selection-btn {
       background-color: var(--primary-color);
       color: white;
       border: none;
       border-radius: 4px;
       padding: 6px 12px;
       font-size: 14px;
       cursor: pointer;
       box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
   }

   .translate-selection-btn:hover {
       background-color: var(--primary-color-dark);
   }
   ```

**Acceptance Criteria:**
- `cargo check` passes
- Button appears near selected text
- Button styling matches existing UI
- Button click emits (selected_text, context) tuple
- Button disappears when selection is cleared

**Phase End:**
- Run full test suite → 100% pass
- Run `cargo clippy` → clean
- Update status doc with Phase 3 complete
- `git add . && git commit -m "Phase 3 (floating translate button) complete"`

---

### Phase 4: Wire Up Translation Flow

**Agent: modular-builder**

**Code Style Checklist:**
- [ ] Functions <20 lines
- [ ] Helper functions for complex logic
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No dead code

**Deliverables:**
1. Connect floating button click to translation service
2. Update main_content.rs to handle selection-based translation
3. Pass selected text + context to translation API
4. Display results in existing translation modal
5. Ensure learning items save with full message context

**Files to Modify:**
- `frontend/src/components/main_content.rs` - Update translation flow
- `frontend/src/components/chat_window.rs` - Pass through selection translate callback
- `frontend/src/components/message_bubble.rs` - Emit selection translate event

**Detailed Tasks:**

1. **Add selection translate callback to MessageBubble props:**
   ```rust
   // In message_bubble.rs
   #[derive(Properties, PartialEq)]
   pub struct MessageBubbleProps {
       // ... existing props ...
       pub on_selection_translate: Option<Callback<(Uuid, String, String)>>,
       // (message_id, selected_text, full_context)
   }
   ```

2. **Implement selection translate handler:**
   ```rust
   // In message_bubble.rs
   let on_selection_translate = {
       let message_id = props.message.id();
       let callback = props.on_selection_translate.clone();

       Callback::from(move |(selected_text, context): (String, String)| {
           if let Some(cb) = &callback {
               cb.emit((message_id, selected_text, context));
           }
       })
   };
   ```

3. **Pass callback through ChatWindow:**
   ```rust
   // In chat_window.rs
   #[derive(Properties, PartialEq)]
   pub struct Props {
       // ... existing props ...
       pub on_selection_translate: Option<Callback<(Uuid, String, String)>>,
   }

   // In render, pass to MessageBubble
   <MessageBubble
       // ... existing props ...
       on_selection_translate={props.on_selection_translate.clone()}
   />
   ```

4. **Update main_content.rs translation handler:**
   ```rust
   // In main_content.rs

   let on_selection_translate_click = {
       let translation_service = translation_service.clone();
       let user_state = user_state.clone();
       let ui_dispatch = ui_dispatch.clone();

       Callback::from(move |(message_id, selected_text, context): (Uuid, String, String)| {
           let translation_service = translation_service.clone();
           let user_state = user_state.clone();
           let ui_dispatch = ui_dispatch.clone();

           wasm_bindgen_futures::spawn_local(async move {
               ui_dispatch.dispatch(UIStateAction::SetTranslateLoading(message_id, true));

               let dialect = user_state.current_dialect();
               let formality = Some(user_state.formality);

               match translation_service
                   .translate_phrase(&selected_text, Some(context.clone()), dialect, formality)
                   .await
               {
                   Ok(response) => {
                       ui_dispatch.dispatch(UIStateAction::SetTranslationModal(Some(
                           TranslationModalState {
                               original_sentence: context,  // Use full context as original
                               phrases: response.segmented_phrases,
                           },
                       )));
                   }
                   Err(e) => {
                       log::error!("Translation failed: {}", e);
                   }
               }

               ui_dispatch.dispatch(UIStateAction::SetTranslateLoading(message_id, false));
           });
       })
   };
   ```

5. **Wire up in ChatWindow render:**
   ```rust
   // In main_content.rs
   <ChatWindow
       // ... existing props ...
       on_selection_translate={Some(on_selection_translate_click)}
   />
   ```

6. **Remove old translate button callback:**
   - Remove `on_translate_click` callback from main_content.rs
   - Remove `on_translate` prop from ChatWindow
   - Remove `on_translate` prop passing in chat_window.rs

**Acceptance Criteria:**
- `cargo check` passes
- Selecting text and clicking "Translate" sends API request
- API request includes selected text and full message context
- Translation modal displays results
- Learning items save with full message as context
- Old translate button flow is removed

**Phase End:**
- Run full test suite → 100% pass
- Run `cargo clippy` → clean
- Update status doc with Phase 4 complete
- `git add . && git commit -m "Phase 4 (translation flow integration) complete"`

---

### Phase 5: Testing & Edge Cases

**Agent: kiss-code-generator**

**Code Style Checklist:**
- [ ] Functions <20 lines
- [ ] Helper functions for complex logic
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No dead code

**Deliverables:**
1. Test with various selection lengths (single word, phrase, full sentence)
2. Test on both agent and user messages
3. Handle edge cases (empty selection, cross-element selection)
4. Verify learning items save correctly with context
5. Test modal display and phrase saving

**Test Cases:**

1. **Single word selection:**
   - Select single word in agent message → translate → verify modal shows result
   - Select single word in user message → translate → verify modal shows result

2. **Phrase selection:**
   - Select multi-word phrase → translate → verify segmented phrases
   - Save phrase → verify learning item has full message context

3. **Full sentence selection:**
   - Select entire message text → translate → verify works same as old system
   - Verify context equals selected text in this case

4. **Edge cases:**
   - Click translate with empty selection → should not trigger
   - Select text across multiple messages → should only use first message
   - Select text then click outside → button disappears
   - Rapid selection changes → only latest selection matters

5. **Learning item verification:**
   - Translate and save → check learning_items in UserState
   - Verify context field contains full original message
   - Verify translated_word and translated_to are correct

**Manual Testing Checklist:**
- [ ] Single word translation works
- [ ] Multi-word phrase translation works
- [ ] Full sentence translation works
- [ ] Button appears on selection
- [ ] Button disappears on deselection
- [ ] Translation modal displays correctly
- [ ] Saving creates correct learning item with context
- [ ] Works on agent messages
- [ ] Works on user messages
- [ ] Old translate button is gone

**Phase End:**
- Run full test suite → 100% pass
- Run `cargo clippy` → clean
- Update status doc with Phase 5 complete
- `git add . && git commit -m "Phase 5 (testing and edge cases) complete"`

---

## Success Criteria

- User can select any text in any message (agent or user)
- Floating "Translate" button appears near selection
- Only selected text is translated (with full message context)
- Translation modal displays segmented phrases
- Learning items save with full message as context
- Old "translate entire message" button is removed
- All tests pass
- No clippy warnings
- No dead code

## Files Summary

**Shared:**
- `shared/src/models/message.rs` - Add context to TranslateRequest

**Backend:**
- `backend/src/translation_handler.rs` - Use context in translation prompt

**Frontend:**
- `frontend/src/components/message_bubble.rs` - Selection detection, remove old button
- `frontend/src/components/translate_selection_button.rs` - New floating button component
- `frontend/src/components/chat_window.rs` - Pass through selection callback
- `frontend/src/components/main_content.rs` - Update translation flow
- `frontend/src/services/translation.rs` - Add context parameter
- `frontend/styles/main.css` - Button styling

## Technical Notes

**Text Selection API:**
- Use `window.getSelection()` to get selected text
- Use `selection.getRangeAt(0).getBoundingClientRect()` for button positioning
- Check `selection.isCollapsed()` to detect empty selection

**Event Handling:**
- `mouseup` on message div to detect selection
- `mousedown` on document to clear selection
- `click` on translate button to trigger translation

**Positioning:**
- Use absolute positioning for floating button
- Position at `selection.right, selection.bottom` coordinates
- Add z-index to ensure button appears above other elements

**Context Preservation:**
- Full message text passed as context to API
- Context stored in learning item for reference
- Context displayed in modal as "original_sentence"
