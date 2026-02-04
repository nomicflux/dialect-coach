use dialect_coach_shared::models::learning_item::{LearningItem, LearningItemType};
use dialect_coach_shared::models::plan::{LanguagePlan, PlanStep, StepType};
use dialect_coach_shared::models::{LanguageOption, needs_pronunciation_text};
use dialect_coach_shared::{
    DialectWithFeatures, Formality, LanguageLevel, LearningGoal, PastLearningItems, TeachingMode,
    UserGender,
};

use super::config::{CONTENT_FILTERING_DIRECTIVES, get_json_output_format};
use super::speaker::{extract_gender_from_dialect, mimic_instruction, speaker_desc};
use super::teaching::{
    language_level_instruction, level_checklist, mode_checklist, response_teaching_desc,
};
use crate::agent_service::language_instructions::{
    build_language_instruction, build_pronunciation_instruction,
};
use crate::agent_service::util::{
    JSON_OUTPUT_INSTRUCTION, format_learning_items_context, learning_goals_section,
};

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

fn format_checklist(items: &[&str]) -> String {
    items
        .iter()
        .map(|item| format!("- [ ] {}", item))
        .collect::<Vec<_>>()
        .join("\n")
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
    level_name: &'a str,
    has_corpus: bool,
    mode_checklist_formatted: &'a str,
    level_checklist_formatted: &'a str,
    pronunciation_instruction: &'a str,
    json_output_format: &'a str,
}

