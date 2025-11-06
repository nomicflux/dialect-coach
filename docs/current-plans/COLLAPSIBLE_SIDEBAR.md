# Collapsible Sidebar and Button Spacing Fix

## Overview

1. Make the branch sidebar collapsible to a narrow strip (40-60px) showing small branch indicators
2. Add keyboard shortcut for toggling sidebar (Ctrl/Cmd + B)
3. Ensure learning panel toggle button doesn't block the send button

## Files to Modify

### 1. State Management (`frontend/src/app/app_state.rs`)
- Add `sidebar_collapsed: bool` field to `UIState` struct (default: `false`)
- Add `ToggleSidebar` action to `UIStateAction` enum
- Handle `ToggleSidebar` in `UIState::apply_action`

### 2. Branch Sidebar Component (`frontend/src/components/branch_sidebar.rs`)
- Add `is_collapsed` prop to `BranchSidebarProps`
- Add `on_toggle` callback prop
- Create collapsed view rendering:
  - Small vertical indicators for each branch (colored dots/strips)
  - Active branch highlighted differently
  - Learning goals count badge
  - Toggle button (chevron/arrow icon)
- Create expanded view (current implementation)
- Conditionally render based on `is_collapsed`

### 3. Main Content (`frontend/src/components/main_content.rs`)
- Pass `sidebar_collapsed` state from `ui_state` to `BranchSidebar`
- Pass toggle callback that dispatches `UIStateAction::ToggleSidebar`
- Add keyboard event listener for sidebar toggle (e.g., `Ctrl/Cmd + B`)

### 4. Branch Sidebar CSS (`frontend/styles/components/branch_sidebar.css`)
- Add `.branch-sidebar--collapsed` class with width ~50px
- Style collapsed indicators:
  - Vertical branch indicators (dots or thin colored bars)
  - Active branch indicator (different color/size)
  - Learning goals count badge positioned at bottom
  - Toggle button styling
- Add transition animations for collapse/expand
- Update main content padding when collapsed (adjust `app-main` padding-left)

### 5. Learning Panel CSS (`frontend/styles/components/learning_panel.css`)
- Adjust `.learning-panel-toggle` position:
  - Increase `bottom` value to ensure it doesn't overlap with composer/send button
  - Or move to `top` position relative to chat container
  - Ensure minimum spacing from bottom of viewport

### 6. Layout CSS (`frontend/styles/layout.css`)
- Update `.app-main` padding-left calculation:
  - When sidebar expanded: `280px` (current)
  - When sidebar collapsed: `~60px` (narrow strip width)
  - Use CSS variable or data attribute for dynamic spacing

## Implementation Phases

### Phase 1: Add Sidebar State Management

#### Code Style Checklist
- [x] Functions <20 lines, prefer <10 lines
- [x] Use reducer pattern (no direct state mutation)
- [x] Keep action handlers simple - just update state
- [x] Add clear action names: `ToggleSidebar` not `Toggle` or `Collapse`
- [x] Follow existing `UIStateAction` enum pattern
- [x] Use `cargo fmt` to format code
- [x] Use `cargo check` to verify compilation

#### 1.1 Add `sidebar_collapsed` to UIState
**File:** `frontend/src/app/app_state.rs`

Added field to `UIState` struct:
```rust
pub struct UIState {
    // ... existing fields
    pub sidebar_collapsed: bool,
}
```

Updated `Default` implementation:
```rust
impl Default for UIState {
    fn default() -> Self {
        Self {
            // ... existing fields
            sidebar_collapsed: false,
        }
    }
}
```

#### 1.2 Add `ToggleSidebar` action
**File:** `frontend/src/app/app_state.rs`

Added to `UIStateAction` enum:
```rust
pub enum UIStateAction {
    // ... existing actions
    ToggleSidebar,
}
```

Added handler in `UIState::apply_action`:
```rust
match action {
    // ... existing matches
    UIStateAction::ToggleSidebar => next.sidebar_collapsed = !next.sidebar_collapsed,
}
```

#### Phase 1 Completion Checklist
- [x] `sidebar_collapsed` field added to `UIState`
- [x] `ToggleSidebar` action added to `UIStateAction`
- [x] Handler implemented in `apply_action`
- [x] `cargo check` passes
- [x] **Update this planning document with Phase 1 status**

