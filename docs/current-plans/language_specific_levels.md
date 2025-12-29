# Language-Specific Proficiency Levels Implementation Plan

## Agreements Made

**Date: 2025-12-29**

User requirements (exact quotes):
- "Make language levels language specific (CEFR for everything except Japanese, JLPT for Japanese)"
- "Enum naming - should always be scoped (`CEFR::A1`, `JLPT:N5`), so short version should not be ambiguous"
- "Default level for Japanese - Yes, stay with N3"
- "C2 -> N1" for migration
- "Autoconvert to equivalent (which means that equivalence code should be made standard, to be used for creation & migration in common)"
- "No text updates needed" for help text

## Explicitly Rejected

- Single flat enum with prefixed variants (e.g., `CefrA1`, `JlptN5`)
- Help text updates for dropdowns
- Resetting level to default on language change (instead: auto-convert)

## Implementation Details

### Type System

**CefrLevel enum:**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CefrLevel {
    A1, A2,
    #[default]
    B1,
    B2, C1, C2,
}
```

**JlptLevel enum:**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum JlptLevel {
    N5, N4,
    #[default]
    N3,
    N2, N1,
}
```

**LanguageLevel wrapper:**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LanguageLevel {
    Cefr(CefrLevel),
    Jlpt(JlptLevel),
}
```

### Equivalence Mapping (CEFR ↔ JLPT)

| CEFR | JLPT | Notes |
|------|------|-------|
| A1 | N5 | Beginner |
| A2 | N4 | Elementary |
| B1 | N3 | Intermediate (bridge level) |
| B2 | N2 | Upper Intermediate |
| C1 | N1 | Advanced |
| C2 | N1 | No JLPT equivalent, maps to highest |

### JLPT Instruction Text (for agent prompts)

Based on official JLPT level descriptions from [jlpt.jp](https://www.jlpt.jp/e/about/levelsummary.html):

- **N5**: Basic hiragana, katakana, ~100 kanji. Very simple sentences. Slow speech. Basic daily expressions.
- **N4**: Basic vocabulary, ~300 kanji. Simple daily topics. Speak slowly and clearly.
- **N3**: Everyday Japanese at near-natural speed. Can use context for slightly difficult expressions.
- **N2**: Varied vocabulary including abstract concepts. Natural speed. Newspaper articles accessible.
- **N1**: Full sophistication. Editorials, critiques, academic materials. Native-level complexity.

### Serialization Format Change

```json
// Old format
{ "dialect": "japanese_tokyo", "level": "B1" }

// New format
{ "dialect": "japanese_tokyo", "level": { "Jlpt": "N3" } }
```

### Migration Logic

1. Detect old format (string values like `"A1"`, `"B2"`)
2. Parse as CefrLevel
3. For each dialect_level entry:
   - Get dialect's language
   - If Japanese: convert via `cefr.to_jlpt()` → wrap as `Jlpt(jlpt_level)`
   - If not Japanese: wrap as `Cefr(cefr_level)`

---

## Phase 1: Core Type System + Migration + Settings UI

### Code Style Checklist
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No dead code
- [ ] Tests for new code paths

### Subagent: modular-builder

### Overview

This phase establishes the complete type system, adds migration support, updates the backend with JLPT instructions, and updates the Settings UI to show language-appropriate level options. After this phase, the app is fully functional with correct level options shown per language.

### Files to Create

*None*

### Files to Update

**shared/src/models/user_state.rs** (lines 21-66, 251-273)

Replace `LanguageLevel` enum with three new types:

```rust
// New CefrLevel enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CefrLevel {
    A1,
    A2,
    #[default]
    B1,
    B2,
    C1,
    C2,
}

impl CefrLevel {
    pub fn name(&self) -> &'static str { /* A1 - Beginner, etc */ }
    pub fn id(&self) -> &'static str { /* cefr_a1, etc */ }
    pub fn from_id(id: &str) -> Option<Self> { /* parse */ }
    pub fn all() -> Vec<Self> { vec![A1, A2, B1, B2, C1, C2] }
    pub fn to_jlpt(&self) -> JlptLevel {
        match self {
            A1 => JlptLevel::N5,
            A2 => JlptLevel::N4,
            B1 => JlptLevel::N3,
            B2 => JlptLevel::N2,
            C1 | C2 => JlptLevel::N1,
        }
    }
}

// New JlptLevel enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum JlptLevel {
    N5,
    N4,
    #[default]
    N3,
    N2,
    N1,
}

