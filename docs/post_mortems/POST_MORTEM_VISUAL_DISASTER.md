# Post-Mortem: Visual Regression Disaster

## Incident Summary
The agent failed to deliver visual changes requested by the user, specifically regarding the "Branch from here" button. The user reported "LITERALLY NO VISUAL CHANGE" despite the agent marking the task as complete.

## Root Causes

### 1. Misidentification of Target Component
- **Failure**: The agent assumed the "branch icon" referred to the `BranchSwitcherPill` component (top of screen) because it contained the text "Switch Branch".
- **Reality**: The user was referring to the `render_branch_button` in `message_bubble.rs` (`title="Branch from here"`), which appears below messages.
- **Why it happened**: The agent did not grep for the specific text "Branch from here" provided in the failing screenshot/context, instead relying on a fuzzy search for "icons" which led to the wrong usage site.

### 2. Failure to Identify CSS Constraints
- **Failure**: The agent modified the SVG (`aurora_branch.rs`) to "glow", but failed to realize that `chat.css` wrapped the icon in a `<button>` with hardcoded borders, background colors, and padding.
- **Reality**: The CSS class `.branch-button` in `frontend/styles/components/chat.css` forces the button into a "teal rounded square" box, completely negating the intended "floating neon path" aesthetic.
- **Why it happened**: The agent did not verify the CSS rules applying to the specific component instance (`message_bubble.rs`), assuming component-level changes (`branch_switcher_pill.rs`) would apply globally or that the SVG change alone was sufficient.

### 3. Inadequate Verification
- **Failure**: The agent verified by running `cargo check`, which only proves code compilation.
- **Reality**: Visual regressions require visual verification (finding the styles) or strictly checking the CSS values against the visual description. The agent incorrectly assumed "Plan Approved" meant "Correct Identification of Files".
- **Why it happened**: The agent did not follow the trail of where the `AuroraBranchIcon` was actually used in the context of the user's specific complaint.

## Corrective Actions for Future Agents
1.  **Grep the Exact Text**: If the user provides screen text ("Branch from here"), grep for that *exact string* to find the component.
2.  **Check Container Styles**: Always check the parent container (e.g., `<button class="branch-button">`) styles in CSS, not just the inner SVG.
3.  **Respect "Box" Constraints**: If the user says "it is confined to a box", look for `border`, `background`, `padding`, and `width/height` constraints in CSS.
