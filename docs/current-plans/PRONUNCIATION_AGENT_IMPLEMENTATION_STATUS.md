# Pronunciation Agent Implementation Plan

## Summary

Split pronunciation text generation from the response agent into a dedicated pronunciation agent that runs in parallel with other post-response agents (learning, translation).

## PSP Rules (Required for Every Phase)

### Code Style Checklist (MANDATORY before writing code)
- [ ] Functions <20 lines, <10 if possible
- [ ] Pure functions preferred, side effects isolated
- [ ] No defensive coding - trust known input/output types
- [ ] No dead code - all code must be used immediately in the phase
- [ ] No TODOs - write working code now
- [ ] Tests for all new functions

### Phase Completion Gate (MANDATORY after every phase)
1. Run `cargo test` - 100% pass required
2. Run `cargo clippy` - fix ALL warnings, remove dead code
3. Update this document
4. Commit the phase: `git add <files> && git commit -m "Phase N complete"`
5. STOP and wait for user approval

### Subagent Specification (BINDING)
When a phase specifies "Subagent: X", the orchestrator MUST launch that subagent and MUST NOT write code itself.

---

## Background

### Current State
- Pronunciation instructions are embedded in the response agent's system prompt (`system_content.rs:270-272`)
- `build_pronunciation_instruction()` in `language_instructions.rs:35-66` generates dialect-specific harakat/furigana instructions
- Response agent returns both `response` and `pronunciation_text` in a single LLM call
- `needs_pronunciation_text()` in `shared/src/models/language_options.rs:78` determines when pronunciation is needed:
  - Arabic Naskh/Ruqa -> true (needs harakat for TTS)
  - Japanese Kanji -> true (needs furigana for TTS)
  - All others -> false
- `get_json_output_format()` in `config.rs:36` switches the JSON output format based on whether pronunciation is needed
- `AgentResponse.pronunciation_text: Option<String>` carries the result (with `#[serde(default)]`)

### Target State
- Response agent always returns `{"response": "..."}` only (no pronunciation instructions in system prompt)
- New `PronunciationAgent` receives the response text and generates `pronunciation_text` via a separate LLM call
- Pronunciation agent runs in parallel with learning/translation agents via `tokio::join!`
- For `skip_learning=true` paths (AI actions), pronunciation agent runs alone
- Final `AgentResponse` shape is unchanged (`pronunciation_text` populated when applicable)
- Pronunciation failure is non-fatal (log error, response works without pronunciation)

### Architecture Pattern
Follow `TranslationAgent` (`backend/src/agent_service/translation.rs`):
- `Params` struct with input references
- `Output` struct with results + `empty()` method
- `Agent` struct holding `Arc<dyn CompletionAgent>`
- `new(agent)` constructor
- `generate_X(&self, params) -> (Result<Output>, Vec<AgentUsage>)` method
- Free functions: system content builder, prompt builder, parser

### Data Flow (Current vs. New)

**Current:**
```
Response Agent LLM call
  -> system prompt includes pronunciation instructions
  -> returns {"response": "...", "pronunciation_text": "..."}
  -> parsed into AgentResponse with pronunciation_text populated
```

**New:**
```
Response Agent LLM call
  -> system prompt has NO pronunciation instructions
  -> returns {"response": "..."}
  -> parsed into AgentResponse with pronunciation_text = None

Then in parallel:
  tokio::join!(
    attach_learning_items(learning + translation agents),
    pronunciation_agent(response_text) -> pronunciation_text
  )
  -> merge pronunciation_text into AgentResponse
```

---

## Phase 1: Create Pronunciation Agent Module and Initial Integration

### Code Style Checklist
- [ ] Functions <20 lines, <10 if possible
- [ ] Pure functions preferred, side effects isolated
- [ ] No defensive coding
- [ ] No dead code
- [ ] No TODOs
- [ ] Tests for all new functions

**Subagent**: modular-builder

**Objective**: Create the pronunciation agent following TranslationAgent pattern and integrate into the `skip_learning=true` code paths in `generation.rs`. The response agent still generates pronunciation too (redundant but functional - pronunciation agent result overwrites).

