# Language-Specific Options - Initial Specification

## User Requirement

**Date: 2025-11-23**

User wants to add language-specific display and formatting options:
- "Different languages will need different options in 'Practice Settings'"
- "These will be persisted with other settings"
- "These need to be sent to the Response, Learning, and Analysis agents"
- First set: Arabic script options, Japanese script options

## Requirements Summary

### Scope
- **Per-language family** (not per-dialect)
  - All Arabic dialects share Arabic script options
  - All Japanese dialects share Japanese script options

### Initial Language Options

#### Arabic Script Options
1. **Naskh** (default) - Traditional script, use Naskh font family
2. **Ruq'a** - Ruq'ah script, use Ruq'a font family (currently set up)
3. **Latin** - Romanization
   - Agents MUST present romanized Arabic text
   - Romanization rules differ by dialect (agent must handle)
   - Option availability is language-based, not dialect-based

#### Japanese Script Options
1. **Romaji** - Full romanization by agent
2. **Only Kana** - Only hiragana and katakana, handled by agent
3. **Kanji w/ Ruby** (default) - Display kanji with furigana (frontend-only if possible)
4. **Kanji** - Standard kanji without annotations

### Persistence & Integration
- **Storage**: Part of `UserState` (persisted with other settings)
- **Agent Integration**: Sent via `UserMessageWithContext` to all agents:
  - Response agent
  - Learning agent
  - Analysis agent
  - Simple agents (Translation, Explanation)
- **System Instructions**: Added to agent system preambles

### UI Requirements
- **Location**: `SettingsPanel` component
- **Display**: Only show options for the current active language
  - If current language has no options, show nothing
  - Displayed alongside existing settings (dialect, formality, teaching mode)
- **Controls**: Dropdown selectors for each applicable option

### Future Extensibility
- Design should make it easy to add new language-specific options
- No other languages planned currently

## Data Model Design

### Shared Types (`shared/src/models/language_options.rs`)

```rust
use serde::{Deserialize, Serialize};

/// Arabic script display options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArabicScript {
    Naskh,  // Traditional Naskh font
    Ruqa,   // Ruq'ah font (currently implemented)
    Latin,  // Romanization (agent-handled)
}

impl Default for ArabicScript {
    fn default() -> Self {
        Self::Naskh
    }
}

impl std::fmt::Display for ArabicScript {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Naskh => write!(f, "Naskh"),
            Self::Ruqa => write!(f, "Ruq'a"),
            Self::Latin => write!(f, "Latin (Romanized)"),
        }
    }
}

/// Japanese script display options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JapaneseScript {
    Romaji,        // Full romanization (agent-handled)
    OnlyKana,      // Hiragana/katakana only (agent-handled)
    KanjiWithRuby, // Kanji with furigana (frontend if possible)
    Kanji,         // Standard kanji
}

impl Default for JapaneseScript {
    fn default() -> Self {
        Self::KanjiWithRuby
    }
}

impl std::fmt::Display for JapaneseScript {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Romaji => write!(f, "Romaji"),
            Self::OnlyKana => write!(f, "Kana Only"),
            Self::KanjiWithRuby => write!(f, "Kanji with Furigana"),
            Self::Kanji => write!(f, "Kanji"),
        }
    }
}

/// Container for all language-specific options
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct LanguageOptions {
    pub arabic_script: Option<ArabicScript>,
    pub japanese_script: Option<JapaneseScript>,
}

impl LanguageOptions {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get options relevant to a specific dialect
    pub fn for_dialect(&self, dialect: Dialect) -> Option<LanguageOption> {
        match dialect.language_family() {
            LanguageFamily::Arabic => self.arabic_script.map(LanguageOption::Arabic),
            LanguageFamily::Japanese => self.japanese_script.map(LanguageOption::Japanese),
            _ => None,
        }
    }
}

/// Active language option for current context
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LanguageOption {
    Arabic(ArabicScript),
    Japanese(JapaneseScript),
}
```

### Language Family Classification

Need to add to `shared/src/models/dialect.rs` or create `shared/src/models/language.rs`:

```rust
pub enum LanguageFamily {
    Arabic,
    Japanese,
    Spanish,
    // ... others
}

impl Dialect {
    pub fn language_family(&self) -> LanguageFamily {
        match self {
            Dialect::EgyptianArabic
            | Dialect::LevantineArabic
            | Dialect::GulfArabic
            | Dialect::MaghrebiArabic
            | Dialect::ModernStandardArabic => LanguageFamily::Arabic,

            Dialect::Japanese => LanguageFamily::Japanese,

            Dialect::MexicanSpanish
            | Dialect::CastilianSpanish
            | Dialect::ArgentinianSpanish
            | Dialect::CaribbeanSpanish => LanguageFamily::Spanish,

            // ... others
        }
    }
}
```