impl JlptLevel {
    pub fn name(&self) -> &'static str { /* N5 - Beginner, etc */ }
    pub fn id(&self) -> &'static str { /* jlpt_n5, etc */ }
    pub fn from_id(id: &str) -> Option<Self> { /* parse */ }
    pub fn all() -> Vec<Self> { vec![N5, N4, N3, N2, N1] }
    pub fn to_cefr(&self) -> CefrLevel {
        match self {
            N5 => CefrLevel::A1,
            N4 => CefrLevel::A2,
            N3 => CefrLevel::B1,
            N2 => CefrLevel::B2,
            N1 => CefrLevel::C1,
        }
    }
}

// LanguageLevel wrapper (replaces old enum)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LanguageLevel {
    Cefr(CefrLevel),
    Jlpt(JlptLevel),
}

impl Default for LanguageLevel {
    fn default() -> Self { LanguageLevel::Cefr(CefrLevel::B1) }
}

impl LanguageLevel {
    pub fn name(&self) -> &'static str {
        match self {
            Cefr(c) => c.name(),
            Jlpt(j) => j.name(),
        }
    }

    pub fn id(&self) -> &'static str {
        match self {
            Cefr(c) => c.id(),
            Jlpt(j) => j.id(),
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        CefrLevel::from_id(id).map(Cefr)
            .or_else(|| JlptLevel::from_id(id).map(Jlpt))
    }

    pub fn for_language(lang: Language) -> Vec<Self> {
        match lang {
            Language::Japanese => JlptLevel::all().into_iter().map(Jlpt).collect(),
            _ => CefrLevel::all().into_iter().map(Cefr).collect(),
        }
    }

    pub fn default_for_language(lang: Language) -> Self {
        match lang {
            Language::Japanese => Jlpt(JlptLevel::N3),
            _ => Cefr(CefrLevel::B1),
        }
    }
}
```

Update `with_initial_settings()` (line ~155-165) to use `LanguageLevel::default_for_language()` when no settings provided.

Update tests (lines 905-958) for new type structure.

**shared/src/models/mod.rs**

Add exports for `CefrLevel`, `JlptLevel` (LanguageLevel already exported).

**shared/src/lib.rs**

Add re-exports for `CefrLevel`, `JlptLevel`.

**shared/src/models/versioning.rs**

Add new version and migration:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum UserStateVersion {
    V1,
    V2ScoredItems,
    #[default]
    V3LanguageLevels,  // NEW
}

pub const CURRENT_USER_STATE_VERSION: UserStateVersion = UserStateVersion::V3LanguageLevels;

// Add to USER_STATE_MIGRATIONS array:
UserStateMigrationStep {
    from: UserStateVersion::V2ScoredItems,
    to: UserStateVersion::V3LanguageLevels,
    forward: v2_to_v3_forward,
    backward: v2_to_v3_backward,
},
```