### Files to Create
- `backend/src/agent_service/pronunciation.rs`

### Files to Modify
- `backend/src/agent_service.rs` - add `pub mod pronunciation;`
- `backend/src/agent_service/response/generation.rs` - add pronunciation calls in `skip_learning=true` branches

### Actionable Steps

#### Step 1: Create `backend/src/agent_service/pronunciation.rs`

**Structs:**
```rust
pub struct PronunciationAgentParams<'a> {
    pub response_text: &'a str,
    pub dialect: Dialect,
    pub language_option: &'a Option<LanguageOption>,
}

pub struct PronunciationAgentOutput {
    pub pronunciation_text: Option<String>,
}
// impl: fn empty() -> Self { pronunciation_text: None }

struct RawPronunciationOutput {  // #[derive(Deserialize)]
    pronunciation_text: String,
}

pub struct PronunciationAgent {
    agent: Arc<dyn CompletionAgent>,
}
```

**`PronunciationAgent` methods:**
- `new(agent: Arc<dyn CompletionAgent>) -> Self`
- `generate_pronunciation(&self, params) -> (Result<PronunciationAgentOutput>, Vec<AgentUsage>)`
  - Early return `empty()` when `!needs_pronunciation_text(params.language_option)`
  - Build system content + prompt, prefill `{` for Anthropic
  - `CompletionRequest { max_tokens: 512, temperature: 0.0 }`
  - `retry_completion_call(agent, request, 3)`, parse result

**Pure helper functions (<20 lines each):**

`build_pronunciation_system_content(dialect_name: &str, language_option: &Option<LanguageOption>) -> String`:
- Match on language_option, dispatch to Arabic or Japanese builder
- Wrap in system prompt template with role description, `JSON_OUTPUT_INSTRUCTION`, output format `{"pronunciation_text": "..."}`

`build_arabic_pronunciation_system(dialect_name: &str) -> String`:
- Adapted from `build_pronunciation_instruction` Arabic branch (language_instructions.rs:40-51)
- Content: add full harakat to all letters, MUST reflect dialect pronunciation not MSA, vowel patterns, dropped vowels, sukun, consonant changes

`build_japanese_pronunciation_system(dialect_name: &str) -> String`:
- Adapted from `build_pronunciation_instruction` Japanese branch (language_instructions.rs:53-62)
- Content: add furigana ruby tags to kanji, MUST reflect dialect pronunciation, dialect-specific readings

`build_pronunciation_prompt(response_text: &str) -> String`:
- `"Add pronunciation annotations to the following text:\n\n{response_text}"`

`try_parse_pronunciation_output(response: &str) -> Result<PronunciationAgentOutput>`:
- `normalize_json_response` -> deserialize `RawPronunciationOutput` -> wrap in output struct

**Tests:**
- `test_pronunciation_output_empty` - `empty()` returns None
- `test_parse_valid_json` - `{"pronunciation_text": "test"}` parses correctly
- `test_parse_invalid_json` - malformed JSON returns error
- `test_system_content_arabic_naskh` - contains "harakat" and dialect name
- `test_system_content_arabic_ruqa` - contains "harakat" and dialect name
- `test_system_content_japanese_kanji` - contains "furigana" and dialect name
- `test_prompt_includes_response_text` - prompt contains passed-in text

#### Step 2: Register module in `backend/src/agent_service.rs`

Add `pub mod pronunciation;` at line ~17 (between `pub mod planning;` and `pub mod provider;`).

#### Step 3: Integrate into `skip_learning=true` paths in `generation.rs`

**Add imports:**
```rust
use crate::agent_service::pronunciation::{
    PronunciationAgent, PronunciationAgentOutput, PronunciationAgentParams,
};
```

**Add method on `ResponseContext` impl block (before `attach_learning_items`):**
```rust
async fn generate_pronunciation_content(
    &self,
    params: &GenerateResponseParams<'_>,
    response_text: &str,
) -> (Result<PronunciationAgentOutput>, Vec<AgentUsage>) {
    let agent = PronunciationAgent::new(self.learning_agent.clone());
    let pron_params = PronunciationAgentParams {
        response_text,
        dialect: params.dialect.dialect,
        language_option: params.language_option,
    };
    agent.generate_pronunciation(&pron_params).await
}
```

