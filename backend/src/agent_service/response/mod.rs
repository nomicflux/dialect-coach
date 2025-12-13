use dialect_coach_shared::{
    Explained, Exploratory, Formality, LanguageLevel, LanguageOption, LearningGoal, Mistake,
    TeachingMode, Translated, UserGender, DialectWithFeatures,
};
use rig::completion::Message as RigMessage;
use std::sync::Arc;

use crate::embedding_service::EmbeddingService;
use crate::qdrant_service::QdrantService;
use crate::rag_config::RAGConfig;


mod config;

use super::provider::CompletionAgent;
mod speaker;

mod teaching;

mod examples;

mod system_content;
mod parsing;


mod retrieval;

mod generation;

/// Parameters for generating a response
pub struct GenerateResponseParams<'a> {
    pub user_message: &'a str,
    pub dialect: DialectWithFeatures,
    pub formality: Formality,
    pub teaching_mode: TeachingMode,
    pub conversation_history: &'a [RigMessage],
    pub learning_goals: &'a [LearningGoal],
    pub rag_config: &'a RAGConfig,
    pub past_mistakes: &'a [Mistake],
    pub past_explained: &'a [Explained],
    pub past_translated: &'a [Translated],
    pub past_exploratory: &'a [Exploratory],
    pub user_gender: UserGender,
    pub language_option: &'a Option<LanguageOption>,
    pub active_plan: Option<&'a dialect_coach_shared::LanguagePlan>,
    pub language_level: LanguageLevel,
}

pub struct ResponseContext {
    pub response_agent: Arc<dyn CompletionAgent>,
    pub learning_agent: Arc<dyn CompletionAgent>,
    pub qdrant: Arc<QdrantService>,
    pub embeddings: Arc<EmbeddingService>,
}

impl ResponseContext {}

#[cfg(test)]
mod tests {
    use dialect_coach_shared::models::dialect::dialect_features;
    use dialect_coach_shared::{
        AgentUsage, Dialect, Formality, TeachingMode, LearningGoal, PastLearningItems, UserGender, LanguageLevel,
    };
    use crate::agent_service::response::system_content::build_system_content;
    use crate::agent_service::response::parsing::{build_simple_completion_request, sanitize_simple_json_response};


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
            LanguageLevel::B1,
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
            LanguageLevel::A2,
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
    fn test_build_simple_completion_request_uses_zero_temperature() {
        let request = build_simple_completion_request("sys", "prompt", &[]);
        assert_eq!(request.temperature, 0.0);
        assert_eq!(request.max_tokens, 1024);
        assert_eq!(request.preamble, "sys");
        assert_eq!(request.prompt, "prompt");
    }

    #[test]
    fn test_sanitize_simple_json_response_strips_markdown_and_validates() {
        let wrapped = "```json\n{\"response\":\"hola\"}\n```";
        let sanitized = sanitize_simple_json_response(wrapped).unwrap();
        let value: serde_json::Value = serde_json::from_str(&sanitized).unwrap();
        assert_eq!(value["response"], "hola");
        assert!(!sanitized.contains("```"));
    }

    #[test]
    fn test_build_system_content_with_active_plan() {
        use crate::agent_service::response::system_content::build_plan_system_content;
        use dialect_coach_shared::models::Dialect;
        use dialect_coach_shared::models::UserState;
        use dialect_coach_shared::models::learning_item::{LearningItem, LearningItemType};
        use dialect_coach_shared::models::{
            LanguagePlan, PlanContent, PlanStep, StepType, Translated,
        };
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
        state.language_plans.push(plan.clone());
        state.active_plan_id = Some(plan.id);

        // Act
        let system_content = build_plan_system_content(&Some(&plan));

        // Assert
        assert!(system_content.contains("RELEVANT LEARNING CONTENT"));
        assert!(system_content.contains("hello"));
        assert!(system_content.contains("Naturally incorporate"));
    }

    #[test]
    fn test_build_plan_system_content() {
        use crate::agent_service::response::system_content::build_plan_system_content;
        use dialect_coach_shared::models::learning_item::{LearningItem, LearningItemType};
        use dialect_coach_shared::models::plan::{PlanContent, PlanStep, StepType};
        use dialect_coach_shared::models::{Dialect, Explained, Translated};

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

        let plan = dialect_coach_shared::LanguagePlan::new(
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
    fn test_language_level_instruction_covers_all_levels() {
        use crate::agent_service::response::teaching::language_level_instruction;
        let levels = [
            LanguageLevel::A1,
            LanguageLevel::A2,
            LanguageLevel::B1,
            LanguageLevel::B2,
            LanguageLevel::C1,
            LanguageLevel::C2,
        ];
        for level in levels {
            let instruction = language_level_instruction(level);
            assert!(!instruction.is_empty());
            assert!(instruction.contains("LANGUAGE LEVEL"));
        }
    }

    #[test]
    fn test_build_plan_system_content_review_step() {
        use crate::agent_service::response::system_content::build_plan_system_content;
        use dialect_coach_shared::models::learning_item::{LearningItem, LearningItemType};
        use dialect_coach_shared::models::plan::{PlanContent, PlanStep, StepType};
        use dialect_coach_shared::models::{Dialect, Translated};

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

        let plan = dialect_coach_shared::LanguagePlan {
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
}