Migration functions:
- `v2_to_v3_forward`: Convert `dialect_levels[].level` from string (`"B1"`) to object (`{"Cefr": "B1"}` or `{"Jlpt": "N3"}` based on dialect's language)
- `v2_to_v3_backward`: Convert object format back to string (extract inner value)

Update `test_user_state_structure_snapshot` expected fields (structure unchanged, but need to verify serialization format).

**backend/src/agent_service/response/teaching.rs** (lines 37-74)

Add JLPT level constants:

```rust
const JLPT_N5: &str = "LANGUAGE LEVEL N5 (Beginner): Use only hiragana, katakana, and basic kanji (~100 characters). Very simple sentences with basic vocabulary. Speak slowly and clearly. Limited to basic daily expressions and greetings.";

const JLPT_N4: &str = "LANGUAGE LEVEL N4 (Elementary): Use basic vocabulary and kanji (~300 characters). Simple sentences about familiar daily topics. Speak slowly. Basic past and future tenses okay.";

const JLPT_N3: &str = "LANGUAGE LEVEL N3 (Intermediate): Everyday Japanese at near-natural speed. Can use context to understand slightly difficult expressions. Standard vocabulary and grammar for daily situations.";

const JLPT_N2: &str = "LANGUAGE LEVEL N2 (Upper Intermediate): Varied vocabulary including abstract concepts. Can follow narratives and understand writer/speaker intent. Natural speech speed. Newspaper articles and general topics accessible.";

const JLPT_N1: &str = "LANGUAGE LEVEL N1 (Advanced): Full sophistication appropriate. Can handle editorials, critiques, and academic materials. Native-level complexity, nuance, and cultural references are appropriate.";
```

Update `language_level_instruction()`:

```rust
pub(crate) fn language_level_instruction(level: LanguageLevel) -> &'static str {
    match level {
        LanguageLevel::Cefr(CefrLevel::A1) => LEVEL_A1,
        LanguageLevel::Cefr(CefrLevel::A2) => LEVEL_A2,
        LanguageLevel::Cefr(CefrLevel::B1) => LEVEL_B1,
        LanguageLevel::Cefr(CefrLevel::B2) => LEVEL_B2,
        LanguageLevel::Cefr(CefrLevel::C1) => LEVEL_C1,
        LanguageLevel::Cefr(CefrLevel::C2) => LEVEL_C2,
        LanguageLevel::Jlpt(JlptLevel::N5) => JLPT_N5,
        LanguageLevel::Jlpt(JlptLevel::N4) => JLPT_N4,
        LanguageLevel::Jlpt(JlptLevel::N3) => JLPT_N3,
        LanguageLevel::Jlpt(JlptLevel::N2) => JLPT_N2,
        LanguageLevel::Jlpt(JlptLevel::N1) => JLPT_N1,
    }
}
```

Update test to cover all 11 levels.

**frontend/src/components/utility_sidebar/settings.rs** (lines 148-161)

Replace hardcoded CEFR options with dynamic rendering:

```rust
<div class="panel-field">
    <label for="language-level-select">{"Your Level"}</label>
    <select id="language-level-select" onchange={on_language_level_change(user.clone(), dispatch.clone())}>
        {
            LanguageLevel::for_language(us.selected_language)
                .into_iter()
                .map(|level| {
                    let is_selected = us.current_language_level() == level;
                    html! {
                        <option value={level.id()} selected={is_selected}>{level.name()}</option>
                    }
                })
                .collect::<Html>()
        }
    </select>
</div>
```

**frontend/src/app/user_state/callbacks.rs** (lines 106-128)

Update `on_language_level_change` to use `LanguageLevel::from_id()`:

```rust
pub fn on_language_level_change(
    user: Rc<UserState>,
    dispatch: Callback<UserDomainAction>,
) -> Callback<Event> {
    Callback::from(move |e: Event| {
        if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
            if let Some(level) = LanguageLevel::from_id(&select.value()) {
                let dialect = user.selected_dialect;
                dispatch.emit(UserDomainAction::Settings(SettingsAction::UpdateLevel(
                    dialect, level,
                )));
            }
        }
    })
}
```

### Deliverables

- `CefrLevel` and `JlptLevel` enums with full method suite
- `LanguageLevel` wrapper enum with `for_language()`, `default_for_language()`
- Migration V2→V3 that converts old string levels to new scoped format
- Backend JLPT instruction text for all 5 JLPT levels
- Settings dropdown dynamically shows correct options per language
- All tests pass, no dead code

### Phase End Checklist

- [ ] Run `cargo test` in workspace root - 100% pass required
- [ ] Run `cargo clippy` - fix ALL errors and warnings, remove any dead code
- [ ] Run `cd frontend && trunk build` - must compile
- [ ] Update Implementation Status table below
- [ ] `git add -A && git commit -m "Phase 1 (Core type system + migration + settings UI) complete"`
- [ ] STOP and wait for explicit user approval before Phase 2

---

## Phase 2: User Creation + Auto-Conversion on Language Change

### Code Style Checklist
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No dead code
- [ ] Tests for new code paths

### Subagent: kiss-code-generator

### Overview

This phase adds the `convert_for_language()` method and updates User Creation to show dynamic level options. It also adds auto-conversion when language changes (both in settings and user creation).

### Files to Update

**shared/src/models/user_state.rs**

Add `convert_for_language()` to `LanguageLevel`:

```rust
impl LanguageLevel {
    // ... existing methods ...

    /// Convert this level to the appropriate type for the given language.
    /// If already the correct type, returns self unchanged.
    pub fn convert_for_language(self, lang: Language) -> Self {
        match (self, lang) {
            // Japanese needs JLPT - convert CEFR to JLPT
            (LanguageLevel::Cefr(c), Language::Japanese) => LanguageLevel::Jlpt(c.to_jlpt()),
            // Non-Japanese needs CEFR - convert JLPT to CEFR
            (LanguageLevel::Jlpt(j), lang) if lang != Language::Japanese => LanguageLevel::Cefr(j.to_cefr()),
            // Already correct type
            _ => self,
        }
    }
}
```

Add test for `convert_for_language()`.

**frontend/src/components/user_creation.rs** (lines 37-40, 105-127, 371-400)

Update state initialization (line 39):
```rust
let selected_level = use_state(|| LanguageLevel::default());
```

Update `on_language_change` callback (lines 105-127) to auto-convert level:
```rust
let on_language_change = {
    let selected_language = selected_language.clone();
    let selected_dialect = selected_dialect.clone();
    let selected_level = selected_level.clone();
    Callback::from(move |e: Event| {
        if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
            let val = select.value();
            let lang = match val.as_str() {
                "Spanish" => Some(Language::Spanish),
                "Arabic" => Some(Language::Arabic),
                "French" => Some(Language::French),
                "English" => Some(Language::English),
                "Japanese" => Some(Language::Japanese),
                _ => None,
            };
            if let Some(new_lang) = lang {
                selected_language.set(Some(new_lang));
                // Auto-convert level for new language
                let current_level = *selected_level;
                selected_level.set(current_level.convert_for_language(new_lang));
            } else {
                selected_language.set(None);
            }
            selected_dialect.set(None);
        }
    })
};
```

Replace hardcoded level dropdown (lines 371-400) with dynamic:
```rust
<div class="form-group">
    <div class="select-wrapper">
        <select
            class="select-field"
            onchange={
                let selected_level = selected_level.clone();
                Callback::from(move |e: Event| {
                    if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
                        if let Some(level) = LanguageLevel::from_id(&select.value()) {
                            selected_level.set(level);
                        }
                    }
                })
            }
            disabled={selected_language.is_none()}
        >
            {
                if let Some(lang) = *selected_language {
                    LanguageLevel::for_language(lang)
                        .into_iter()
                        .map(|level| {
                            let is_selected = *selected_level == level;
                            html! {
                                <option value={level.id()} selected={is_selected}>{level.name()}</option>
                            }
                        })
                        .collect::<Html>()
                } else {
                    // Show placeholder when no language selected
                    html! { <option value="" disabled=true selected=true>{"Select a language first"}</option> }
                }
            }
        </select>
    </div>
</div>
```

**frontend/src/app/app_state/user/reducer.rs**

Update `SettingsAction::ChangeLanguage` handler to auto-convert the user's level:

Find the `ChangeLanguage(lang)` match arm and add level conversion:
```rust
ChangeLanguage(lang) => {
    // ... existing dialect selection logic ...

    // Auto-convert level for new language
    let current_level = next.current_language_level();
    let converted = current_level.convert_for_language(lang);
    if current_level != converted {
        next.set_level_for_dialect(next.selected_dialect, converted);
    }
}
```

### Deliverables

- `convert_for_language()` method on `LanguageLevel`
- User Creation shows dynamic level options based on selected language
- User Creation auto-converts level when language changes
- Settings auto-converts level when language changes
- Seamless UX when switching between Japanese and other languages

### Phase End Checklist

- [ ] Run `cargo test` in workspace root - 100% pass required
- [ ] Run `cargo clippy` - fix ALL errors and warnings, remove any dead code
- [ ] Run `cd frontend && trunk build` - must compile
- [ ] Update Implementation Status table below
- [ ] `git add -A && git commit -m "Phase 2 (User creation + auto-conversion) complete"`
- [ ] STOP and wait for explicit user approval

---

## Implementation Status

| Phase | Description | Status |
|-------|-------------|--------|
| 1 | Core type system + migration + settings UI | **Complete** |
| 2 | User creation + auto-conversion on language change | **Complete** |

## Phase 2 Completion Details (2025-12-29)

**Implementation Summary:**
- Added `convert_for_language()` method to `LanguageLevel` (4 lines)
- Auto-converts proficiency level when language changes in user creation
- User creation level dropdown now dynamically renders CEFR or JLPT options
- Settings reducer auto-converts level on language change

**Code Changes:**
1. `shared/src/models/user_state.rs`: Added `convert_for_language()` method + 4 comprehensive tests
2. `frontend/src/components/user_creation.rs`:
   - Updated `on_language_change` to auto-convert level
   - Replaced hardcoded CEFR dropdown with dynamic rendering
3. `frontend/src/app/app_state/user/reducer.rs`: Added level conversion in `ChangeLanguage` handler

**Test Results:**
- All 258 tests pass
- 4 new conversion tests verify both directions (CEFR↔JLPT)
- No clippy warnings
- Frontend builds successfully with trunk

**User Experience:**
- User Creation: Select Spanish → B2 level → Switch to Japanese → Level auto-converts to N2
- User Creation: Level dropdown disabled until language selected, shows appropriate options
- Settings: Change language → level auto-converts seamlessly
- No manual re-selection needed when switching between Japanese and other languages
