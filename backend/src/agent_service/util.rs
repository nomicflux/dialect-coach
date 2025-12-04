use dialect_coach_shared::{Explained, Exploratory, Mistake, Translated};
use rig::completion::{
    Message as RigMessage, message::AssistantContent, message::Text, message::UserContent,
};
use rig::one_or_many::OneOrMany;

pub const JSON_OUTPUT_INSTRUCTION: &str =
    "Return raw JSON only. No markdown code blocks. Start with { end with }.";

pub fn learning_goals_section(goals: &[dialect_coach_shared::LearningGoal]) -> String {
    if goals.is_empty() {
        return String::new();
    }
    let goals_list = goals
        .iter()
        .enumerate()
        .map(|(i, learning_goal)| format!("{}. {}", i + 1, learning_goal.goal))
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
        .map(|m| {
            format!(
                "- {} -> {} ({})",
                m.specific_mistake, m.correction, m.mistake_category
            )
        })
        .collect::<Vec<_>>();
    render_learning_section("Mistakes to watch - guide the user to correct usage, and demonstrate correct usage naturally of 1-2 items if any exist", lines)
}

fn explained_section(explained: &[Explained]) -> Option<String> {
    let lines = explained
        .iter()
        .map(|e| format!("- {} — {}", e.new_phrase, e.explanation))
        .collect::<Vec<_>>();
    render_learning_section(
        "Explained items in progress - use 1-2 of these as a basis for conversation if any exist",
        lines,
    )
}

fn translated_section(translated: &[Translated]) -> Option<String> {
    let lines = translated
        .iter()
        .map(|t| format!("- {} -> {}", t.translated_word, t.translated_to))
        .collect::<Vec<_>>();
    render_learning_section(
        "Translations already covered - use 1-3 of these words in your response if any exist, and guide the user to use them",
        lines,
    )
}

fn exploratory_section(exploratory: &[Exploratory]) -> Option<String> {
    let lines = exploratory
        .iter()
        .map(|e| format!("- {} — {}", e.point_to_try, e.instructions_for_use))
        .collect::<Vec<_>>();
    render_learning_section(
        "Exploratory prompts assigned - use 1-2 of these in your conversation with the user if any exist",
        lines,
    )
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
        RigMessage::Assistant { content, .. } => get_assistant_content(&content.first()),
    }
}

pub fn create_prefilled_assistant_message() -> RigMessage {
    RigMessage::Assistant {
        id: None,
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
    use dialect_coach_shared::{Explained, Exploratory, Mistake, MistakeCategory, Translated};

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
        let translated = Translated::new("house".to_string(), "casa".to_string(), None);
        let exploratory = Exploratory::new(
            "Usa el pretérito".to_string(),
            "Haz una frase corta".to_string(),
        );

        let summary =
            format_learning_items_context(&[mistake], &[explained], &[translated], &[exploratory]);

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
