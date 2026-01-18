use super::config::tokens_per_mode;
use dialect_coach_shared::{CefrLevel, JlptLevel, LanguageLevel, TeachingMode};

const IMMERSIVE_DESC: &str = "IMMERSIVE MODE: Keep responses brief and conversational - just chat naturally without explanations or corrections. If the user codeswitches to English, incorporate the dialect translation of one phrase naturally as you continue the conversation.";

const CORRECTIVE_DESC: &str = r#"CORRECTIVE MODE: Respond naturally, warmly but concisely.
If and only if the user made mistakes in their previous message, include some corrected versions of their specific mistakes as a gentle guide, ONLY if the user made mistakes and scoped to their mistakes.
Otherwise, continue the conversation naturally while naturally incorporating learning items.
If the user codeswitches to English, incorporate the dialect translation of one phrase naturally."#;

const EXPLANATORY_DESC: &str = r#"EXPLANATORY MODE: Respond naturally and curiously. Introduce NEW vocabulary, idioms, or culturally interesting expressions.
Keep explanations brief and practical.
Introduce NEW items, do not make corrections.
If previous user message used previously explained items from the learning item list, continue talking about them. If the user codeswitches to English, incorporate the dialect translation of one phrase naturally."#;

const STORYTELLER_DESC: &str = r#"STORYTELLER MODE: You are telling an interactive story with the user.
Improvise the next part of the story in natural dialectical usage, and give the user a hook to continue.
Keeping the story flow is important. Improvise. Go with the flow. Do not be didactic. Be creative. If starting the story, it is your responsibility to provide the beginning, not the user.
Do not explain what you are doing. Just tell the story. Keep the flow immersive.
Keep your story evocative yet brief; you are telling this story together with the user.
Do not correct the user in your message or explain linguistic constructs. If the user makes mistakes, show correct usage without explanation and naturally within the story.
Use elements from previous messages. If the user codeswitches to English, incorporate the dialect translation of one phrase naturally into the story."#;

const DEBUG_DESC: &str = r#"DEBUG MODE: Answer in English with clear, brief explanations. The user is debugging an issue.
You are a prompt engineer.
Provide technical details about what went wrong and how prompts could be improved.
The prompts cannot be clarified to prevent every case of what not to do. Focus on how to make the prompt clearer about what the agent should do, given the specific failure mode.
Your deliverable will be the updated prompt, with commentary as appropriate for why the fixes are included.
Do not format the prompt. Do not include any markdown for any reason."#;

const ERROR_FINDING_DESC: &str = r#"ERROR FINDING MODE: Chat naturally and conversationally.
You MUST intentionally include grammatical/vocabulary/spelling errors that are common mistakes made by English speakers learning this language.
Do NOT acknowledge or point out your errors. Speak as if they are natural (the user must discover them).
If the user corrects your error, respond positively and encouragingly.
Never reveal you are making intentional errors. Keep responses brief."#;

const LEVEL_A1: &str = "LANGUAGE LEVEL A1 (Beginner): CRITICAL: You are strictly limited to A1 (Beginner) grammar. Do NOT use idiomatic expressions. Speak as if you are talking to a 6-year-old child who is just learning to speak. Use standard, short, predictable sentence structures only.";
const LEVEL_A2: &str = "LANGUAGE LEVEL A2 (Elementary): CRITICAL: Strictly limited to A2 (Elementary). Do NOT use rare vocabulary. Speak as if you are talking to a primary/elementary school student. Keep sentences short and direct.";
const LEVEL_B1: &str = "LANGUAGE LEVEL B1 (Intermediate): CRITICAL: Limited to B1 (Intermediate). Do NOT use complex literary structures. Speak as if you are talking to a tourist who knows the basics but struggles with fast or complex speech.";
const LEVEL_B2: &str = "LANGUAGE LEVEL B2 (Upper Intermediate): CRITICAL: Limited to B2 (Upper Intermediate). Speak as if you are talking to a high school exchange student. Natural speed is okay, but avoid obscure cultural references.";
const LEVEL_C1: &str = "LANGUAGE LEVEL C1 (Advanced): CRITICAL: C1 (Advanced). Speak as if to a university colleague. Full complexity is expected.";
const LEVEL_C2: &str = "LANGUAGE LEVEL C2 (Proficient): CRITICAL: C2 (Proficient). No grammatical restrictions. Speak as if to a native peer with a high level of education.";

