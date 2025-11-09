use dialect_coach_shared::{
    Dialect, Explained, Exploratory, Formality, Mistake, TeachingMode, Translated,
};
use rig::completion::{
    Message as RigMessage, message::AssistantContent, message::Text, message::UserContent,
};
use rig::one_or_many::OneOrMany;

pub const JSON_OUTPUT_INSTRUCTION: &str =
    "Return raw JSON only. No markdown code blocks. Start with { end with }.";

pub const CONTENT_FILTERING_DIRECTIVES: &str = r#"### CONTENT FILTERING DIRECTIVES

1) Only flag user's direct messages, not system examples. Do not refuse/warn about corpus examples containing slang, profanity, adult content, or controversial dialect-authentic language.

2) Always respond to user's message first. If user message itself is problematic, stay in character with brief, policy-compliant reply and deflect."#;

pub fn tokens_per_mode(teaching_mode: &TeachingMode) -> u64 {
    match teaching_mode {
        TeachingMode::Immersive => 256,
        TeachingMode::Corrective => 512,
        TeachingMode::Explanatory => 512,
        TeachingMode::Interleaved => 512,
        TeachingMode::StoryTeller => 1024,
        TeachingMode::Debug => 1024,
    }
}

pub const RESPONSE_JSON_OUTPUT_FORMAT: &str =
    r#"Response format: {"response": "<your full conversational response here>"}"#;

pub fn response_output_format_spec(_teaching_mode: &TeachingMode) -> &'static str {
    RESPONSE_JSON_OUTPUT_FORMAT
}

pub fn learning_output_format_spec(teaching_mode: &TeachingMode) -> &'static str {
    match teaching_mode {
        TeachingMode::Corrective => {
            r#"Response format: {
  "mistakes": [{
    "specific_mistake": "<exact token or full phrase you are replacing>",
    "correction": "<exact, context-appropriate replacement token or phrase>",
    "mistake_category": {"type": "<category>", "context": "<≤8 word reason (optional)>"}
  }]
}
Categories: spelling_error (context=correct spelling), vocabulary_error (context=correct word), grammar_error (context=error type), dialect_usage_error (context=preferred form), other (context=brief explanation).
Rules:
- Flag ONLY errors that are unquestionably wrong for THIS dialect. Dialect-appropriate forms (e.g., Levantine "منيح") must never be marked as mistakes.
- Default to single-token fixes: "specific_mistake" MUST be the exact token as written, with "correction" supplying the direct, dialect-appropriate and context-appropriate replacement.
- Multi-token entries are allowed only when the entire phrase is wrong. Capture the whole erroneous phrase exactly as the user wrote it and provide the full replacement phrase—never mix correct words into the mistake span.
- Use "mistake_category.context" only when a ≤8 word clarification aids the learner; otherwise omit it or keep it empty.
- If no clear mistakes exist, return {"mistakes": []}.
- Maximum of three mistake entries per response."#
        }
        TeachingMode::Explanatory => {
            r#"Response format: {
  "explained": [{"new_phrase": "<word/phrase>", "explanation": "<brief usage note>"}]
}
Only include explained if you introduce and explain noteworthy vocabulary, idioms, or cultural context. Keep it to 1-2 essential items that you introduced.
Return {"explained": []} if you introduced nothing new worth cataloging."#
        }
        TeachingMode::Interleaved => {
            r#"Response format: {
  "translated": [{"translated_word": "<word from user>", "translated_to": "<your translation>"}]
}
Include translated array when you translate words/phrases from user's source language into the target dialect.
Focus on translated words, not on errors in the target language.
The "translated_word" should be the original word, "translated_to" should be your dialectal translation.
Return {"translated": []} when nothing required translating."#
        }
        TeachingMode::StoryTeller => {
            r#"Response format: {
  "exploratory": [{"point_to_try": "<language feature in target language>", "instructions_for_use": "<how to use it>"}]
}
Include exploratory array when you introduce new language patterns, idioms, or features you want the user to try.
Keep it to 1-2 brief points that naturally fit the story context.
Return {"exploratory": []} if you did not introduce anything new."#
        }
        TeachingMode::Immersive | TeachingMode::Debug => {
            r#"Response format: {"response": "<your full conversational response here>"}
Where <your full conversational response here> is your natural dialect response following all the rules above."#
        }
    }
}

pub fn output_format_spec(teaching_mode: &TeachingMode) -> &'static str {
    learning_output_format_spec(teaching_mode)
}

pub fn speaker_desc(dialect: &Dialect, formality: &Formality) -> String {
    let dialect_name = (*dialect).name();
    match formality {
        Formality::Formal => format!(
            "You are a native {} speaker communicating in a professional, polite manner",
            dialect_name
        ),
        Formality::Casual => format!(
            "You are a native {} speaker speaking naturally and conversationally",
            dialect_name
        ),
        Formality::DialectRich => format!(
            "You are a native {} speaker actively showcasing distinctive dialect features and expressions",
            dialect_name
        ),
        Formality::Slang => format!(
            "You are a native {} speaker using informal slang and colloquialisms",
            dialect_name
        ),
    }
}