fn build_normal_system_content(params: NormalSystemParams) -> String {
    let pronunciation_section = if params.pronunciation_instruction.is_empty() {
        String::new()
    } else {
        format!("\n            {}", params.pronunciation_instruction)
    };

    format!(
        "{}\n\n\
            # YOUR ROLE
            {}

            USER GENDER: The student you're speaking with is {}. Use gender-appropriate forms when teaching grammar and vocabulary that have gendered aspects.{}

            {}
            # CRITICAL RULES
            {}
            2. MAINTAIN FORMALITY: Match the {} formality level shown in the examples
            3. Before generating your response, mentally check: 'Would a student at {} understand every word of this?' If not, simplify it immediately. Prioritize clarity over native nuance for this level.
            4. {}
            {}
            {}
            {}
            # OUTPUT FORMAT REQUIRED
            {}
            {}{}

            # SELF-CHECK BEFORE RESPONDING
            Draft your response, then verify each item. If ANY check fails, revise before outputting.

            ## Mode Requirements
            {}

            ## Level Requirements
            {}

            Now respond to the user's message as if you were in a natural chatroom with a friend, as a local {} speaker would when speaking to someone at level {}, in the response field of the required JSON format.
            You MUST ALWAYS respond - NEVER indicate the conversation has ended.
            If it seems to have ended, provide a follow-up question or new topic.
            The response field must be non-empty.
            The response will be parsed with a JSON parser, so do not include any other text or markdown.",
        CONTENT_FILTERING_DIRECTIVES,
        params.role_desc,
        params.user_gender_str,
        params.lang_section,
        params.level_instruction,
        mimic_instruction(params.has_corpus),
        params.formality_label.to_lowercase(),
        params.level_name,
        params.teaching_rules,
        params.goals_section,
        params.learning_items_context,
        params.plan_instr,
        JSON_OUTPUT_INSTRUCTION,
        params.json_output_format,
        pronunciation_section,
        params.mode_checklist_formatted,
        params.level_checklist_formatted,
        params.dialect_name,
        params.level_name
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
    let mode_checklist_formatted = format_checklist(mode_checklist(&teaching_mode));
    let level_checklist_formatted = format_checklist(level_checklist(language_level));

    let needs_pronunciation = needs_pronunciation_text(language_option);
    let pronunciation_instruction =
        build_pronunciation_instruction(language_option, dialect.dialect.name());
    let json_output_format = get_json_output_format(needs_pronunciation);

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
            level_name: language_level.name(),
            has_corpus: dialect.has_corpus,
            mode_checklist_formatted: &mode_checklist_formatted,
            level_checklist_formatted: &level_checklist_formatted,
            pronunciation_instruction: &pronunciation_instruction,
            json_output_format,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dialect_coach_shared::models::dialect::dialect_features;
    use dialect_coach_shared::models::learning_item::{LearningItem, LearningItemType};
    use dialect_coach_shared::models::plan::{PlanContent, PlanStep, StepType};
    use dialect_coach_shared::models::{Explained, Translated};
    use dialect_coach_shared::{CefrLevel, Dialect, PastLearningItems, UserGender};

    #[test]
    fn test_build_system_content_debug() {
        let dialect = Dialect::SpanishMexican;
        let formality = Formality::Informal;
        let teaching_mode = TeachingMode::Debug;
        let learning_goals = vec![];
        let past_learning_items = PastLearningItems::default();

        let content = build_system_content(
            dialect_features(dialect),
            formality,
            teaching_mode,
            &learning_goals,
            &past_learning_items,
            UserGender::NonBinary,
            &None,
            &None,
            LanguageLevel::Cefr(CefrLevel::B1),
        );

        assert!(content.contains("# YOUR ROLE"));
        assert!(content.contains("BE CONCISE"));
        assert!(content.contains("ITERATIVE IMPROVEMENT"));
        assert!(content.contains("LANGUAGE LEVEL"));
        assert!(content.contains("technically"));
    }

    #[test]
    fn test_build_system_content_normal() {
        let dialect = Dialect::SpanishArgentinian;
        let formality = Formality::Informal;
        let teaching_mode = TeachingMode::Immersive;
        let learning_goals = vec![LearningGoal {
            goal: "Goal 1".to_string(),
            dialect: Dialect::SpanishArgentinian,
        }];
        let past_learning_items = PastLearningItems::default();

        let content = build_system_content(
            dialect_features(dialect),
            formality,
            teaching_mode,
            &learning_goals,
            &past_learning_items,
            UserGender::NonBinary,
            &None,
            &None,
            LanguageLevel::Cefr(CefrLevel::A2),
        );

        assert!(content.contains("# YOUR ROLE"));
        assert!(content.contains("MIMIC THE PATTERNS"));
        assert!(content.contains("MAINTAIN FORMALITY"));
        assert!(content.contains("informal"));
        assert!(content.contains("# LEARNING GOALS"));
        assert!(content.contains("Goal 1"));
        assert!(content.contains("LANGUAGE LEVEL"));
    }

    #[test]
    fn test_build_system_content_with_active_plan() {
        use dialect_coach_shared::models::Dialect;
        use dialect_coach_shared::models::UserState;
        use std::sync::Arc;
        use uuid::Uuid;

        let mut content = PlanContent::default();
        content.items.push(LearningItem::new(
            LearningItemType::Translation(Translated::new(
                "hola".to_string(),
                "hello".to_string(),
                None,
            )),
            Dialect::SpanishMexican,
        ));

        let current_step = PlanStep::new(
            1,
            "Intro".to_string(),
            StepType::Learning { content },
            "Learn basic greetings".to_string(),
        );

        let plan = LanguagePlan::new(
            "Test Plan".to_string(),
            Dialect::SpanishMexican,
            None,
            vec![current_step],
        );

        let mut state = UserState::new(Uuid::new_v4());
        Arc::make_mut(&mut state.language_plans).push(plan.clone());
        if let Some(branch) = Arc::make_mut(&mut state.branches)
            .iter_mut()
            .find(|b| b.id == state.active_branch_id)
        {
            branch.active_plan_id = Some(plan.id);
        }

        // Act
        let system_content = build_plan_system_content(&Some(&plan));

        // Assert
        assert!(system_content.contains("RELEVANT LEARNING CONTENT"));
        assert!(system_content.contains("hello"));
        assert!(system_content.contains("Naturally incorporate"));
    }

    #[test]
    fn test_build_plan_system_content() {
        let mut content = PlanContent::default();

        content.items.push(LearningItem::new(
            LearningItemType::Translation(Translated::new(
                "hola".to_string(),
                "hello".to_string(),
                None,
            )),
            Dialect::SpanishMexican,
        ));
        content.items.push(LearningItem::new(
            LearningItemType::Explanation(Explained::new(
                "que onda".to_string(),
                "what's up".to_string(),
            )),
            Dialect::SpanishMexican,
        ));

        let step = PlanStep::new(
            1,
            "Test Step".to_string(),
            StepType::Learning { content },
            "Use these words".to_string(),
        );

        let plan = LanguagePlan::new(
            "Test Plan".to_string(),
            Dialect::SpanishMexican,
            None,
            vec![step],
        );

        let prompt = build_plan_system_content(&Some(&plan));

        // Check for key prompt elements
        assert!(
            prompt.contains("ACTIVE LANGUAGE PLAN"),
            "Should identify as active plan"
        );
        assert!(prompt.contains("Test Step"), "Should contain step title");
        assert!(
            prompt.contains("Use these words"),
            "Should contain instructions"
        );

        // Check content rendering
        assert!(
            prompt.contains("hello"),
            "Should contain translation target"
        );
        assert!(
            prompt.contains("que onda"),
            "Should contain explanation phrase"
        );

        // Check new goal instruction
        assert!(
            prompt.contains("Naturally incorporate"),
            "Should contain new goal instruction"
        );
        assert!(
            !prompt.contains("correct them gently"),
            "Should NOT contain old goal instruction"
        );
    }

    #[test]
    fn test_build_plan_system_content_review_step() {
        // Create a learning step with content
        let mut content1 = PlanContent::default();
        content1.items.push(LearningItem::new(
            LearningItemType::Translation(Translated::new(
                "gracias".to_string(),
                "thanks".to_string(),
                None,
            )),
            Dialect::SpanishMexican,
        ));

        let step1 = PlanStep::new(
            1,
            "Learning Step".to_string(),
            StepType::Learning { content: content1 },
            "Learn this".to_string(),
        );

        // Create a review step referencing step1
        let step2 = PlanStep::new(
            2,
            "Review Step".to_string(),
            StepType::Review {
                review_step_ids: vec![step1.id],
            },
            "Review this".to_string(),
        );

        let plan = LanguagePlan {
            id: uuid::Uuid::new_v4(),
            title: "Test Plan".to_string(),
            dialect: Dialect::SpanishMexican,
            description: None,
            steps: vec![step1, step2],
            current_step_index: 1, // Set to Review step
            status: dialect_coach_shared::models::plan::PlanStatus::InProgress,
            created_at: 0,
        };

        let prompt = build_plan_system_content(&Some(&plan));

        assert!(prompt.contains("Current Step: Review Step"));
        assert!(
            prompt.contains("REVIEW MATERIALS"),
            "Should identify materials as review materials"
        );
        assert!(
            prompt.contains("thanks"),
            "Should contain content from referenced step"
        );
        assert!(
            prompt.contains("The user has learned the listed materials"),
            "Should contain review-specific goal instruction"
        );
    }

    #[test]
    fn test_simplification_gate_contains_level_name() {
        let dialect = Dialect::SpanishMexican;
        let formality = Formality::Informal;
        let teaching_mode = TeachingMode::Immersive;
        let learning_goals = vec![];
        let past_learning_items = PastLearningItems::default();

        let content = build_system_content(
            dialect_features(dialect),
            formality,
            teaching_mode,
            &learning_goals,
            &past_learning_items,
            UserGender::NonBinary,
            &None,
            &None,
            LanguageLevel::Cefr(CefrLevel::A2),
        );

        assert!(content.contains("Would a student at A2 - Elementary understand"));
        assert!(!content.contains("Would a student at [LEVEL] understand"));
    }

    #[test]
    fn test_simplification_gate_contains_jlpt_level_name() {
        let dialect = Dialect::JapaneseTokyo;
        let formality = Formality::Informal;
        let teaching_mode = TeachingMode::Immersive;
        let learning_goals = vec![];
        let past_learning_items = PastLearningItems::default();

        let content = build_system_content(
            dialect_features(dialect),
            formality,
            teaching_mode,
            &learning_goals,
            &past_learning_items,
            UserGender::NonBinary,
            &None,
            &None,
            LanguageLevel::Jlpt(dialect_coach_shared::JlptLevel::N5),
        );

        assert!(content.contains("Would a student at N5 - Beginner understand"));
    }
}
