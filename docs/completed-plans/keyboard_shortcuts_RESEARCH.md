# Keyboard Shortcuts Research - macOS Chrome

## Research Date: 2025-11-16

## Summary

**RESOLVED**: Implementation updated from Ctrl+Option to **Ctrl+Shift** to avoid VoiceOver accessibility conflicts.

**Previous issue**: Ctrl+Option combinations conflicted with ALL macOS VoiceOver shortcuts.

---

## Current Implementation Analysis

The current implementation (`frontend/src/keyboard_shortcuts.rs`) uses:
- **Modifier**: Ctrl+Option (Control+Alt in web terms)
- **Keys**: Letters (U, P, A, R, D, T, F, G, I) and brackets ([, ])
- **Event handling**: Uses `event.code()` (physical key) not `event.key()` (character)

### What's Good
1. Uses `event.code` instead of `event.key` - avoids special character issues from Option key
2. Doesn't conflict with Chrome's Command-based shortcuts (Cmd+T, Cmd+S, etc.)
3. Calls `event.prevent_default()` to stop browser behavior
4. Skips shortcuts when focused on text input (correct behavior)

### What's Bad
1. **ALL shortcuts conflict with VoiceOver** (macOS screen reader):
   - Ctrl+Option+U = VoiceOver Rotor
   - Ctrl+Option+P = Read paragraph
   - Ctrl+Option+A = Read all
   - Ctrl+Option+R = Read row header
   - Ctrl+Option+D = Go to Dock
   - Ctrl+Option+T = Text attributes
   - Ctrl+Option+F = Find text
   - Ctrl+Option+G = Find next searched text
   - Ctrl+Option+I = Item Chooser
   - Ctrl+Option+[ = Move to previous Window Spot
   - Ctrl+Option+] = Move to next Window Spot

This makes the app **unusable for VoiceOver users** when shortcuts are enabled.

---

## Research Findings

### Chrome macOS Shortcuts

Chrome uses **Command (⌘)** for shortcuts, not Ctrl:
- Cmd+T = New tab
- Cmd+W = Close tab
- Cmd+S = Save page
- Cmd+P = Print
- Cmd+D = Bookmark
- Cmd+R = Reload

**Ctrl-based shortcuts are mostly unused in Chrome on macOS**, making them technically safe from browser conflicts.

### macOS System Shortcuts

macOS reserves:
- **Command+Space** = Spotlight
- **Control+Space** = Input source switching
- **Control+Option+Space** = Previous/next input source
- **Control+Option+Command+8** = Invert colors
- **Control+Option+[letter]** = **VoiceOver commands** (CONFLICT!)

### VoiceOver (Critical Accessibility Issue)

VoiceOver uses **Control+Option** as its modifier key (called "VO key"). Every single Ctrl+Option+letter combination is reserved:
- Nearly the entire alphabet has a VoiceOver function
- Bracket keys have Window Spot navigation functions
- This is not configurable without disabling VoiceOver

### Option Key and Special Characters

On macOS, Option+letter produces special characters:
- Option+A = å
- Option+P = π
- Option+D = ∂
- Option+R = ®
- etc.

**However**, when Control is also pressed (Ctrl+Option), the special character is NOT inserted if `event.prevent_default()` is called. The current implementation does this correctly.

### Best Practices from Major Web Apps

**Gmail, GitHub, Slack, Notion** all use **single-letter shortcuts without modifiers**:
- `j`/`k` = Navigate up/down (Vim-style)
- `?` = Show keyboard shortcuts help
- `c` = Compose/Create
- `e` = Archive/Edit
- `g p` = Go to pull requests (sequence)
- `g i` = Go to issues (sequence)

These only activate when NOT focused on text input fields.

---

## Recommended Approaches (Ranked by Safety)

### Option 1: Single Letter Keys (RECOMMENDED)
**Best for accessibility and usability**

```
u = Toggle usage footer
p = Toggle practice settings
a = Toggle auto-speak
r = Replay last message
d = Cycle dialect
t = Cycle teaching mode
f = Cycle formality
g = Focus goal input
i = Focus chat input
[ = Toggle sidebar
] = Toggle learning panel
? = Show shortcuts help
```

**Pros**:
- No system conflicts
- No browser conflicts
- No VoiceOver conflicts (VoiceOver uses Ctrl+Option)
- Easiest to type
- Industry standard (Gmail, GitHub)
- Already disabled when typing in input fields (current implementation handles this)

