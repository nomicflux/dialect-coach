use dialect_coach_shared::{
    DialectWithFeatures, Formality, LanguageLevel, LanguageOption, LearningGoal,
    PastLearningItems, TeachingMode, UserGender,
};
use dialect_coach_shared::models::learning_item::{LearningItem, LearningItemType};
use dialect_coach_shared::models::plan::{LanguagePlan, PlanStep, StepType};

use crate::agent_service::language_instructions::build_language_instruction;
use crate::agent_service::util::{format_learning_items_context, learning_goals_section, JSON_OUTPUT_INSTRUCTION};
use super::config::{CONTENT_FILTERING_DIRECTIVES, RESPONSE_JSON_OUTPUT_FORMAT};
use super::speaker::{extract_gender_from_dialect, mimic_instruction, speaker_desc};
use super::teaching::{language_level_instruction, response_teaching_desc};

fn collect_plan_items<'a>(step: &'a PlanStep, plan: &'a LanguagePlan) -> Vec<&'a LearningItem> {
    match &step.step_type {
        StepType::Learning { content } => content.items.iter().collect(),
        StepType::Review { review_step_ids } => review_step_ids
            .iter()
            .filter_map(|id| plan.steps.iter().find(|s| s.id == *id))
            .filter_map(|step| match &step.step_type {
                StepType::Learning { content } => Some(content),
                _ => None,
            })
            .flat_map(|content| content.items.iter())
            .collect(),
    }
}

fn format_learning_items(items: &[&LearningItem]) -> String {
    let mut content = String::new();
    for item in items {
        match &item.item {
            LearningItemType::Translation(t) => {
                content.push_str(&format!("- Vocab word to use: {}\n", t.translated_to));
            }
            LearningItemType::Explanation(e) => {
                content.push_str(&format!(
                    "- Grammatical point to incorporate ({}): {}\n",
                    e.new_phrase, e.explanation
                ));
            }
            _ => {}
        }
    }
    content
}

fn get_goal_instruction(step_type: &StepType) -> &'static str {
    match step_type {
        StepType::Learning { .. } => {
            "Your Goal: Naturally incorporate the Provided Step Materials into your own speech to demonstrate them. Do NOT explicitly teach, list the items, or ask the user to use them. Just chat naturally using the target vocabulary/grammar."
        }
        StepType::Review { .. } => {
            "Your Goal: This is a REVIEW step. The user has learned the listed materials in previous steps. Verify the user remembers them by using them in context or asking questions that require the user to use them. Do not spoon-feed answers. Challenge them."
        }
    }
}

pub(crate) fn build_plan_system_content(plan: &Option<&LanguagePlan>) -> String {
    if let Some(plan) = plan
        && let Some(step) = plan.steps.get(plan.current_step_index)
    {
        let items_to_display = collect_plan_items(step, plan);
        let mut content = String::new();

        if !items_to_display.is_empty() {
            match step.step_type {
                StepType::Learning { .. } => content.push_str("\nRELEVANT LEARNING CONTENT:\n"),
                StepType::Review { .. } => {
                    content.push_str("\nREVIEW MATERIALS (FROM PREVIOUS STEPS):\n")
                }
            }
            content.push_str(&format_learning_items(&items_to_display));
        }

        let goal_instruction = get_goal_instruction(&step.step_type);
        return format!(
            "\n\n# ACTIVE LANGUAGE PLAN\nYou are guiding the user through the plan: \"{}\".\n\
                Current Step: {}\n\
                Instructions: {}\n\
                {}\
                {}",
            plan.title, step.title, step.instructions, content, goal_instruction
        );
    }

    String::new()
}

fn build_lang_section(language_instr: &str) -> String {
    if !language_instr.is_empty() {
        format!("\n\nLANGUAGE INSTRUCTION: {}", language_instr)
    } else {
        String::new()
    }
}

fn build_debug_system_content(
    role_desc: &str,
    user_gender_str: &str,
    lang_section: &str,
    level_instruction: &str,
    goals_section: &str,
    learning_items_context: &str,
) -> String {
    format!(
        r#"# YOUR ROLE
            {}

            USER GENDER: The student you're speaking with is {}. Use gender-appropriate forms when teaching grammar and vocabulary that have gendered aspects.{}

            {}
            # CRITICAL RULES
            1. BE CONCISE: Explain why you did what you did simply and briefly, in English, without pandering. This will be within the "response" field of the required JSON format.
            2. ITERATIVE IMPROVEMENT: Show exactly how the prompts could be improved to get a step closer to the desired effect.
            {}
            {}
            {}
            Now respond to the user's message technically."#,
        role_desc,
        user_gender_str,
        lang_section,
        level_instruction,
        goals_section,
        learning_items_context,
        JSON_OUTPUT_INSTRUCTION
    )
}

