# Remove Interleaved Mode, Add Universal Translation Agent

## Summary

Remove `TeachingMode::Interleaved` as a user-facing mode. Make translation functionality universal across ALL non-debug teaching modes by creating a separate `TranslationAgent` that runs alongside mode-specific learning agents.

## Architecture Change

**Before:** One learning agent per mode, each produces one item type
**After:** Multiple learning agents called per response:

| Mode | Agents Called |
|------|--------------|
| Immersive | Translation only |
| Corrective | Mistakes + Translation |
| Explanatory | Explained + Translation |
| StoryTeller | Exploratory + Translation |
| Debug | None |

## Agreements Made

- Date: 2025-12-23
- User specified: "Remove interleaved. Interleaved functionality should be a standard (non-debug) element"
- User specified: "make MULTIPLE calls to learning agents (INCLUDING for immersive, but NOT for debug)"
- User specified: "Make a 'translation' agent type if needed to preserve the functionality"
- User specified: "Add max_translated for all modes"
- User specified: "Generic instruction for all dialects. LLM needs to use judgement" for foreign imports warning
- User specified: "if it was interleaved in saved user state, move to immersive"

## Explicitly Rejected

- Combining multiple learning item types into a single agent call
- Making translation optional based on teaching mode (it's always on for non-debug)
- Dialect-specific foreign import word lists (use generic LLM judgment instead)
- behavior_mode() or any other "hack" abstraction - use direct pattern matching
- Keeping Interleaved as a hidden alias - it must be fully removed

---

## Phase 1: Add TranslationAgent + Integrate

### Code Style Checklist
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No dead code
- [ ] Tests for new functions

### Subagent: modular-builder

### Files to Create

1. **backend/src/agent_service/translation.rs** (NEW)

### Files to Modify

2. **backend/src/agent_service/response/generation.rs**
   - Import TranslationAgent
   - Call TranslationAgent for all non-debug modes in `attach_learning_items()`
   - Merge translation results with learning results

### Precise Implementation

**translation.rs:**
```rust
use anyhow::Result;
use dialect_coach_shared::{AgentUsage, Dialect, Formality, LanguageOption, Translated};
use serde::Deserialize;
use std::sync::Arc;

use super::language_instructions::build_language_instruction;
use super::provider::{CompletionAgent, CompletionRequest, ANTHROPIC_PROVIDER};
use super::retry::retry_completion_call;
use super::util::{JSON_OUTPUT_INSTRUCTION, create_prefilled_assistant_message, normalize_json_response};

#[derive(Debug, Clone)]
pub struct TranslationAgentParams<'a> {
    pub user_message: &'a str,
    pub dialect: Dialect,
    pub formality: Formality,
    pub past_translated: &'a [Translated],
    pub language_option: &'a Option<LanguageOption>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TranslationAgentOutput {
    pub translated: Vec<Translated>,
}

impl TranslationAgentOutput {
    pub fn empty() -> Self {
        Self { translated: Vec::new() }
    }
}

fn calculate_max_translated(past_count: usize) -> u8 {
    if past_count == 0 { 2 } else { 1 }
}

fn should_skip_translation(past_translated: &[Translated]) -> bool {
    past_translated.len() >= 20
}

#[derive(Debug, Deserialize)]
struct RawTranslationOutput {
    #[serde(default)]
    translated: Vec<Translated>,
}

pub struct TranslationAgent {
    agent: Arc<dyn CompletionAgent>,
}

impl TranslationAgent {
    pub fn new(agent: Arc<dyn CompletionAgent>) -> Self {
        Self { agent }
    }

    pub async fn generate_translations(
        &self,
        params: &TranslationAgentParams<'_>,
    ) -> (Result<TranslationAgentOutput>, Vec<AgentUsage>) {
        if should_skip_translation(params.past_translated) {
            return (Ok(TranslationAgentOutput::empty()), Vec::new());
        }

        let max_translated = calculate_max_translated(params.past_translated.len());
        let system_content = build_translation_system_content(params, max_translated);
        let prompt = build_translation_prompt(params);

        let mut history = Vec::new();
        if self.agent.provider() == ANTHROPIC_PROVIDER {
            history.push(create_prefilled_assistant_message());
        }

        let request = CompletionRequest {
            preamble: &system_content,
            prompt: &prompt,
            history: &history,
            max_tokens: 256,
            temperature: 0.0,
        };

        let (result, usage) = retry_completion_call(self.agent.as_ref(), &request, 3).await;
        match result {
            Ok(response) => match try_parse_translation_output(&response) {
                Ok(output) => {
                    log_translation_success(&output);
                    (Ok(output), usage)
                }
                Err(e) => (Err(e), usage),
            },
            Err(e) => (Err(e), usage),
        }
    }
}

fn build_translation_system_content(params: &TranslationAgentParams<'_>, max_items: u8) -> String {
    let language_instr = build_language_instruction(params.language_option);
    let lang_section = if !language_instr.is_empty() {
        format!("\n\nLANGUAGE INSTRUCTION: {}\n", language_instr)
    } else {
        String::new()
    };

    format!(
        r#"# TRANSLATION AGENT ROLE
You identify English words/phrases in the user's message that should be translated to the target dialect.

# CONTEXT
Target dialect: {} at {} formality.{}

# CRITICAL: FOREIGN IMPORTS WARNING
Do NOT translate English words that are commonly used as loanwords in this dialect.
Examples of words to KEEP in English:
- Technical terms (computer, internet, software, email)
- Brand names and proper nouns
- Words that have been adopted into the dialect with no native equivalent
- Words the dialect commonly uses in English form

Only translate when the user is clearly code-switching back to English, NOT using established loanwords.

# RULES
- Log translations only, not conversational responses
- Do not duplicate previously logged translations
- Maximum {} item(s)

# OUTPUT FORMAT
{}
{{
  "translated": [{{"translated_word": "<English word>", "translated_to": "<dialect translation>"}}]
}}
Return {{"translated": []}} when nothing requires translation."#,
        params.dialect.name(),
        params.formality.name(),
        lang_section,
        max_items,
        JSON_OUTPUT_INSTRUCTION,
    )
}

fn build_translation_prompt(params: &TranslationAgentParams<'_>) -> String {
    let past_section = if params.past_translated.is_empty() {
        "No previous translations.".to_string()
    } else {
        params.past_translated
            .iter()
            .map(|t| format!("{} -> {}", t.translated_word, t.translated_to))
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        "USER MESSAGE:\n{}\n\nPREVIOUS TRANSLATIONS:\n{}",
        params.user_message,
        past_section
    )
}

fn try_parse_translation_output(response: &str) -> Result<TranslationAgentOutput> {
    let normalized = normalize_json_response(response);
    let parsed: RawTranslationOutput = serde_json::from_str(&normalized)?;
    Ok(TranslationAgentOutput { translated: parsed.translated })
}

fn log_translation_success(output: &TranslationAgentOutput) {
    tracing::info!("Generated {} translation items", output.translated.len());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_skip_translation_under_limit() {
        assert!(!should_skip_translation(&[]));
    }

    #[test]
    fn test_should_skip_translation_at_limit() {
        let items: Vec<Translated> = (0..20)
            .map(|i| Translated::new(format!("word{}", i), format!("trans{}", i), None))
            .collect();
        assert!(should_skip_translation(&items));
    }

    #[test]
    fn test_calculate_max_translated_empty() {
        assert_eq!(calculate_max_translated(0), 2);
    }

    #[test]
    fn test_calculate_max_translated_has_items() {
        assert_eq!(calculate_max_translated(5), 1);
    }

    #[test]
    fn test_translation_output_empty() {
        let output = TranslationAgentOutput::empty();
        assert!(output.translated.is_empty());
    }
}
```

**generation.rs changes:**

Add imports:
```rust
use crate::agent_service::translation::{TranslationAgent, TranslationAgentParams, TranslationAgentOutput};
```

Update `attach_learning_items()`:
```rust
pub(super) async fn attach_learning_items(
    &self,
    params: &GenerateResponseParams<'_>,
    parsed_response: dialect_coach_shared::AgentResponse,
) -> Result<(dialect_coach_shared::AgentResponse, Vec<AgentUsage>)> {
    let assistant_response = parsed_response.response.clone();

    // Call mode-specific learning agent
    let learning_agent = LearningAgent::new(self.learning_agent.clone());
    let learning_params = build_learning_params(params, &assistant_response);
    let (learning_result, learning_usage) = learning_agent
        .generate_learning_items(&learning_params)
        .await;

    // Call translation agent for all non-debug modes
    let (translation_result, translation_usage) = if params.teaching_mode != TeachingMode::Debug {
        let translation_agent = TranslationAgent::new(self.learning_agent.clone());
        let translation_params = TranslationAgentParams {
            user_message: params.user_message,
            dialect: params.dialect.dialect,
            formality: params.formality,
            past_translated: params.past_translated,
            language_option: params.language_option,
        };
        translation_agent.generate_translations(&translation_params).await
    } else {
        (Ok(TranslationAgentOutput::empty()), Vec::new())
    };

    // Merge results
    let mut all_usage = learning_usage;
    all_usage.extend(translation_usage);

    match (learning_result, translation_result) {
        (Ok(learning_output), Ok(translation_output)) => {
            let merged = merge_learning_outputs(learning_output, translation_output);
            Ok((apply_learning_output(parsed_response, merged), all_usage))
        }
        (Err(e), _) | (_, Err(e)) => Err(e),
    }
}

fn merge_learning_outputs(
    learning: LearningAgentOutput,
    translation: TranslationAgentOutput,
) -> LearningAgentOutput {
    LearningAgentOutput {
        mistakes: learning.mistakes,
        explained: learning.explained,
        translated: translation.translated,
        exploratory: learning.exploratory,
    }
}
```

### Deliverables
- [ ] TranslationAgent struct exists with `generate_translations()` method
- [ ] Foreign imports warning included in system prompt
- [ ] TranslationAgent called for all non-debug modes (including Interleaved)
- [ ] Results merged correctly
- [ ] All tests pass

### Phase End Verification

**MANDATORY - Do not skip any step:**
1. Run FULL test suite: `cargo test --all`
2. Run clippy on ALL crates: `cargo clippy --all -- -D warnings`
3. Fix ALL failures - there is no "out of scope". If you see a failure, fix it.
4. Remove ALL dead code. No excuses. You see dead code, remove it.
5. Update the Implementation Status table below with results.
6. Only after 100% pass: `git add -A && git commit -m "Phase 1 (Add TranslationAgent + integrate) complete"`

---

## Phase 2: Update LearningAgent - Group Interleaved with Immersive

### Code Style Checklist
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No dead code
- [ ] Tests for new functions

### Subagent: kiss-code-generator

### Files to Modify

1. **backend/src/agent_service/learning.rs**

### Precise Changes

**Update skip_mode() to include Interleaved:**
```rust
fn skip_mode(mode: &TeachingMode) -> bool {
    matches!(mode, TeachingMode::Immersive | TeachingMode::Interleaved | TeachingMode::Debug)
}
```

**Update should_skip_learning_call() - remove Interleaved exception:**
```rust
fn should_skip_learning_call(params: &LearningAgentParams<'_>) -> bool {
    let total_items = params.past_mistakes.len()
        + params.past_explained.len()
        + params.past_translated.len()
        + params.past_exploratory.len();

    if total_items < 10 {
        return false;
    }

    // Skip for modes that generate limited items when total >= 10
    matches!(
        params.teaching_mode,
        TeachingMode::Corrective | TeachingMode::Explanatory | TeachingMode::StoryTeller
    )
}
```

**Remove Interleaved branch from learning_mode_context():**
```rust
// DELETE this entire match arm:
TeachingMode::Interleaved => {
    format!(
        "# WHAT TO LOG\n\
        - English words/phrases from the user's message that needed translation\n\
        - Translations must be in {} at {} formality\n\n",
        dialect.name(),
        formality.name()
    )
}

// Change the Immersive/Debug arm to include Interleaved:
TeachingMode::Immersive | TeachingMode::Interleaved | TeachingMode::Debug => String::new(),
```

**Remove Interleaved branch from learning_output_format_spec():**
```rust
// DELETE this entire match arm:
TeachingMode::Interleaved => r#"{
  "translated": [{"translated_word": "<source word>", "translated_to": "<dialect translation>"}]
}
- Return {"translated": []} when nothing required translating"#
    .to_string(),

// Change the Immersive/Debug arm to include Interleaved:
TeachingMode::Immersive | TeachingMode::Interleaved | TeachingMode::Debug => r#"{}
No learning items for this mode."#
    .to_string(),
```

**Remove Interleaved from build_learning_prompt():**
```rust
// BEFORE:
let (user_section, asst_section) = match params.teaching_mode {
    TeachingMode::Corrective | TeachingMode::Interleaved => (
        format!("LATEST USER MESSAGE:\n{}\n\n", params.user_message),
        String::new(),
    ),

// AFTER:
let (user_section, asst_section) = match params.teaching_mode {
    TeachingMode::Corrective => (
        format!("LATEST USER MESSAGE:\n{}\n\n", params.user_message),
        String::new(),
    ),
```

**Delete Interleaved test:**
```rust
// DELETE entire test function:
#[test]
fn test_should_not_skip_learning_call_at_10_interleaved() {
    // ... entire test body
}
```

### Deliverables
- [ ] skip_mode() groups Interleaved with Immersive
- [ ] learning_mode_context() groups Interleaved with Immersive
- [ ] learning_output_format_spec() groups Interleaved with Immersive
- [ ] build_learning_prompt() does not have Interleaved-specific handling
- [ ] Interleaved test deleted
- [ ] All tests pass

### Phase End Verification

**MANDATORY - Do not skip any step:**
1. Run FULL test suite: `cargo test --all`
2. Run clippy on ALL crates: `cargo clippy --all -- -D warnings`
3. Fix ALL failures - there is no "out of scope". If you see a failure, fix it.
4. Remove ALL dead code. No excuses. You see dead code, remove it.
5. Update the Implementation Status table below with results.
6. Only after 100% pass: `git add -A && git commit -m "Phase 2 (Update LearningAgent - group Interleaved with Immersive) complete"`

---

## Phase 3: Update Response Config - Group Interleaved with Immersive

### Code Style Checklist
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No dead code
- [ ] Tests for new functions

### Subagent: kiss-code-generator

### Files to Modify

1. **backend/src/agent_service/response/teaching.rs**
2. **backend/src/agent_service/response/config.rs**

### Precise Changes

**teaching.rs:**

DELETE the INTERLEAVED_DESC constant entirely:
```rust
// DELETE these lines:
const INTERLEAVED_DESC: &str = r#"3. INTERLEAVED MODE: User mixes target language with source language.
Respond naturally as a chat conversation partner in 1-2 sentences.
If the user made errors in their message, weave the correct forms into your response conversationally without mentioning them. Maximum two corrections per response.
If they used English words, incorporate the dialect translation of one phrase naturally as you continue the conversation.
Never explain corrections. Never say "you meant" or "you're saying". Just chat naturally using correct forms."#;
```

Update mode_description() to group Interleaved with Immersive:
```rust
fn mode_description(mode: &TeachingMode) -> &'static str {
    match mode {
        TeachingMode::Immersive | TeachingMode::Interleaved => IMMERSIVE_DESC,
        TeachingMode::Corrective => CORRECTIVE_DESC,
        TeachingMode::Explanatory => EXPLANATORY_DESC,
        TeachingMode::StoryTeller => STORYTELLER_DESC,
        TeachingMode::Debug => DEBUG_DESC,
    }
}
```

**config.rs:**

Update temperature_for_mode() to group Interleaved with Immersive:
```rust
pub(crate) fn temperature_for_mode(mode: &TeachingMode) -> f64 {
    match mode {
        TeachingMode::Immersive | TeachingMode::Interleaved => 0.3,
        // ... rest unchanged
    }
}
```

Update tokens_per_mode() to group Interleaved with Immersive:
```rust
pub(crate) fn tokens_per_mode(mode: &TeachingMode) -> u32 {
    match mode {
        TeachingMode::Immersive | TeachingMode::Interleaved => 256,
        // ... rest unchanged
    }
}
```

### Deliverables
- [ ] INTERLEAVED_DESC constant deleted
- [ ] mode_description() groups Interleaved with Immersive
- [ ] temperature_for_mode() groups Interleaved with Immersive
- [ ] tokens_per_mode() groups Interleaved with Immersive
- [ ] All tests pass

### Phase End Verification

**MANDATORY - Do not skip any step:**
1. Run FULL test suite: `cargo test --all`
2. Run clippy on ALL crates: `cargo clippy --all -- -D warnings`
3. Fix ALL failures - there is no "out of scope". If you see a failure, fix it.
4. Remove ALL dead code. No excuses. You see dead code, remove it.
5. Update the Implementation Status table below with results.
6. Only after 100% pass: `git add -A && git commit -m "Phase 3 (Update response config - group Interleaved with Immersive) complete"`

---

## Phase 4: Update Frontend - Hide Interleaved from UI

### Code Style Checklist
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No dead code
- [ ] Tests for new functions

### Subagent: kiss-code-generator

### Files to Modify

1. **frontend/src/components/utility_sidebar/settings.rs**
2. **frontend/src/app/user_state/callbacks.rs**
3. **frontend/src/app/app_state/user/helpers.rs**
4. **frontend/src/app/app_state/user/tests.rs**
5. **shared/src/models/user_state.rs**

### Precise Changes

**settings.rs - Remove Interleaved from dropdown:**
```rust
// DELETE this line:
<option value="interleaved" selected={us.teaching_mode == TeachingMode::Interleaved}>{"Interleaved"}</option>
```

**callbacks.rs - Remove Interleaved case:**
```rust
// DELETE this line from on_teaching_mode_change():
"interleaved" => TeachingMode::Interleaved,
```

**helpers.rs - Update cycle to skip Interleaved:**
```rust
pub fn cycle_teaching_mode(current: TeachingMode) -> TeachingMode {
    match current {
        TeachingMode::Immersive => TeachingMode::Corrective,
        TeachingMode::Corrective => TeachingMode::Explanatory,
        TeachingMode::Explanatory => TeachingMode::StoryTeller,
        TeachingMode::Interleaved => TeachingMode::StoryTeller, // Skip past deprecated mode
        TeachingMode::StoryTeller => TeachingMode::Debug,
        TeachingMode::Debug => TeachingMode::Immersive,
    }
}
```

**tests.rs - Update cycle test:**
```rust
#[test]
fn test_cycle_teaching_mode() {
    assert_eq!(
        cycle_teaching_mode(TeachingMode::Immersive),
        TeachingMode::Corrective
    );
    assert_eq!(
        cycle_teaching_mode(TeachingMode::Corrective),
        TeachingMode::Explanatory
    );
    assert_eq!(
        cycle_teaching_mode(TeachingMode::Explanatory),
        TeachingMode::StoryTeller
    );
    assert_eq!(
        cycle_teaching_mode(TeachingMode::Interleaved),
        TeachingMode::StoryTeller
    );
    assert_eq!(
        cycle_teaching_mode(TeachingMode::StoryTeller),
        TeachingMode::Debug
    );
    assert_eq!(
        cycle_teaching_mode(TeachingMode::Debug),
        TeachingMode::Immersive
    );
}
```

**user_state.rs - Group Interleaved with Immersive in display:**
```rust
pub fn teaching_mode_display(&self) -> &'static str {
    match self.teaching_mode {
        TeachingMode::Immersive | TeachingMode::Interleaved => "Immersive",
        TeachingMode::Corrective => "Corrective",
        TeachingMode::Explanatory => "Explanatory",
        TeachingMode::StoryTeller => "Storyteller",
        TeachingMode::Debug => "Debug",
    }
}
```

### Deliverables
- [ ] Interleaved not in dropdown
- [ ] Interleaved not in callback handler
- [ ] Cycle skips from Interleaved to StoryTeller
- [ ] Display shows "Immersive" for Interleaved
- [ ] Cycle test updated
- [ ] Frontend builds successfully

### Phase End Verification

**MANDATORY - Do not skip any step:**
1. Run FULL test suite: `cargo test --all`
2. Run clippy on ALL crates: `cargo clippy --all -- -D warnings`
3. Build frontend: `cd frontend && trunk build && cd ..`
4. Fix ALL failures - there is no "out of scope". If you see a failure, fix it.
5. Remove ALL dead code. No excuses. You see dead code, remove it.
6. Update the Implementation Status table below with results.
7. Only after 100% pass: `git add -A && git commit -m "Phase 4 (Update frontend - hide Interleaved from UI) complete"`

---

## Phase 5: Migration + Remove Enum Variant

### Code Style Checklist
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No defensive coding
- [ ] No dead code
- [ ] Tests for new functions

### Subagent: modular-builder

### Files to Modify

1. **shared/src/models/versioning.rs** - Add V2 migration
2. **shared/src/models/dialect.rs** - Remove Interleaved variant
3. **shared/src/models/user_state.rs** - Remove Interleaved from display match
4. **backend/src/agent_service/learning.rs** - Remove `| TeachingMode::Interleaved` from patterns
5. **backend/src/agent_service/response/teaching.rs** - Remove `| TeachingMode::Interleaved` from pattern
6. **backend/src/agent_service/response/config.rs** - Remove `| TeachingMode::Interleaved` from patterns
7. **frontend/src/app/app_state/user/helpers.rs** - Remove Interleaved arm from cycle
8. **frontend/src/app/app_state/user/tests.rs** - Remove Interleaved assertion from cycle test

### Precise Changes

**versioning.rs - Add V2 migration:**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum UserStateVersion {
    V1,
    #[default]
    V2RemoveInterleaved,
}

pub const CURRENT_USER_STATE_VERSION: UserStateVersion = UserStateVersion::V2RemoveInterleaved;

pub fn migrate_user_state_to_current(
    from_version: UserStateVersion,
    data: serde_json::Value,
) -> UserState {
    match from_version {
        UserStateVersion::V1 => {
            let migrated = migrate_v1_to_v2(data);
            serde_json::from_value(migrated).expect("Valid V2 UserState after migration")
        }
        UserStateVersion::V2RemoveInterleaved => {
            serde_json::from_value(data).expect("Valid V2 UserState")
        }
    }
}

fn migrate_v1_to_v2(mut data: serde_json::Value) -> serde_json::Value {
    if let Some(obj) = data.as_object_mut() {
        if obj.get("teaching_mode") == Some(&serde_json::json!("interleaved")) {
            obj.insert("teaching_mode".to_string(), serde_json::json!("immersive"));
        }
    }
    data
}
```

**dialect.rs - Remove Interleaved variant:**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TeachingMode {
    #[serde(rename = "immersive")]
    Immersive,
    #[serde(rename = "corrective")]
    Corrective,
    #[serde(rename = "explanatory")]
    Explanatory,
    #[serde(rename = "storyteller")]
    StoryTeller,
    #[serde(rename = "debug")]
    Debug,
}
```

**user_state.rs - Remove Interleaved from display:**
```rust
pub fn teaching_mode_display(&self) -> &'static str {
    match self.teaching_mode {
        TeachingMode::Immersive => "Immersive",
        TeachingMode::Corrective => "Corrective",
        TeachingMode::Explanatory => "Explanatory",
        TeachingMode::StoryTeller => "Storyteller",
        TeachingMode::Debug => "Debug",
    }
}
```

**learning.rs - Remove Interleaved from patterns:**
```rust
// Change:
matches!(mode, TeachingMode::Immersive | TeachingMode::Interleaved | TeachingMode::Debug)
// To:
matches!(mode, TeachingMode::Immersive | TeachingMode::Debug)

// Change:
TeachingMode::Immersive | TeachingMode::Interleaved | TeachingMode::Debug => String::new(),
// To:
TeachingMode::Immersive | TeachingMode::Debug => String::new(),
```

**teaching.rs - Remove Interleaved from pattern:**
```rust
// Change:
TeachingMode::Immersive | TeachingMode::Interleaved => IMMERSIVE_DESC,
// To:
TeachingMode::Immersive => IMMERSIVE_DESC,
```

**config.rs - Remove Interleaved from patterns:**
```rust
// Change:
TeachingMode::Immersive | TeachingMode::Interleaved => 0.3,
// To:
TeachingMode::Immersive => 0.3,

// Change:
TeachingMode::Immersive | TeachingMode::Interleaved => 256,
// To:
TeachingMode::Immersive => 256,
```

**helpers.rs - Remove Interleaved from cycle:**
```rust
pub fn cycle_teaching_mode(current: TeachingMode) -> TeachingMode {
    match current {
        TeachingMode::Immersive => TeachingMode::Corrective,
        TeachingMode::Corrective => TeachingMode::Explanatory,
        TeachingMode::Explanatory => TeachingMode::StoryTeller,
        TeachingMode::StoryTeller => TeachingMode::Debug,
        TeachingMode::Debug => TeachingMode::Immersive,
    }
}
```

**tests.rs - Remove Interleaved from cycle test:**
```rust
#[test]
fn test_cycle_teaching_mode() {
    assert_eq!(
        cycle_teaching_mode(TeachingMode::Immersive),
        TeachingMode::Corrective
    );
    assert_eq!(
        cycle_teaching_mode(TeachingMode::Corrective),
        TeachingMode::Explanatory
    );
    assert_eq!(
        cycle_teaching_mode(TeachingMode::Explanatory),
        TeachingMode::StoryTeller
    );
    assert_eq!(
        cycle_teaching_mode(TeachingMode::StoryTeller),
        TeachingMode::Debug
    );
    assert_eq!(
        cycle_teaching_mode(TeachingMode::Debug),
        TeachingMode::Immersive
    );
}
```

### Deliverables
- [ ] V2 migration converts stored "interleaved" to "immersive"
- [ ] TeachingMode enum has 5 variants (no Interleaved)
- [ ] No remaining `TeachingMode::Interleaved` references in codebase
- [ ] All tests pass
- [ ] Frontend builds

### Phase End Verification

**MANDATORY - Do not skip any step:**
1. Run FULL test suite: `cargo test --all`
2. Run clippy on ALL crates: `cargo clippy --all -- -D warnings`
3. Build frontend: `cd frontend && trunk build && cd ..`
4. Verify no remaining Interleaved references: `grep -r "Interleaved" --include="*.rs" . | grep -v "target/" | grep -v "REMOVE_INTERLEAVED"` (should return empty)
5. Fix ALL failures - there is no "out of scope". If you see a failure, fix it.
6. Remove ALL dead code. No excuses. You see dead code, remove it.
7. Update the Implementation Status table below with results.
8. Only after 100% pass: `git add -A && git commit -m "Phase 5 (Migration + remove enum variant) complete"`

---

## Implementation Status

| Phase | Description | Status | Tests |
|-------|-------------|--------|-------|
| 1 | Add TranslationAgent + integrate | Pending | - |
| 2 | Update LearningAgent - group Interleaved with Immersive | Pending | - |
| 3 | Update response config - group Interleaved with Immersive | Pending | - |
| 4 | Update frontend - hide Interleaved from UI | Pending | - |
| 5 | Migration + remove enum variant | Pending | - |

## Test Results

_To be filled in as phases complete_

## Issues Encountered

_To be filled in during implementation_