**Add free functions:**
```rust
fn apply_pronunciation_result(
    response: &mut AgentResponse,
    result: Result<PronunciationAgentOutput>,
) {
    match result {
        Ok(output) => response.pronunciation_text = output.pronunciation_text,
        Err(e) => tracing::error!(error = %e, "Pronunciation agent failed"),
    }
}

async fn generate_and_apply_pronunciation(
    ctx: &ResponseContext,
    params: &GenerateResponseParams<'_>,
    mut response: AgentResponse,
) -> (AgentResponse, Vec<AgentUsage>) {
    let (result, usage) = ctx
        .generate_pronunciation_content(params, &response.response)
        .await;
    apply_pronunciation_result(&mut response, result);
    (response, usage)
}
```

**Modify `handle_parse_success` (line 292-293) - change `skip_learning=true` branch ONLY:**

Current:
```rust
if skip_learning {
    Ok((parsed_response, initial_usage, Vec::new()))
}
```
New:
```rust
if skip_learning {
    let (response, pron_usage) =
        generate_and_apply_pronunciation(ctx, params, parsed_response).await;
    Ok((response, initial_usage, pron_usage))
}
```

**Modify `execute_retry_with_learning` (line 410) - change `skip_learning=true` branch ONLY:**

Current:
```rust
if skip_learning {
    Ok((parsed_response, all_response_usage, Vec::new()))
}
```
New:
```rust
if skip_learning {
    let (response, pron_usage) =
        generate_and_apply_pronunciation(ctx, params, parsed_response).await;
    Ok((response, all_response_usage, pron_usage))
}
```

### Phase 1 Deliverables
- `pronunciation.rs` created with full agent implementation and 7 tests
- Module registered in `agent_service.rs`
- `skip_learning=true` paths call pronunciation agent via 3 new functions + 2 modified branches in `generation.rs`
- Response agent still generates pronunciation too (redundant, functional)

### Phase 1 Completion Gate
1. Run `cargo test` - 100% pass required
2. Run `cargo clippy` - fix ALL warnings, remove dead code
3. Update this document
4. `git add <specific files> && git commit -m "Phase 1 (pronunciation agent module + skip_learning integration) complete"`
5. STOP and wait for user approval

---

## Phase 2: Parallel Execution with Learning/Translation

### Code Style Checklist
- [ ] Functions <20 lines, <10 if possible
- [ ] Pure functions preferred, side effects isolated
- [ ] No defensive coding
- [ ] No dead code
- [ ] No TODOs
- [ ] Tests for all new functions

**Subagent**: kiss-code-generator

**Objective**: Wire pronunciation into the `skip_learning=false` path, running in parallel with learning/translation via `tokio::join!`. Replace the two old wrapper functions with a unified `run_post_response_agents`.

### Files to Modify
- `backend/src/agent_service/response/generation.rs`

### Actionable Steps

#### Step 1: Add `run_post_response_agents` function

```rust
async fn run_post_response_agents(
    ctx: &ResponseContext,
    params: &GenerateResponseParams<'_>,
    parsed_response: AgentResponse,
    response_usage: Vec<AgentUsage>,
) -> Result<(AgentResponse, Vec<AgentUsage>, Vec<AgentUsage>)> {
    let response_text = parsed_response.response.clone();
    let (learning_result, (pron_result, pron_usage)) = tokio::join!(
        ctx.attach_learning_items(params, parsed_response),
        ctx.generate_pronunciation_content(params, &response_text)
    );
    match learning_result {
        Ok((mut response, learning_usage)) => {
            apply_pronunciation_result(&mut response, pron_result);
            let mut all_learning = learning_usage;
            all_learning.extend(pron_usage);
            Ok((response, response_usage, all_learning))
        }
        Err(e) => {
            tracing::error!(
                dialect = %params.dialect.dialect.name(),
                error = %e, "Post-response agents failed"
            );
            Err(e)
        }
    }
}
```

