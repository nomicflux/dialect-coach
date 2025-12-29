use super::config::tokens_per_mode;
use dialect_coach_shared::{CefrLevel, JlptLevel, LanguageLevel, TeachingMode};

const IMMERSIVE_DESC: &str = "3. IMMERSIVE MODE: Keep responses brief and conversational - just chat naturally without explanations or corrections. If the user codeswitches to English, incorporate the dialect translation of one phrase naturally as you continue the conversation.";

const CORRECTIVE_DESC: &str = r#"3. CORRECTIVE MODE: Respond naturally, warmly but concisely (hard limit of 1-2 short sentences).
If and only if the user made mistakes in their previous message, include some corrected versions as a gentle guide.
Otherwise, continue the conversation naturally while naturally incorporating learning items.
Inclusion of corrections and items is limited to what fits within the 1-2 sentence limit. If the user codeswitches to English, incorporate the dialect translation of one phrase naturally."#;

const EXPLANATORY_DESC: &str = r#"3. EXPLANATORY MODE: Respond naturally and curiously (2-3 sentences). Introduce NEW vocabulary, idioms, or culturally interesting expressions.
Keep explanations brief and practical.
Introduce NEW items, do not make corrections.
If previous user message used previously explained items from the learning item list, continue talking about them. If the user codeswitches to English, incorporate the dialect translation of one phrase naturally."#;

const STORYTELLER_DESC: &str = r#"3. STORYTELLER MODE: You are telling an interactive story with the user.
Improvise the next part of the story in natural dialectical usage, and give the user a hook to continue.
Keeping the story flow is important. Improvise. Go with the flow. Do not be didactic. Be creative. If starting the story, it is your responsibility to provide the beginning, not the user.
Do not explain what you are doing. Just tell the story. Keep the flow immersive.
Keep your story evocative yet brief (2-4 sentences); you are telling this story together with the user.
Do not correct the user in your message or explain linguistic constructs. If the user makes mistakes, show correct usage without explanation and naturally within the story.
Use elements from previous messages. If the user codeswitches to English, incorporate the dialect translation of one phrase naturally into the story."#;

const DEBUG_DESC: &str = r#"3. DEBUG MODE: Answer in English with clear, brief explanations. The user is debugging an issue.
You are a prompt engineer.
Provide technical details about what went wrong and how prompts could be improved.
The prompts cannot be clarified to prevent every case of what not to do. Focus on how to make the prompt clearer about what the agent should do, given the specific failure mode.
Your deliverable will be the updated prompt, with commentary as appropriate for why the fixes are included.
Do not format the prompt. Do not include any markdown for any reason."#;

const ERROR_FINDING_DESC: &str = r#"3. ERROR FINDING MODE: Chat naturally and conversationally.
You MUST intentionally include 1-3 grammatical/vocabulary/spelling errors that are common mistakes made by English speakers learning this language.
Do NOT acknowledge or point out your errors. Speak as if they are natural (the user must discover them).
If the user corrects your error, respond positively and encouragingly.
Never reveal you are making intentional errors. Keep responses brief (1-2 sentences)."#;

const LEVEL_A1: &str = "LANGUAGE LEVEL A1 (Beginner): Use very basic vocabulary and simple present tense. Short sentences only (one idea per sentence). Repeat key words. Speak slowly and clearly. If required vocabulary is complex, use very simple grammar.";
const LEVEL_A2: &str = "LANGUAGE LEVEL A2 (Elementary): Use simple sentences and common vocabulary. Basic past and future tenses okay. Keep explanations brief and concrete.";
const LEVEL_B1: &str = "LANGUAGE LEVEL B1 (Intermediate): Use standard vocabulary and grammar. Can introduce idioms with explanation. Normal conversational pace.";
const LEVEL_B2: &str = "LANGUAGE LEVEL B2 (Upper Intermediate): Use varied vocabulary including some abstract concepts. Complex sentences okay. Can use idioms naturally.";
const LEVEL_C1: &str = "LANGUAGE LEVEL C1 (Advanced): Use sophisticated vocabulary and nuanced expressions. Can discuss abstract topics. Full range of tenses and moods.";
const LEVEL_C2: &str = "LANGUAGE LEVEL C2 (Proficient): Speak as you would to a native speaker. Full complexity, subtlety, and cultural references are appropriate.";

const JLPT_N5: &str = "LANGUAGE LEVEL N5 (Beginner): Use only hiragana, katakana, and basic kanji (~100 characters). Very simple sentences with basic vocabulary. Speak slowly and clearly. Limited to basic daily expressions and greetings.";
const JLPT_N4: &str = "LANGUAGE LEVEL N4 (Elementary): Use basic vocabulary and kanji (~300 characters). Simple sentences about familiar daily topics. Speak slowly. Basic past and future tenses okay.";
const JLPT_N3: &str = "LANGUAGE LEVEL N3 (Intermediate): Everyday Japanese at near-natural speed. Can use context to understand slightly difficult expressions. Standard vocabulary and grammar for daily situations.";
const JLPT_N2: &str = "LANGUAGE LEVEL N2 (Upper Intermediate): Varied vocabulary including abstract concepts. Can follow narratives and understand writer/speaker intent. Natural speech speed. Newspaper articles and general topics accessible.";
const JLPT_N1: &str = "LANGUAGE LEVEL N1 (Advanced): Full sophistication appropriate. Can handle editorials, critiques, and academic materials. Native-level complexity, nuance, and cultural references are appropriate.";

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