**Cons**:
- Won't work when text input is focused (but this is correct behavior)
- Need to educate users (add `?` for help)

### Option 2: Sequence Keys (Like GitHub)
**Good alternative with more "namespacing"**

```
g u = Toggle usage footer
g p = Toggle practice settings
g a = Toggle auto-speak
g r = Replay last message
g d = Cycle dialect
g t = Cycle teaching mode
g f = Cycle formality
g g = Focus goal input
g i = Focus chat input
s [ = Toggle sidebar
s ] = Toggle learning panel
? = Show shortcuts help
```

**Pros**:
- Even less likely to conflict
- More keys available
- Clearer intent

**Cons**:
- Harder to type (two key presses)
- Requires state management for "pending" first key
- More complex implementation

### Option 3: Control+Shift (Safe Multi-Modifier)
**If you must use modifier keys**

```
Ctrl+Shift+U = Toggle usage footer
Ctrl+Shift+P = Toggle practice settings
...
```

**Pros**:
- Doesn't conflict with VoiceOver (which uses Ctrl+Option)
- Doesn't conflict with Chrome (which uses Command)
- Doesn't produce special characters

**Cons**:
- Ctrl+Shift+P might conflict with browser's command palette in future
- Harder to type than single letters
- Less intuitive

### Option 4: Keep Current (NOT RECOMMENDED)

**Only if accessibility is explicitly deprioritized**

Document that the app is incompatible with VoiceOver and provide a way to disable shortcuts for VoiceOver users.

---

## Implementation (COMPLETED)

**Using Ctrl+Shift**

### Why Ctrl+Shift

1. **No VoiceOver conflicts** - VoiceOver uses Ctrl+Option, not Ctrl+Shift
2. **No Chrome conflicts** - Chrome uses Command on macOS
3. **No macOS system conflicts** - System uses Command for most shortcuts
4. **Works in text inputs** - Unlike single-letter shortcuts (critical for chat app)
5. **No special characters** - Unlike Option key combinations

### Final Shortcuts

```
Ctrl+Shift+[ = Toggle sidebar
Ctrl+Shift+] = Toggle learning panel
Ctrl+Shift+U = Toggle usage footer
Ctrl+Shift+P = Toggle practice settings
Ctrl+Shift+A = Toggle auto-speak/TTS
Ctrl+Shift+R = Replay last message
Ctrl+Shift+D = Cycle dialect
Ctrl+Shift+T = Cycle teaching mode
Ctrl+Shift+F = Cycle formality
Ctrl+Shift+G = Focus goal input
Ctrl+Shift+I = Focus chat input
Shift+Enter = Send message (in chat input)
```

### Implementation

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct KeyBinding {
    pub code: String,
    pub ctrl: bool,
    pub shift: bool,
}

impl KeyBinding {
    pub fn with_ctrl_shift(code: &str) -> Self {
        Self {
            code: code.to_string(),
            ctrl: true,
            shift: true,
        }
    }
}

pub fn matches_binding(event: &KeyboardEvent, binding: &KeyBinding) -> bool {
    event.ctrl_key() == binding.ctrl
        && event.shift_key() == binding.shift
        && event.code() == binding.code
}
```

---

## Testing Plan

After implementation:

1. **VoiceOver test**: Enable VoiceOver (Cmd+F5), verify shortcuts don't interfere
2. **Text input test**: Verify shortcuts are ignored when typing in chat/goal input
3. **Browser test**: Verify no conflicts with Chrome shortcuts
4. **Functionality test**: Verify all actions work as expected

---

## References

- [Chrome Keyboard Shortcuts](https://support.google.com/chrome/answer/157179)
- [macOS Keyboard Shortcuts](https://support.apple.com/en-us/102650)
- [VoiceOver Keyboard Shortcuts](https://www.applevis.com/guides/complete-list-voiceover-keyboard-shortcuts-available-macos)
- [GitHub Keyboard Shortcuts](https://docs.github.com/en/get-started/using-github/keyboard-shortcuts)
- [Stack Overflow: Safe Web App Shortcuts](https://stackoverflow.com/questions/3329420/what-are-cross-browser-and-cross-os-safe-keyboard-shortcuts-usable-for-web-appli)
- [MDN: KeyboardEvent](https://developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent)
