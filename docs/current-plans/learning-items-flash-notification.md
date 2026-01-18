# Learning Items Flash Notification

## Overview

When the agent sends back new learning items, display them in a notification list on the right side of the screen. Items queue up as they arrive. Clicking an item dismisses it.

## Requirements

1. New learning items appear in a list on the right side of the screen
2. Clicking an item dismisses that specific item
3. New items are appended to the list (queue behavior)
4. No auto-dismiss (user clicks to dismiss)

## Research Summary

### Data Flow (Current)
```
Backend AgentResponse
  → WebSocket (websocket_hooks.rs:84-107)
  → LearningAction::AddItems(mistakes, explained, translated, exploratory)
  → UserState reducer
```

### Key Files
| File | Purpose |
|------|---------|
| `frontend/src/app/websocket_hooks.rs:90-107` | Where items arrive from agent |
| `frontend/src/app/app_state/ui.rs` | UIState and actions |
| `frontend/index.html` | CSS loading via `<link data-trunk rel="css">` |
| `frontend/src/components/mod.rs` | Component exports |
| `frontend/src/components/main_content.rs` | Where component will be rendered |

### Item Construction
Items arrive as tuples and convert via:
```rust
LearningItem::with_score(LearningItemType::Mistake(mistake), dialect, score)
```

### Dialect Access in WebSocket
`session_handle` is `UseReducerHandle<SessionState>`. Need to check how to get dialect from session state.

---

## Phase 1: Complete Flash Notification Feature

**Subagent**: modular-builder (cross-file work)

### Code Style Checklist
- [ ] Functions < 20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No dead code

### Files to Create
| File | Purpose |
|------|---------|
| `frontend/src/components/learning_items_flash.rs` | Flash notification component |
| `frontend/styles/components/learning_items_flash.css` | Component styling |

### Files to Modify
| File | Changes |
|------|---------|
| `frontend/src/app/app_state/ui.rs` | Add `flashed_items` field, `QueueFlashItems`, `DismissFlashItem` actions |
| `frontend/src/app/websocket_hooks.rs` | Dispatch `QueueFlashItems` when items arrive |
| `frontend/src/components/mod.rs` | Export `LearningItemsFlash` |
| `frontend/src/components/main_content.rs` | Render `LearningItemsFlash` component |
| `frontend/index.html` | Add CSS link for `learning_items_flash.css` |

### Detailed Changes

#### 1. ui.rs

Add to imports:
```rust
use uuid::Uuid;
```

Add to `UIStateAction` enum:
```rust
QueueFlashItems(Vec<LearningItem>),
DismissFlashItem(Uuid),
```

Add to `UIState` struct:
```rust
pub flashed_items: Vec<LearningItem>,
```

Add to `apply_action` match:
```rust
UIStateAction::QueueFlashItems(items) => {
    next.flashed_items.extend(items);
}
UIStateAction::DismissFlashItem(id) => {
    next.flashed_items.retain(|item| item.id() != id);
}
```

#### 2. websocket_hooks.rs

Add to imports:
```rust
use dialect_coach_shared::{LearningItem, LearningItemType};
```

Replace lines 95-107 (the existing item dispatch block) with:
```rust
if !mistakes.is_empty()
    || !explained.is_empty()
    || !translated.is_empty()
    || !exploratory.is_empty()
{
    // Get dialect from session for constructing LearningItems
    let dialect = session_handle.user.as_ref()
        .map(|u| u.active_branch_dialect())
        .unwrap_or_default();

    // Build flash items
    let mut flash_items = Vec::new();
    for (m, s) in &mistakes {
        flash_items.push(LearningItem::with_score(
            LearningItemType::Mistake(m.clone()),
            dialect,
            *s,
        ));
    }
    for (e, s) in &explained {
        flash_items.push(LearningItem::with_score(
            LearningItemType::Explanation(e.clone()),
            dialect,
            *s,
        ));
    }
    for (t, s) in &translated {
        flash_items.push(LearningItem::with_score(
            LearningItemType::Translation(t.clone()),
            dialect,
            *s,
        ));
    }
    for (x, s) in &exploratory {
        flash_items.push(LearningItem::with_score(
            LearningItemType::Exploration(x.clone()),
            dialect,
            *s,
        ));
    }

    // Queue items for flash display
    uis.dispatch(UIStateAction::QueueFlashItems(flash_items));

    // Existing: add to user state
    session_handle.dispatch(SessionAction::Domain(
        UserDomainAction::Learning(LearningAction::AddItems(
            mistakes,
            explained,
            translated,
            exploratory,
        )),
    ));
}
```

#### 3. learning_items_flash.rs (new file)

