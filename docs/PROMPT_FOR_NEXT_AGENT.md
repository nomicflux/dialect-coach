# Task: True Neon Rope Overhaul

## Status
The previous agent **FAILED** to deliver the "Neon Rope" aesthetic.
-   It looked like a blurry teal line.
-   It was confined in a box.
-   It was just a stroke.

## Your Objective
Create a **visibly physical, glowing, multi-colored NEON TUBE** that sits **OUTSIDE** the message box constraints.

## Requirements

### 1. Visuals: The "Rope"
-   **LOOK AT THE REFERENCE IMAGES** (Lumary Neon Rope).
-   **Core**: It needs a **solid, bright center** (the "tube" itself) that is clearly defined.
-   **Glow**: It needs a saturated colored glow *surrounding* the tube. (Hint: Use specific `drop-shadow` on the parent or a pseudo-element, not just blurring the stroke itself).
-   **Gradient**: It MUST be **Teal -> Purple -> Coral**. **DO NOT** let CSS `stroke` properties override your gradient.
-   **Thickness**: 6px-10px. It must look like a physical object.

### 2. Layout: "NO BOX"
-   **Constraint**: You are forbidden from thinking in terms of "buttons" or "containers".
-   **Implementation**: Use `position: absolute`, `z-index: 10`, and coordinate positioning to hang the rope off the message bubble.
-   **Mental Model**: Think of it as a physical wire soldered to the component, not an icon inside it.

### 3. "Not a Button"
-   Strip all button semantics visually. It is a **path**.
-   Make it a Div. DO NOT EVEN THINK OF IT AS A BUTTON.
-   When hovering, the **Tube itself** should light up/swell. The "container" should be invisible. The glow should match the existing chat box and dynamic island glows.

## Implementation Tips
-   **Gradient Fix**: Check `chat.css` and **remove** any `stroke: var(--teal)` rules that target the icon. They are killing the gradient.
-   **Glow Fix**: **USE THE VERIFIED TOKENS**. Do not invent new values.
    -   **Pattern**: `filter: drop-shadow(0 0 4px var(--aurora-1)) drop-shadow(0 0 8px var(--aurora-2));`
    -   This provides the exact Teal-to-Purple glow the user wants.
-   **Positioning**: There should be no clipping. Radically redesign the layout to avoid parent container issues. You can completely change the look and code structure to accomplish the goal.

## File Locations
-   `frontend/src/components/icons/aurora_branch.rs` (The SVG).
-   `frontend/styles/components/chat.css` (The style crimes).
-   `docs/POST_MORTEM_NEON_FAIL.md` (The post-mortem).
 
**DO NOT** edit the existing files expecting a different result.
**Action**:
1.  **DELETE `aurora_branch.rs` NOW**. Do not read it. Do not copy it. `rm frontend/src/components/icons/aurora_branch.rs`.
2.  **DELETE `.branch-button` CSS**. Go into `chat.css` and delete the block.
3.  **THEN** create your new component.

> [!CRITICAL]
> **User Definition**: "START FROM SCRATCH" LITERALLY MEANS "**DELETE THE CURRENT APPROACH AND REDESIGN ENTIRELY**".
> Do not try to save the old code. It is trash.