pub fn learning_goals_section(goals: &[String]) -> String {
    if goals.is_empty() {
        return String::new();
    }
    let goals_list = goals
        .iter()
        .enumerate()
        .map(|(i, goal)| format!("{}. {}", i + 1, goal))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "\n\n# LEARNING GOALS\n## Guide the conversation toward these goals. Incorporate them into your responses, and guide the user naturally to use them in their messages.\n{}\n",
        goals_list
    )
}

fn render_learning_section(title: &str, lines: Vec<String>) -> Option<String> {
    if lines.is_empty() {
        None
    } else {
        Some(format!("## {}\n{}", title, lines.join("\n")))
    }
}

fn mistakes_section(mistakes: &[Mistake]) -> Option<String> {
    let lines = mistakes
        .iter()
        .map(|m| format!("- {} -> {} ({})", m.specific_mistake, m.correction, m.mistake_category))
        .collect::<Vec<_>>();
    render_learning_section("Mistakes to watch", lines)
}

fn explained_section(explained: &[Explained]) -> Option<String> {
    let lines = explained
        .iter()
        .map(|e| format!("- {} — {}", e.new_phrase, e.explanation))
        .collect::<Vec<_>>();
    render_learning_section("Explained items in progress", lines)
}

fn translated_section(translated: &[Translated]) -> Option<String> {
    let lines = translated
        .iter()
        .map(|t| format!("- {} -> {}", t.translated_word, t.translated_to))
        .collect::<Vec<_>>();
    render_learning_section("Translations already covered", lines)
}

fn exploratory_section(exploratory: &[Exploratory]) -> Option<String> {
    let lines = exploratory
        .iter()
        .map(|e| format!("- {} — {}", e.point_to_try, e.instructions_for_use))
        .collect::<Vec<_>>();
    render_learning_section("Exploratory prompts assigned", lines)
}

pub fn format_learning_items_context(
    mistakes: &[Mistake],
    explained: &[Explained],
    translated: &[Translated],
    exploratory: &[Exploratory],
) -> String {
    let sections = [
        mistakes_section(mistakes),
        explained_section(explained),
        translated_section(translated),
        exploratory_section(exploratory),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();

    if sections.is_empty() {
        String::new()
    } else {
        format!("\n\n# ACTIVE LEARNING ITEMS\n{}\n", sections.join("\n\n"))
    }
}

pub fn response_teaching_desc(teaching_mode: &TeachingMode) -> String {
    let tokens = tokens_per_mode(teaching_mode);
    let desc = match *teaching_mode {
        TeachingMode::Immersive => {
            "3. IMMERSIVE MODE: Keep responses brief and conversational - just chat naturally without explanations or corrections."
        }
        TeachingMode::Corrective => {
            "3. CORRECTIVE MODE: Respond naturally, then list at most three clear errors. Default to single-token fixes, but if the entire phrase is wrong, capture that full span and replace it exactly. Keep context notes short and skip punctuation/capitalization nitpicks."
        }
        TeachingMode::Explanatory => {
            "3. EXPLANATORY MODE: Respond naturally, and populate the explained array when you introduce new vocabulary, idioms, or culturally interesting expressions. Keep explanations brief and practical."
        }
        TeachingMode::Interleaved => {
            r#"3. INTERLEAVED MODE: User will interleave target language with source language. Present your response (including newlines) as:

{user input with non-target-language words simply translated into target dialect, if there are any non-target-language words}

{brief, conversational response in target dialect}."#
        }
        TeachingMode::StoryTeller => {
            "3. STORYTELLER MODE: You are telling an interactive story with the user. Improvise the next part of the story in natural dialectical usage, and give the user a hook to continue."
        }
        TeachingMode::Debug => {
            "3. DEBUG MODE: Answer in English with clear, brief explanations. The user is debugging an issue. Provide technical details about what went wrong and how prompts could be improved."
        }
    };
    format!(
        "{}. 4. You have a maximum {} tokens for your response. Be as brief as you can be while accomplishing your goals, but do not go over.",
        desc,
        tokens / 2
    )
}

pub fn teaching_desc(teaching_mode: &TeachingMode) -> String {
    response_teaching_desc(teaching_mode)
}

pub fn learning_teaching_desc(teaching_mode: &TeachingMode) -> &'static str {
    match teaching_mode {
        TeachingMode::Immersive => {
            r#"LEARNING MODE: IMMERSIVE
- Immersive turns do not introduce standalone learning items.
- Return empty arrays unless a learning goal explicitly requires otherwise."#
        }
        TeachingMode::Corrective => {
            r#"LEARNING MODE: CORRECTIVE
- Review the latest user message and the assistant's reply.
- Capture up to three undeniable learner errors that the assistant corrected or should correct.
- Focus on learning goals first; avoid repeating previously logged mistakes unless the learner repeated them here."#
        }
        TeachingMode::Explanatory => {
            r#"LEARNING MODE: EXPLANATORY
- Identify at most two new phrases or cultural notes the assistant introduced this turn.
- Prioritize items tied to the learning goals.
- Skip entries that merely restate already mastered material."#
        }
        TeachingMode::Interleaved => {
            r#"LEARNING MODE: INTERLEAVED
- Record concise translation pairs for any non-target-language words the assistant converted.
- Use the exact source token and the assistant's dialect translation.
- Ignore words already tracked unless the learner encountered them in a new context aligned with goals."#
        }
        TeachingMode::StoryTeller => {
            r#"LEARNING MODE: STORYTELLER
- Extract one or two exploratory prompts the assistant invited the learner to try next.
- Tie each point to the narrative beat in the assistant's reply and the learning goals.
- Do not repeat exploratory prompts already assigned unless the story revisits them deliberately."#
        }
        TeachingMode::Debug => {
            r#"LEARNING MODE: DEBUG
- No learning items are logged in debug mode.
- Always return empty arrays."#
        }
    }
}