```rust
use dialect_coach_shared::{LearningItem, LearningItemType};
use uuid::Uuid;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct LearningItemsFlashProps {
    pub items: Vec<LearningItem>,
    pub on_dismiss: Callback<Uuid>,
}

#[function_component(LearningItemsFlash)]
pub fn learning_items_flash(props: &LearningItemsFlashProps) -> Html {
    if props.items.is_empty() {
        return html! {};
    }

    html! {
        <div class="learning-items-flash">
            {for props.items.iter().map(|item| {
                render_flash_item(item, props.on_dismiss.clone())
            })}
        </div>
    }
}

fn render_flash_item(item: &LearningItem, on_dismiss: Callback<Uuid>) -> Html {
    let id = item.id();
    let onclick = Callback::from(move |_| on_dismiss.emit(id));
    let (icon, text) = get_item_display(item);

    html! {
        <div class="flash-item" {onclick} title="Click to dismiss">
            <span class="flash-item-icon">{icon}</span>
            <span class="flash-item-text">{text}</span>
        </div>
    }
}

fn get_item_display(item: &LearningItem) -> (&'static str, String) {
    match &item.item {
        LearningItemType::Mistake(m) => ("🛠️", m.specific_mistake.clone()),
        LearningItemType::Explanation(e) => ("💡", e.new_phrase.clone()),
        LearningItemType::Translation(t) => {
            ("🌐", format!("{} → {}", t.translated_word, t.translated_to))
        }
        LearningItemType::Exploration(x) => ("🎯", x.point_to_try.clone()),
    }
}
```

#### 4. learning_items_flash.css (new file)

```css
/* Learning Items Flash - Right-side notification list */
.learning-items-flash {
    position: fixed;
    top: var(--s-4);
    right: var(--s-4);
    z-index: 1000;
    display: flex;
    flex-direction: column;
    gap: var(--s-2);
    max-width: 320px;
    max-height: calc(100vh - var(--s-8));
    overflow-y: auto;
    pointer-events: none;
}

.flash-item {
    pointer-events: auto;
    display: flex;
    align-items: flex-start;
    gap: var(--s-2);
    padding: var(--s-3);
    background: linear-gradient(135deg, rgba(30, 30, 30, 0.95), rgba(20, 20, 20, 0.98));
    border: 1px solid rgba(78, 205, 196, 0.3);
    border-radius: var(--r-m);
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3);
    cursor: pointer;
    transition: all var(--dur-fast) var(--ease-out);
    animation: flash-slide-in 0.3s var(--ease-out);
}

.flash-item:hover {
    background: linear-gradient(135deg, rgba(40, 40, 40, 0.95), rgba(30, 30, 30, 0.98));
    border-color: rgba(78, 205, 196, 0.5);
    transform: translateX(-4px);
}

@keyframes flash-slide-in {
    from {
        opacity: 0;
        transform: translateX(20px);
    }
    to {
        opacity: 1;
        transform: translateX(0);
    }
}

.flash-item-icon {
    font-size: var(--text-base);
    flex-shrink: 0;
}

.flash-item-text {
    font-size: var(--text-sm);
    color: rgba(255, 255, 255, 0.9);
    line-height: 1.4;
    word-break: break-word;
}
```

#### 5. mod.rs

Add line after other module declarations:
```rust
pub mod learning_items_flash;
```

Add to exports section:
```rust
pub use learning_items_flash::LearningItemsFlash;
```

#### 6. index.html

Add after line 31 (after `gamification.css`):
```html
    <link data-trunk rel="css" href="styles/components/learning_items_flash.css">
```

#### 7. main_content.rs

Add to imports (with other component imports):
```rust
use crate::components::LearningItemsFlash;
```

Add dismiss callback (near other callbacks):
```rust
let on_dismiss_flash = {
    let ui_state = ui_state.clone();
    Callback::from(move |id: Uuid| {
        ui_state.dispatch(UIStateAction::DismissFlashItem(id));
    })
};
```

Add component render (inside the returned HTML, as a sibling to main content area):
```rust
<LearningItemsFlash
    items={(*ui_state).flashed_items.clone()}
    on_dismiss={on_dismiss_flash}
/>
```

### Deliverables
- [ ] New learning items appear in list on right side when agent sends them
- [ ] Clicking an item dismisses it from the list
- [ ] Multiple items queue up in the list
- [ ] CSS loads correctly and items are styled
- [ ] No dead code

### Phase End Verification

1. Run full test suite:
```bash
cargo test --package dialect-coach-frontend
```

2. Run clippy and fix ALL errors:
```bash
cargo clippy --package dialect-coach-frontend
```

3. Verify no dead code warnings

4. Manual test:
   - Send a message that generates learning items
   - Verify items appear on right side
   - Click an item to dismiss it
   - Verify new items append to list

5. On 100% success, commit:
```bash
git add -A && git commit -m "Phase 1 (learning items flash notification) complete"
```

---

## Status

| Phase | Status | Date |
|-------|--------|------|
| Phase 1 | Complete | 2026-01-17 |