struct NormalSystemParams<'a> {
    role_desc: &'a str,
    user_gender_str: &'a str,
    lang_section: &'a str,
    level_instruction: &'a str,
    teaching_rules: &'a str,
    goals_section: &'a str,
    learning_items_context: &'a str,
    plan_instr: &'a str,
    formality_label: &'a str,
    dialect_name: &'a str,
    has_corpus: bool,
}

fn build_normal_system_content(params: NormalSystemParams) -> String {
    format!(
        "{}\n\n\
            # YOUR ROLE
            {}

            USER GENDER: The student you're speaking with is {}. Use gender-appropriate forms when teaching grammar and vocabulary that have gendered aspects.{}

            {}
            # CRITICAL RULES
            {}
            2. MAINTAIN FORMALITY: Match the {} formality level shown in the examples
            {}
            {}
            {}
            {}
            # OUTPUT FORMAT REQUIRED
            {}
            {}

            Now respond to the user's message as if you were in a natural chatroom with a friend, as a local {} speaker would, in the response field of the required JSON format. You MUST ALWAYS respond - NEVER indicate the conversation has ended. If it seems to have ended, provide a follow-up question or new topic. The response field must be non-empty. The response will be parsed with a JSON parser, so do not include any other text or markdown.",
        CONTENT_FILTERING_DIRECTIVES,
        params.role_desc,
        params.user_gender_str,
        params.lang_section,
        params.level_instruction,
        mimic_instruction(params.has_corpus),
        params.formality_label.to_lowercase(),
        params.teaching_rules,
        params.goals_section,
        params.learning_items_context,
        params.plan_instr,
        JSON_OUTPUT_INSTRUCTION,
        RESPONSE_JSON_OUTPUT_FORMAT,
        params.dialect_name
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_system_content(
    dialect: DialectWithFeatures,
    formality: Formality,
    teaching_mode: TeachingMode,
    learning_goals: &[LearningGoal],
    past_learning_items: &PastLearningItems,
    user_gender: UserGender,
    language_option: &Option<LanguageOption>,
    active_plan: &Option<&LanguagePlan>,
    language_level: LanguageLevel,
) -> String {
    let formality_label = match formality {
        Formality::Formal => "FORMAL",
        Formality::ProfessionalCasual => "PROFESSIONAL-CASUAL",
        Formality::Informal => "INFORMAL",
        Formality::Slang => "SLANG",
    };

    let gender = extract_gender_from_dialect(&dialect);
    let role_desc = speaker_desc(&dialect.dialect, &formality, &gender);
    let teaching_rules = response_teaching_desc(&teaching_mode);
    let goals_section = learning_goals_section(learning_goals);
    let learning_items_context = format_learning_items_context(
        &past_learning_items.mistakes,
        &past_learning_items.explained,
        &past_learning_items.translated,
        &past_learning_items.exploratory,
    );

    let user_gender_str = match user_gender {
        UserGender::Male => "male",
        UserGender::Female => "female",
        UserGender::NonBinary => "non-binary",
    };

    let level_instruction = language_level_instruction(language_level);
    let language_instr = build_language_instruction(language_option);
    let plan_instr = build_plan_system_content(active_plan);
    let lang_section = build_lang_section(&language_instr);

    if teaching_mode == TeachingMode::Debug {
        build_debug_system_content(
            &role_desc,
            user_gender_str,
            &lang_section,
            level_instruction,
            &goals_section,
            &learning_items_context,
        )
    } else {
        build_normal_system_content(NormalSystemParams {
            role_desc: &role_desc,
            user_gender_str,
            lang_section: &lang_section,
            level_instruction,
            teaching_rules: &teaching_rules,
            goals_section: &goals_section,
            learning_items_context: &learning_items_context,
            plan_instr: &plan_instr,
            formality_label,
            dialect_name: dialect.dialect.name(),
            has_corpus: dialect.has_corpus,
        })
    }
}