#### Step 2: Update `handle_parse_success` `skip_learning=false` branch

Current:
```rust
} else {
    attach_learning_with_error_handling(ctx, params, parsed_response, initial_usage).await
}
```
New:
```rust
} else {
    run_post_response_agents(ctx, params, parsed_response, initial_usage).await
}
```

#### Step 3: Update `execute_retry_with_learning` `skip_learning=false` branch

Current:
```rust
} else {
    attach_learning_after_retry(ctx, params, parsed_response, all_response_usage).await
}
```
New:
```rust
} else {
    run_post_response_agents(ctx, params, parsed_response, all_response_usage).await
}
```

#### Step 4: Delete replaced functions (now dead code)

- Delete `attach_learning_with_error_handling` (lines 299-320)
- Delete `attach_learning_after_retry` (lines 420-436)

### Phase 2 Deliverables
- All code paths run pronunciation agent
- Pronunciation runs in parallel with learning/translation via `tokio::join!`
- Two old functions deleted, one new unified function replaces them

### Phase 2 Completion Gate
1. Run `cargo test` - 100% pass required
2. Run `cargo clippy` - fix ALL warnings, remove dead code
3. Update this document
4. `git add <specific files> && git commit -m "Phase 2 (parallel pronunciation execution) complete"`
5. STOP and wait for user approval

---

## Phase 3: Remove JSON Format Switching from Response Agent

### Code Style Checklist
- [ ] Functions <20 lines, <10 if possible
- [ ] Pure functions preferred, side effects isolated
- [ ] No defensive coding
- [ ] No dead code
- [ ] No TODOs
- [ ] Tests for all new functions

**Subagent**: kiss-code-generator

**Objective**: Remove the pronunciation-specific JSON output format from `config.rs` and update `system_content.rs` to always use the simple response format. After this phase, the response agent's JSON format no longer mentions `pronunciation_text`, but the pronunciation instruction text is still in the system prompt (removed in Phase 4).

### Files to Modify
- `backend/src/agent_service/response/config.rs`
- `backend/src/agent_service/response/system_content.rs`

### Actionable Steps

#### Step 1: In `config.rs`

- Delete `RESPONSE_JSON_OUTPUT_FORMAT_WITH_PRONUNCIATION` constant (line 34)
- Delete `get_json_output_format` function (lines 36-42)
- Keep `RESPONSE_JSON_OUTPUT_FORMAT` (line 31-32)

#### Step 2: In `system_content.rs` - imports

- Line 3: `use dialect_coach_shared::models::{LanguageOption, needs_pronunciation_text};` -> `use dialect_coach_shared::models::LanguageOption;`
- Line 9: `use super::config::{CONTENT_FILTERING_DIRECTIVES, get_json_output_format};` -> `use super::config::{CONTENT_FILTERING_DIRECTIVES, RESPONSE_JSON_OUTPUT_FORMAT};`

#### Step 3: In `system_content.rs` - `NormalSystemParams`

- Delete field (line 161): `json_output_format: &'a str`

#### Step 4: In `system_content.rs` - `build_normal_system_content`

- Line 218: replace `params.json_output_format` with `RESPONSE_JSON_OUTPUT_FORMAT`

#### Step 5: In `system_content.rs` - `build_system_content`

- Delete line 270: `let needs_pronunciation = needs_pronunciation_text(language_option);`
- Delete line 273: `let json_output_format = get_json_output_format(needs_pronunciation);`
- Delete from NormalSystemParams construction (line 301): `json_output_format,`

### Phase 3 Deliverables
- Response agent JSON format always `{"response": "..."}` - never mentions `pronunciation_text`
- `pronunciation_instruction` still in system prompt (rendered into prompt string, not dead code) - removed next phase

### Phase 3 Completion Gate
1. Run `cargo test` - 100% pass required
2. Run `cargo clippy` - fix ALL warnings, remove dead code
3. Update this document
4. `git add <specific files> && git commit -m "Phase 3 (remove JSON format switching) complete"`
5. STOP and wait for user approval

---