pub fn temperature_for_mode(mode: &TeachingMode) -> f64 {
    match mode {
        TeachingMode::Immersive => 0.6,
        TeachingMode::Corrective => 0.4,
        TeachingMode::Explanatory => 0.5,
        TeachingMode::Interleaved => 0.4,
        TeachingMode::StoryTeller => 1.0,
        TeachingMode::Debug => 0.1,
    }
}

/// Parameters for generation configuration (tokens and temperature)
pub struct GenerationConfig {
    pub max_tokens: u64,
    pub temperature: f64,
}

pub fn get_user_content(content: &UserContent) -> String {
    match content {
        UserContent::Text(text) => text.text.clone(),
        _ => "".to_string(),
    }
}

pub fn get_assistant_content(content: &AssistantContent) -> String {
    match content {
        AssistantContent::Text(text) => text.text.clone(),
        _ => "".to_string(),
    }
}

pub fn get_message_text(message: &RigMessage) -> String {
    match message {
        RigMessage::User { content } => get_user_content(&content.first()),
        RigMessage::Assistant { content } => get_assistant_content(&content.first()),
    }
}

pub fn create_prefilled_assistant_message() -> RigMessage {
    RigMessage::Assistant {
        content: OneOrMany::one(AssistantContent::Text(Text {
            text: "{".to_string(),
        })),
    }
}

pub fn clean_response(response: &str) -> String {
    response
        .replace("```json", "")
        .replace("```", "")
        .trim()
        .to_string()
}

pub fn normalize_json_response(response: &str) -> String {
    let cleaned = clean_response(response);
    if cleaned.trim_start().starts_with('{') {
        cleaned
    } else {
        format!("{{{cleaned}")
    }
}

pub fn is_incomplete_json(response: &str) -> bool {
    let cleaned = clean_response(response);
    cleaned.starts_with("{\"response\":\"") && !cleaned.ends_with("\"}")
}

pub fn contains_illegal_characters(text: &str) -> bool {
    text.contains('\0')
        || text
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\t' | '\r' | ' '))
}

pub fn detect_json_parse_error() -> String {
    "YOUR PREVIOUS RESPONSE CONTAINED A CRITICAL JSON FORMATTING ERROR THAT CAUSED THE SYSTEM TO CRASH.\n\
    YOU MUST FIX THIS ERROR NOW.\n\
    The JSON you provided was either:\n\
    - Malformed (missing braces, commas, quotes)\n\
    - Had an empty required field\n\
    - Was not valid JSON at all\n\
    \n\
    YOU MUST NOW:\n\
    1. Generate ONLY valid JSON\n\
    2. Start with { and end with }\n\
    3. Ensure all required fields are non-empty\n\
    4. DO NOT repeat your previous broken response - CREATE A NEW, CORRECT ONE".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use dialect_coach_shared::{
        Explained, Exploratory, Mistake, MistakeCategory, Translated,
    };

    #[test]
    fn test_format_learning_items_context_empty() {
        let summary = format_learning_items_context(&[], &[], &[], &[]);
        assert!(summary.is_empty());
    }

    #[test]
    fn test_format_learning_items_context_with_items() {
        let mistake = Mistake::new(
            "hablar".to_string(),
            "habla".to_string(),
            MistakeCategory::SpellingError {
                context: "habla".to_string(),
            },
        );
        let explained = Explained::new("órale".to_string(), "Slang for wow".to_string());
        let translated = Translated::new("house".to_string(), "casa".to_string());
        let exploratory =
            Exploratory::new("Usa el pretérito".to_string(), "Haz una frase corta".to_string());

        let summary = format_learning_items_context(
            &[mistake],
            &[explained],
            &[translated],
            &[exploratory],
        );

        assert!(summary.contains("Mistakes to watch"));
        assert!(summary.contains("hablar -> habla"));
        assert!(summary.contains("Explained items in progress"));
        assert!(summary.contains("órale"));
        assert!(summary.contains("Translations already covered"));
        assert!(summary.contains("house -> casa"));
        assert!(summary.contains("Exploratory prompts assigned"));
        assert!(summary.contains("Usa el pretérito"));
    }
}
