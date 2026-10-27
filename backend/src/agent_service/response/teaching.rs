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

// Adapted from the Council of Europe's CEFR global scale and Companion Volume (2020),
// especially overall oral interaction and goal-oriented co-operation (pp. 72, 77):
// https://www.coe.int/en/web/common-european-framework-reference-languages/table-1-cefr-3.3-common-reference-levels-global-scale
// https://rm.coe.int/cefr-companion-volume-with-new-descriptors-2020/16809ea0d4
const LEVEL_A1: &str = "LANGUAGE LEVEL A1 (Basic user): Use familiar phrases for concrete needs and personal details. Keep questions and statements simple; repeat or rephrase when needed. Support the exchange without assuming the learner can sustain it independently.";
const LEVEL_A2: &str = "LANGUAGE LEVEL A2 (Basic user): Use common expressions for familiar routines, needs and short social exchanges. Support straightforward information exchange, simple descriptions and practical suggestions. Keep the situation clear and help sustain the conversation.";
const LEVEL_B1: &str = "LANGUAGE LEVEL B1 (Independent user): Use clear, connected language about familiar interests, experiences and plans. Allow opinions with brief reasons, comparisons and explanations of practical problems. Rephrase unfamiliar expressions when needed.";
const LEVEL_B2: &str = "LANGUAGE LEVEL B2 (Independent user): Support spontaneous discussion of concrete and abstract topics, detailed explanations and reasoned viewpoints with trade-offs. Use varied language while keeping the argument clear; adapt specialist vocabulary to the topic and learner.";
const LEVEL_C1: &str = "LANGUAGE LEVEL C1 (Proficient user): Allow complex topics, implicit meaning and flexible social or professional expression. Use precise vocabulary and well-connected arguments. Match complexity to the conversation rather than making every reply elaborate.";
const LEVEL_C2: &str = "LANGUAGE LEVEL C2 (Proficient user): Allow fine distinctions of meaning, idiomatic expression and precise reformulation of complex ideas. Match register and nuance to the situation; clarity and relevance still matter.";

// JLPT levels describe comprehension, not certified speaking/writing proficiency.
// These are input-difficulty guides; no official exhaustive item lists are published:
// https://www.jlpt.jp/e/about/levelsummary.html
// https://www.jlpt.jp/e/about/cefr_reference.html
// https://www.jlpt.jp/e/faq/index.html
const JLPT_N5: &str = "LANGUAGE LEVEL N5: Target comprehension of some basic Japanese. Use familiar everyday expressions and short, explicit exchanges. Keep vocabulary and constructions basic; use common kana and basic kanji subject to the selected script setting.";
const JLPT_N4: &str = "LANGUAGE LEVEL N4: Target comprehension of basic Japanese on familiar daily topics. Use accessible vocabulary and connected everyday exchanges with clear context. Keep unfamiliar language easy to clarify.";
const JLPT_N3: &str = "LANGUAGE LEVEL N3: Target partial comprehension of everyday Japanese: coherent conversations, concrete content and main points. Use rephrasing to make harder passages accessible; keep relationships and references clear.";
const JLPT_N2: &str = "LANGUAGE LEVEL N2: Target comprehension across everyday and some broader settings. Allow coherent discussions, general commentary and viewpoints; make the main ideas and relationships followable. Do not assume specialist knowledge.";
const JLPT_N1: &str = "LANGUAGE LEVEL N1: Target comprehension of complex or abstract Japanese across varied settings, including logical structure and speaker intent. Use natural detail and nuance where relevant; do not force rare language into ordinary chat.";

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

// Sentence-length limits are app-specific prompting controls, not proficiency definitions.
const LEVEL_A1_CHECKLIST: &[&str] = &[
    "The immediate meaning and next conversational move are clear",
    "Repetition or rephrasing supports understanding where needed",
    "Sentences are 4-5 words each",
];

const LEVEL_A2_CHECKLIST: &[&str] = &[
    "The exchange is concrete and manageable with support",
    "Sentences are 5-8 words each",
];

const LEVEL_B1_CHECKLIST: &[&str] = &[
    "Ideas are connected and reasons are easy to follow",
    "Unfamiliar language can be clarified without derailing the selected mode",
];

const LEVEL_B2_CHECKLIST: &[&str] = &[
    "Detail and argument serve the user's topic",
    "Difficulty reflects language proficiency, not presumed subject expertise",
];

const LEVEL_C1_CHECKLIST: &[&str] =
    &["Complexity and implicit meaning are appropriate to the exchange"];

const LEVEL_C2_CHECKLIST: &[&str] = &["Nuance is precise without unnecessary complexity"];

const JLPT_N5_CHECKLIST: &[&str] = &[
    "Basic language respects the selected dialect, formality and script",
    "Sentences are 4-5 words each",
];

const JLPT_N4_CHECKLIST: &[&str] = &[
    "Everyday meaning remains accessible in the selected dialect and script",
    "The learner's replies guide support; JLPT does not certify interaction skills",
    "Sentences are 5-8 words each",
];

const JLPT_N3_CHECKLIST: &[&str] = &[
    "The main point stays accessible even when individual expressions are unfamiliar",
    "Support follows the learner's replies rather than presumed speaking fluency",
];

const JLPT_N2_CHECKLIST: &[&str] =
    &["Broader comprehension is not treated as mastery of every topic"];

const JLPT_N1_CHECKLIST: &[&str] =
    &["Advanced comprehension does not imply native or uniformly advanced productive ability"];

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