---

### Phase 2: Modify Branch Sidebar Component

#### Code Style Checklist
- [x] Component render functions <20 lines total
- [x] Extract collapsed/expanded views into helper functions
- [x] Keep callbacks simple - just dispatch actions
- [x] No inline styles - use CSS classes
- [x] Clear prop names: `is_collapsed` not `collapsed`
- [x] Use `cargo fmt` to format code
- [x] Use `cargo check` to verify compilation

#### 2.1 Update BranchSidebarProps
**File:** `frontend/src/components/branch_sidebar.rs`

Added props to `BranchSidebarProps`:
```rust
#[derive(Properties, PartialEq)]
pub struct BranchSidebarProps {
    // ... existing props
    pub is_collapsed: bool,
    pub on_toggle: Callback<()>,
}
```

#### 2.2 Create collapsed view helper
**File:** `frontend/src/components/branch_sidebar.rs`

Created helper function to render collapsed indicators:
```rust
fn render_collapsed_branch_indicator(
    _branch: &ConversationBranch,
    is_active: bool,
    message_count: usize,
) -> Html {
    let indicator_class = if is_active {
        "branch-indicator branch-indicator--active"
    } else {
        "branch-indicator"
    };
    html! {
        <div class={indicator_class} title={format!("{} messages", message_count)}>
        </div>
    }
}
```

#### 2.3 Create collapsed sidebar rendering
**File:** `frontend/src/components/branch_sidebar.rs`

Added collapsed view to component:
- Shows branch indicators vertically
- Shows learning goals count badge at bottom
- Shows toggle button to expand

#### 2.4 Update main component render
**File:** `frontend/src/components/branch_sidebar.rs`

Updated `BranchSidebar` component to conditionally render:
```rust
if props.is_collapsed {
    render_collapsed_view(props)
} else {
    render_expanded_view(props)
}
```

#### Phase 2 Completion Checklist
- [x] Props added to `BranchSidebarProps`
- [x] Collapsed view helper functions created
- [x] Component conditionally renders based on `is_collapsed`
- [x] Toggle button renders in collapsed view
- [x] `cargo check` passes
- [x] **Update this planning document with Phase 2 status**

---

### Phase 3: Connect State to Components

#### Code Style Checklist
- [x] Callback functions <20 lines
- [x] Keep callbacks simple - just dispatch actions
- [x] Extract keyboard event handling into helper hook if needed
- [x] Use `cargo fmt` to format code
- [x] Use `cargo check` to verify compilation

#### 3.1 Pass state to BranchSidebar
**File:** `frontend/src/components/main_content.rs`

Updated `BranchSidebar` component call:
```rust
<BranchSidebar
    // ... existing props
    is_collapsed={ui_state.sidebar_collapsed}
    on_toggle={Callback::from({
        let ui_state = ui_state.clone();
        move |_| {
            ui_state.dispatch(UIStateAction::ToggleSidebar);
        }
    })}
/>
```

#### 3.2 Add keyboard shortcut
**File:** `frontend/src/components/main_content.rs`

Added keyboard event listener using `use_effect`:
```rust
use_effect_with((), move |_| {
    let listener = EventListener::new(&window(), "keydown", move |e| {
        if let Some(event) = e.dyn_ref::<KeyboardEvent>() {
            let is_ctrl_or_cmd = event.ctrl_key() || event.meta_key();
            if is_ctrl_or_cmd && (event.key() == "b" || event.key() == "B") {
                event.prevent_default();
                ui_state.dispatch(UIStateAction::ToggleSidebar);
            }
        }
    });
    move || drop(listener)
});
```

#### Phase 3 Completion Checklist
- [x] `sidebar_collapsed` state passed to `BranchSidebar`
- [x] Toggle callback connected
- [x] Keyboard shortcut implemented (Ctrl/Cmd + B)
- [x] Keyboard shortcut works to toggle sidebar
- [x] `cargo check` passes
- [x] **Update this planning document with Phase 3 status**

---

### Phase 4: Add CSS for Collapsed Sidebar

