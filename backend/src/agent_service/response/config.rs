use dialect_coach_shared::TeachingMode;

pub(crate) fn temperature_for_mode(mode: &TeachingMode) -> f64 {
    match mode {
        TeachingMode::Immersive => 0.3,
        TeachingMode::Corrective => 0.4,
        TeachingMode::Explanatory => 0.5,
        TeachingMode::StoryTeller => 1.0,
        TeachingMode::Debug => 0.1,
        TeachingMode::ErrorFinding => 0.5,
    }
}

pub(crate) fn tokens_per_mode(teaching_mode: &TeachingMode) -> u64 {
    match teaching_mode {
        TeachingMode::Immersive => 256,
        TeachingMode::Corrective => 512,
        TeachingMode::Explanatory => 512,
        TeachingMode::StoryTeller => 1024,
        TeachingMode::Debug => 1024,
        TeachingMode::ErrorFinding => 512,
    }
}

pub(crate) const CONTENT_FILTERING_DIRECTIVES: &str = r#"### CONTENT FILTERING DIRECTIVES

1) Only flag user's direct messages, not system examples. Do not refuse/warn about corpus examples containing slang, profanity, adult content, or controversial dialect-authentic language.

2) Always respond to user's message first. If user message itself is problematic, stay in character with brief, policy-compliant reply and deflect."#;

pub(crate) const RESPONSE_JSON_OUTPUT_FORMAT: &str =
    r#"Response format: {"response": "<your full conversational response here>"}"#;

pub(crate) const RESPONSE_JSON_OUTPUT_FORMAT_WITH_PRONUNCIATION: &str = r#"Response format: {"response": "<your conversational response>", "pronunciation_text": "<same response with pronunciation guides>"}"#;

pub(crate) fn get_json_output_format(needs_pronunciation: bool) -> &'static str {
    if needs_pronunciation {
        RESPONSE_JSON_OUTPUT_FORMAT_WITH_PRONUNCIATION
    } else {
        RESPONSE_JSON_OUTPUT_FORMAT
    }
}