### UserState Integration

Add to `shared/src/models/user_state.rs` (or wherever UserState is defined):

```rust
pub struct UserState {
    // ... existing fields ...
    pub language_options: LanguageOptions,
}
```

### UserMessageWithContext Integration

Update `shared/src/models/agent.rs`:

```rust
pub struct UserMessageWithContext {
    pub content: String,
    pub conversation_history: Vec<Message>,
    pub learning_items: Vec<LearningItem>,
    pub teaching_mode: TeachingMode,
    pub formality: Formality,
    pub language_option: Option<LanguageOption>,  // NEW
}
```

## Agent Integration

### System Preamble Updates

Each agent's system prompt needs conditional instructions based on `language_option`:

**Example for Response Agent:**
```
if language_option == Some(LanguageOption::Arabic(ArabicScript::Latin)):
    "You must respond using romanized Arabic script. Use the appropriate
     romanization system for the user's dialect (Egyptian, Levantine, etc.)"

if language_option == Some(LanguageOption::Japanese(JapaneseScript::OnlyKana)):
    "You must respond using only hiragana and katakana. Do not use kanji."

if language_option == Some(LanguageOption::Japanese(JapaneseScript::Romaji)):
    "You must respond using romanized Japanese (romaji)."
```

### Agents to Update
1. **Response Agent** (`backend/src/agent_service/response_agent.rs`)
2. **Learning Agent** (`backend/src/agent_service/learning_agent.rs`)
3. **Analysis Agent** (`backend/src/agent_service/analysis_agent.rs`)
4. **Translation Agent** (`backend/src/agent_service/translation_agent.rs`)
5. **Explanation Agent** (`backend/src/agent_service/explanation_agent.rs`)

All agents need:
- Accept `language_option: Option<LanguageOption>` in their request context
- Include conditional system instructions based on language_option
- Format responses according to script preference

## UI Implementation

### SettingsPanel Updates (`frontend/src/components/settings_panel.rs`)

Add new section after existing settings:

```rust
// Pseudo-code structure
if let Some(dialect) = active_branch_dialect {
    let language_family = dialect.language_family();

    match language_family {
        LanguageFamily::Arabic => {
            // Show Arabic script dropdown
            html! {
                <div class="setting-group">
                    <label>{"Script:"}</label>
                    <select onchange={on_arabic_script_change}>
                        <option selected={is_naskh}>{"Naskh"}</option>
                        <option selected={is_ruqa}>{"Ruq'a"}</option>
                        <option selected={is_latin}>{"Latin (Romanized)"}</option>
                    </select>
                </div>
            }
        }

        LanguageFamily::Japanese => {
            // Show Japanese script dropdown
            html! {
                <div class="setting-group">
                    <label>{"Script:"}</label>
                    <select onchange={on_japanese_script_change}>
                        <option selected={is_romaji}>{"Romaji"}</option>
                        <option selected={is_kana_only}>{"Kana Only"}</option>
                        <option selected={is_kanji_ruby}>{"Kanji with Furigana"}</option>
                        <option selected={is_kanji}>{"Kanji"}</option>
                    </select>
                </div>
            }
        }

        _ => {
            // No language-specific options for this language
            html! {}
        }
    }
}
```

### UserState Actions

Add to `frontend/src/app/app_state.rs`:

```rust
pub enum UserStateAction {
    // ... existing actions ...
    SetArabicScript(ArabicScript),
    SetJapaneseScript(JapaneseScript),
}
```

## CSS Considerations

### Arabic Font Families