#### Code Style Checklist
- [x] Use CSS variables for colors/spacing
- [x] Add smooth transitions for collapse/expand
- [x] Keep CSS organized by component
- [x] Use semantic class names: `.branch-sidebar--collapsed`
- [x] Test responsive behavior

#### 4.1 Collapsed sidebar styles
**File:** `frontend/styles/components/branch_sidebar.css`

Added collapsed state styles:
- `.branch-sidebar--collapsed` with width 50px
- `.branch-indicator` for small branch indicators
- `.branch-indicator--active` for active branch indicator
- `.learning-goals-count` for learning goals count badge
- `.sidebar-toggle` for toggle button styling

#### 4.2 Update layout spacing
**File:** `frontend/styles/layout.css`

Updated `.app-main` padding calculation:
- Uses data attribute `data-sidebar-collapsed` based on sidebar state
- Adjusts padding-left when sidebar is collapsed (60px)

**File:** `frontend/src/app.rs`

Added data attribute to `app-main` element:
```rust
<main class="app-main" data-sidebar-collapsed={if ui_state.sidebar_collapsed { "true" } else { "false" }}>
```

#### Phase 4 Completion Checklist
- [x] Collapsed sidebar CSS added
- [x] Branch indicator styles implemented
- [x] Learning goals count badge styled
- [x] Layout spacing updates when collapsed
- [x] Transitions work smoothly
- [x] **Update this planning document with Phase 4 status**

---

### Phase 5: Fix Learning Panel Button Spacing

#### Code Style Checklist
- [x] Use CSS variables for positioning
- [x] Ensure accessible spacing
- [x] Test on different screen sizes
- [x] Verify button doesn't overlap with composer

#### 5.1 Adjust button position
**File:** `frontend/styles/components/learning_panel.css`

Updated `.learning-panel-toggle` position:
- Changed `bottom` from `80px` to `150px` to ensure clearance from composer
- Ensures minimum 30px spacing from send button

#### Phase 5 Completion Checklist
- [x] Learning panel button position adjusted
- [x] Button doesn't block send button
- [x] Spacing verified on different screen sizes
- [x] Button remains accessible
- [x] **Update this planning document with Phase 5 status**

---

## Implementation Details

### Collapsed Sidebar Design
- Width: ~50px when collapsed
- Branch indicators: Small colored vertical bars (8px width, 12px when active/hovered) aligned vertically
- Active branch: Different color/border (teal) to indicate current branch
- Learning goals: Count badge (e.g., "3") at bottom of collapsed sidebar
- Toggle button: Chevron icon (◄/►) at top of sidebar

### Keyboard Shortcut
- Default: `Ctrl+B` (Windows/Linux) or `Cmd+B` (Mac)
- Listens for keydown events in main content component
- Dispatches toggle action when shortcut is pressed
- Prevents default browser behavior

### Button Spacing
- Learning panel toggle button moved from `bottom: 80px` to `bottom: 150px`
- Composer is at bottom of chat container
- Ensures minimum 30px clearance between button and composer
- Button remains accessible and doesn't block send button

## Testing Considerations
- Verify sidebar collapses/expands smoothly
- Test keyboard shortcut works
- Verify branch indicators show correctly in collapsed state
- Test that learning panel button doesn't overlap send button on various screen sizes
- Ensure collapsed sidebar doesn't break responsive mobile layout

---

## Status Tracking

**Last Updated:** 2025-01-XX

**Current Phase:** All phases complete

**Overall Progress:** 5/5 phases complete

### Implementation Status

**Phase 1:** ✅ Complete
- Added `sidebar_collapsed` state to `UIState`
- Added `ToggleSidebar` action
- Handler implemented

**Phase 2:** ✅ Complete
- Modified `BranchSidebar` component with collapsed/expanded views
- Added toggle button functionality
- Created collapsed indicator rendering

**Phase 3:** ✅ Complete
- Connected state to components
- Added keyboard shortcut (Ctrl/Cmd + B)
- Callback connected

**Phase 4:** ✅ Complete
- Added CSS for collapsed sidebar
- Implemented branch indicators
- Updated layout spacing

**Phase 5:** ✅ Complete
- Adjusted learning panel button position
- Ensured no overlap with send button