const JLPT_N5: &str = "LANGUAGE LEVEL N5 (Beginner): CRITICAL: Strictly limited to N5. Speak as if you are talking to a 6-year-old child who is just learning to speak. Use standard, short, predictable sentence structures only.";
const JLPT_N4: &str = "LANGUAGE LEVEL N4 (Elementary): CRITICAL: Strictly limited to N4. Speak as if you are talking to a primary/elementary school student. Keep sentences short and direct.";
const JLPT_N3: &str = "LANGUAGE LEVEL N3 (Intermediate): CRITICAL: Limited to N3. Speak as if you are talking to a tourist who knows the basics but struggles with fast or complex speech. Articulate clearly with direct sentences.";
const JLPT_N2: &str = "LANGUAGE LEVEL N2 (Upper Intermediate): CRITICAL: Limited to N2. Speak as if you are talking to a high school exchange student. Natural speed is okay, but avoid obscure cultural references.";
const JLPT_N1: &str = "LANGUAGE LEVEL N1 (Advanced): CRITICAL: N1 (Advanced). No grammatical restrictions. Speak as if to a university colleague.";

fn mode_description(mode: &TeachingMode) -> &'static str {
    match mode {
        TeachingMode::Immersive => IMMERSIVE_DESC,
        TeachingMode::Corrective => CORRECTIVE_DESC,
        TeachingMode::Explanatory => EXPLANATORY_DESC,
        TeachingMode::StoryTeller => STORYTELLER_DESC,
        TeachingMode::Debug => DEBUG_DESC,
        TeachingMode::ErrorFinding => ERROR_FINDING_DESC,
    }
}

pub(crate) fn response_teaching_desc(teaching_mode: &TeachingMode) -> String {
    let tokens = tokens_per_mode(teaching_mode);
    let desc = mode_description(teaching_mode);
    format!(
        "{}. 4. You have a maximum of {} tokens for your response. Be as brief as you can be while accomplishing your goals. Do not go over your limit.",
        desc,
        tokens / 2
    )
}

fn cefr_instruction(level: CefrLevel) -> &'static str {
    match level {
        CefrLevel::A1 => LEVEL_A1,
        CefrLevel::A2 => LEVEL_A2,
        CefrLevel::B1 => LEVEL_B1,
        CefrLevel::B2 => LEVEL_B2,
        CefrLevel::C1 => LEVEL_C1,
        CefrLevel::C2 => LEVEL_C2,
    }
}

fn jlpt_instruction(level: JlptLevel) -> &'static str {
    match level {
        JlptLevel::N5 => JLPT_N5,
        JlptLevel::N4 => JLPT_N4,
        JlptLevel::N3 => JLPT_N3,
        JlptLevel::N2 => JLPT_N2,
        JlptLevel::N1 => JLPT_N1,
    }
}

pub(crate) fn language_level_instruction(level: LanguageLevel) -> &'static str {
    match level {
        LanguageLevel::Cefr(cefr) => cefr_instruction(cefr),
        LanguageLevel::Jlpt(jlpt) => jlpt_instruction(jlpt),
    }
}

const IMMERSIVE_CHECKLIST: &[&str] = &[
    "Response is brief and conversational",
    "No explanations or corrections included",
];

const CORRECTIVE_CHECKLIST: &[&str] = &[
    "Response is 1-2 short sentences maximum",
    "Corrections included ONLY if user made mistakes",
];

const EXPLANATORY_CHECKLIST: &[&str] = &[
    "Response is exactly 2-3 sentences",
    "Introduces NEW vocabulary, idiom, or cultural expression",
    "No corrections included",
];

const STORYTELLER_CHECKLIST: &[&str] = &[
    "Response is 2-4 sentences",
    "Story flows naturally without didactic explanation",
    "No explicit teaching or corrections",
];

const ERROR_FINDING_CHECKLIST: &[&str] = &[
    "Response is 1-2 sentences",
    "Contains 1-3 intentional errors",
    "Errors are not acknowledged or pointed out",
];

const DEBUG_CHECKLIST: &[&str] = &[
    "Response is in English",
    "Explains what went wrong clearly",
    "Provides concrete prompt improvements",
];

pub(crate) fn mode_checklist(mode: &TeachingMode) -> &'static [&'static str] {
    match mode {
        TeachingMode::Immersive => IMMERSIVE_CHECKLIST,
        TeachingMode::Corrective => CORRECTIVE_CHECKLIST,
        TeachingMode::Explanatory => EXPLANATORY_CHECKLIST,
        TeachingMode::StoryTeller => STORYTELLER_CHECKLIST,
        TeachingMode::Debug => DEBUG_CHECKLIST,
        TeachingMode::ErrorFinding => ERROR_FINDING_CHECKLIST,
    }
}

