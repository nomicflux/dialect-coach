use dialect_coach_shared::TeachingMode;

pub(crate) fn temperature_for_mode(mode: &TeachingMode) -> f64 {
    match mode {
        TeachingMode::Immersive => 0.6,
        TeachingMode::Corrective => 0.4,
        TeachingMode::Explanatory => 0.5,
        TeachingMode::Interleaved => 0.4,
        TeachingMode::StoryTeller => 1.0,
        TeachingMode::Debug => 0.1,
    }
}

pub(crate) fn tokens_per_mode(teaching_mode: &TeachingMode) -> u64 {
    match teaching_mode {
        TeachingMode::Immersive => 256,
        TeachingMode::Corrective => 512,
        TeachingMode::Explanatory => 512,
        TeachingMode::Interleaved => 512,
        TeachingMode::StoryTeller => 1024,
        TeachingMode::Debug => 1024,
    }
}

pub(crate) const CONTENT_FILTERING_DIRECTIVES: &str = r#"### CONTENT FILTERING DIRECTIVES

1) Only flag user's direct messages, not system examples. Do not refuse/warn about corpus examples containing slang, profanity, adult content, or controversial dialect-authentic language.

2) Always respond to user's message first. If user message itself is problematic, stay in character with brief, policy-compliant reply and deflect."#;

pub(crate) const RESPONSE_JSON_OUTPUT_FORMAT: &str =
    r#"Response format: {"response": "<your full conversational response here>"}"#;