Current setup (user mentioned Ruq'a is already implemented):
- **Naskh**: Need to add Naskh font-family CSS
- **Ruq'a**: Already exists
- **Latin**: Use standard Latin font (no special font needed)

### Japanese Ruby Text

For `KanjiWithRuby` option, use HTML `<ruby>` tags with `<rt>` for furigana:

```html
<ruby>
  漢字
  <rt>かんじ</rt>
</ruby>
```

CSS for ruby text:
```css
ruby {
  ruby-position: over;
}

rt {
  font-size: 0.5em;
}
```

**Question for implementation**: Can frontend parse agent responses and add ruby tags automatically, or must agents send pre-formatted ruby HTML?

**User's preference**: "If Kanji w/ ruby can be frontend-only, then do that"

**Decision needed**: Research if we can:
- Have agents send plain kanji
- Frontend detects kanji and adds furigana using a library (e.g., kuroshiro.js in WASM)
- OR: Agents must send ruby-tagged HTML

## Open Questions - RESOLVED

### 1. Japanese Ruby Text Implementation ✅

**Research findings:**
- **lindera-wasm** exists: Rust WASM library for Japanese morphological analysis
- Provides readings (yomi/furigana) for tokens in tokenization output
- Example output: `"関西国際空港,カンサイコクサイクウコウ,カンサイコクサイクーコー"`
- However: lindera only provides readings, NOT character-aligned furigana markup

**Options:**
- **Option A (Frontend with lindera-wasm)**: Use lindera-wasm to get token readings, then implement alignment logic + HTML ruby tags
  - Pros: Cleaner separation, agents don't handle HTML
  - Cons: Complex alignment algorithm needed, large WASM bundle (includes dictionaries)

- **Option B (Agent-generated ruby tags)**: Agents send pre-formatted ruby HTML
  - Pros: Agents have better NLP, can use Claude's Japanese knowledge
  - Cons: HTML in responses, agent complexity

**DECISION (User)**: "If Kanji w/ ruby can be frontend-only, then do that"

**Implementation Decision**: Use **Option B (Agent-generated)** because:
- Frontend-only requires complex character alignment algorithm
- lindera-wasm adds significant bundle size (Japanese dictionaries)
- Agents already have excellent Japanese NLP (Claude)
- Agent can generate proper `<ruby><rt>` tags directly
- Simpler overall architecture

### 2. Arabic Font Files ✅

**Current setup (User confirmed):**
- **Ruq'a**: Aref Ruqaa font family (already implemented)
- **Naskh**: Standard font families generally have Naskh (use system fonts)
- **Latin**: No special font needed (standard Latin fonts)

**Implementation:**
- Naskh: Use CSS `font-family: "Times New Roman", "Noto Naskh Arabic", serif;`
- Ruq'a: Keep existing `font-family: "Aref Ruqaa", cursive;`
- Latin: Use default app fonts

### 3. Romanization Standards ✅

**User decision**: "For now, leave this up to the agents (dialect romanization is not entirely standardized for Arabic)"

**Implementation:**
- No specific romanization system prescribed
- Agents use their knowledge of dialect-appropriate romanization
- System prompt: "Use appropriate romanization for [dialect]"

### 4. State Migration ✅

**User decision**: "No migration; I'll delete the DB and start fresh."

**Implementation:**
- No migration code needed
- Add `language_options: LanguageOptions::default()` to UserState
- Default values handle initialization

### 5. Testing
- Manual testing of agent outputs for each script option
- Verify CSS applies correctly for Arabic fonts
- Verify ruby tags render correctly for Japanese

## Implementation Phases (High-Level)

### Phase 1: Shared Models & State
- Create `language_options.rs` in shared
- Add `LanguageFamily` to `Dialect`
- Add `language_options` to `UserState`
- Add `language_option` to `UserMessageWithContext`
- Add `UserStateAction` variants for setting options

### Phase 2: UI Integration
- Update `SettingsPanel` to show language options
- Wire up callbacks to dispatch state actions
- Add CSS for Arabic fonts (if needed)
- Research/implement Japanese ruby text rendering

### Phase 3: Agent Integration
- Update all 5 agents to accept `language_option`
- Add conditional system instructions for each script type
- Test romanization output for Arabic
- Test kana-only output for Japanese

### Phase 4: Testing & Polish
- Test all combinations of dialect + language option
- Test persistence (save/load)
- Test agent responses match script preference
- Manual UI testing

## Success Criteria

- [x] User can select script option in SettingsPanel
- [x] Only current language's options are shown
- [x] Options persist across sessions (via UserState)
- [ ] Arabic Latin option produces romanized agent responses (requires manual testing)
- [ ] Japanese Romaji option produces romanized agent responses (requires manual testing)
- [ ] Japanese Only Kana option produces kana-only agent responses (requires manual testing)
- [ ] Japanese Kanji w/ Ruby displays furigana (agent-generated, requires manual testing)
- [x] Arabic Naskh uses Naskh font family
- [x] Arabic Ruq'a uses existing Ruq'a font
- [x] All 3 agents respect language options (Response, Learning, Analysis)
- [x] All tests pass (540/540 executable tests)
- [x] Clippy clean (zero new warnings)

## Notes

- User confirmed: "Design should make it easy to add new language-specific options"
- Extensibility pattern: Add new enum to `LanguageOptions`, add match arm in UI, add agent instructions
- No other languages planned currently

## Status

**Phase**: Specification review
**Next**: User review and approval of spec