## Phase 4: Remove Pronunciation Instructions from Response Agent

### Code Style Checklist
- [ ] Functions <20 lines, <10 if possible
- [ ] Pure functions preferred, side effects isolated
- [ ] No defensive coding
- [ ] No dead code
- [ ] No TODOs
- [ ] Tests for all new functions

**Subagent**: kiss-code-generator

**Objective**: Remove all remaining pronunciation from the response agent. Delete `build_pronunciation_instruction` from `language_instructions.rs`, remove pronunciation fields from `NormalSystemParams`, clean up `build_normal_system_content`. After this phase, pronunciation comes entirely from the pronunciation agent.

### Files to Modify
- `backend/src/agent_service/response/system_content.rs`
- `backend/src/agent_service/language_instructions.rs`

### Actionable Steps

#### Step 1: In `system_content.rs` - imports

- Lines 14-16: `use crate::agent_service::language_instructions::{build_language_instruction, build_pronunciation_instruction};` -> `use crate::agent_service::language_instructions::build_language_instruction;`

#### Step 2: In `system_content.rs` - `NormalSystemParams`

- Delete field (line 160): `pronunciation_instruction: &'a str`

#### Step 3: In `system_content.rs` - `build_normal_system_content`

- Delete `pronunciation_section` variable (lines 165-169)
- In format string (line 189 area): remove `{}` placeholder for `pronunciation_section`
- Remove `pronunciation_section` from format args (line 219)

#### Step 4: In `system_content.rs` - `build_system_content`

- Delete lines 271-272: `let pronunciation_instruction = build_pronunciation_instruction(language_option, dialect.dialect.name());`
- Delete from NormalSystemParams construction (line 300): `pronunciation_instruction: &pronunciation_instruction,`

#### Step 5: In `language_instructions.rs` - delete function

- Delete `build_pronunciation_instruction` function and doc comment (lines 33-66)

#### Step 6: In `language_instructions.rs` - delete tests

- Delete all pronunciation tests (lines 149-198):
  - `test_pronunciation_instruction_arabic_naskh`
  - `test_pronunciation_instruction_arabic_ruqa`
  - `test_pronunciation_instruction_arabic_latin`
  - `test_pronunciation_instruction_arabic_fully_voweled`
  - `test_pronunciation_instruction_japanese_kanji`
  - `test_pronunciation_instruction_japanese_kanji_with_ruby`
  - `test_pronunciation_instruction_none`
- Keep `build_language_instruction` and its tests (lines 3-31, 68-148) unchanged

### Phase 4 Deliverables
- Response agent system prompt contains zero mention of pronunciation
- `build_pronunciation_instruction` deleted, all its tests deleted
- Pronunciation comes entirely from pronunciation agent (Phase 1)
- End result identical to pre-migration behavior: `AgentResponse.pronunciation_text` populated for Arabic Naskh/Ruqa and Japanese Kanji

### Phase 4 Completion Gate
1. Run `cargo test` - 100% pass required
2. Run `cargo clippy` - fix ALL warnings, remove dead code
3. Update this document
4. `git add <specific files> && git commit -m "Phase 4 (remove pronunciation from response agent) complete"`
5. STOP and wait for user approval

---

### Verification

After all deliverables:
1. `cargo test` - 100% pass
2. `cargo clippy` - zero warnings
3. No dead code: every new function is called, every removed function is no longer referenced
4. `AgentResponse.pronunciation_text` is still populated for Arabic Naskh/Ruqa and Japanese Kanji (via pronunciation agent instead of response agent)
5. Response agent system prompt no longer mentions pronunciation

---

## Agreements Made

(User agreements will be recorded here as the implementation proceeds)

## Explicitly Rejected

- Multi-phase split where Phase 1 creates agent and Phase 2 uses it (violates PSP: creates dead code in Phase 1)
- Making pronunciation failure fatal (non-fatal is consistent with current best-effort behavior)
- Single-phase plan combining all changes into one phase (rejected by user: violates PSP minimum 3 phases, too complex for review, no intermediate verification points)

## Issues Encountered

(Issues and their resolutions will be recorded here)