const LEVEL_A1_CHECKLIST: &[&str] = &[
    "All vocabulary is in top 500 most common words",
    "No relative clauses, past tense, or conditionals",
    "Sentences are 4-5 words each",
];

const LEVEL_A2_CHECKLIST: &[&str] = &[
    "No complex subordination or abstract nouns",
    "No passive voice",
    "Sentences are 5-8 words each",
];

const LEVEL_B1_CHECKLIST: &[&str] = &[
    "No highly nuanced academic vocabulary",
    "No obscure idioms without immediate explanation",
];

const LEVEL_B2_CHECKLIST: &[&str] = &[
    "No overly archaic or purely academic vocabulary",
    "Clarity prioritized over stylistic flourish",
];

const LEVEL_C1_CHECKLIST: &[&str] =
    &["No purely obscure or archaic terms outside general educated use"];

const LEVEL_C2_CHECKLIST: &[&str] = &["Nuance and precision maintained"];

const JLPT_N5_CHECKLIST: &[&str] = &[
    "ONLY Desu/Masu forms used (no casual/dictionary forms)",
    "Kanji limited to N5 set (~100)",
    "No complex conjunctions",
    "Sentences are 4-5 words each",
];

const JLPT_N4_CHECKLIST: &[&str] = &[
    "No N3+ grammar points",
    "Kanji limited to N4 set (~300)",
    "Sentences are 5-8 words each",
];

const JLPT_N3_CHECKLIST: &[&str] = &[
    "No overly formal business Japanese unless role demands",
    "No rare literary grammar",
];

const JLPT_N2_CHECKLIST: &[&str] = &["No highly specialized or archaic N1 vocabulary"];

const JLPT_N1_CHECKLIST: &[&str] = &["Full complexity expected"];

pub(crate) fn level_checklist(level: LanguageLevel) -> &'static [&'static str] {
    match level {
        LanguageLevel::Cefr(CefrLevel::A1) => LEVEL_A1_CHECKLIST,
        LanguageLevel::Cefr(CefrLevel::A2) => LEVEL_A2_CHECKLIST,
        LanguageLevel::Cefr(CefrLevel::B1) => LEVEL_B1_CHECKLIST,
        LanguageLevel::Cefr(CefrLevel::B2) => LEVEL_B2_CHECKLIST,
        LanguageLevel::Cefr(CefrLevel::C1) => LEVEL_C1_CHECKLIST,
        LanguageLevel::Cefr(CefrLevel::C2) => LEVEL_C2_CHECKLIST,
        LanguageLevel::Jlpt(JlptLevel::N5) => JLPT_N5_CHECKLIST,
        LanguageLevel::Jlpt(JlptLevel::N4) => JLPT_N4_CHECKLIST,
        LanguageLevel::Jlpt(JlptLevel::N3) => JLPT_N3_CHECKLIST,
        LanguageLevel::Jlpt(JlptLevel::N2) => JLPT_N2_CHECKLIST,
        LanguageLevel::Jlpt(JlptLevel::N1) => JLPT_N1_CHECKLIST,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_level_instruction_covers_all_cefr_levels() {
        let levels = [
            LanguageLevel::Cefr(CefrLevel::A1),
            LanguageLevel::Cefr(CefrLevel::A2),
            LanguageLevel::Cefr(CefrLevel::B1),
            LanguageLevel::Cefr(CefrLevel::B2),
            LanguageLevel::Cefr(CefrLevel::C1),
            LanguageLevel::Cefr(CefrLevel::C2),
        ];
        for level in levels {
            let instruction = language_level_instruction(level);
            assert!(!instruction.is_empty());
            assert!(instruction.contains("LANGUAGE LEVEL"));
        }
    }

    #[test]
    fn test_language_level_instruction_covers_all_jlpt_levels() {
        let levels = [
            LanguageLevel::Jlpt(JlptLevel::N5),
            LanguageLevel::Jlpt(JlptLevel::N4),
            LanguageLevel::Jlpt(JlptLevel::N3),
            LanguageLevel::Jlpt(JlptLevel::N2),
            LanguageLevel::Jlpt(JlptLevel::N1),
        ];
        for level in levels {
            let instruction = language_level_instruction(level);
            assert!(!instruction.is_empty());
            assert!(instruction.contains("LANGUAGE LEVEL"));
        }
    }
}
