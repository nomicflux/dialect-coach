use dialect_coach_shared::UserState;
use dialect_coach_shared::models::{
    AgentResponse, Dialect, Formality, Language, LanguagePlan, Message, MessageMetadata, PlanStep,
    Quest, StepType, TeachingMode,
};
use std::sync::Arc;
use uuid::Uuid;

pub fn agent_message(text: &str) -> Message {
    let metadata = MessageMetadata::at_now(
        Formality::Informal,
        TeachingMode::Immersive,
        Language::Spanish,
        Dialect::SpanishArgentinian,
        Uuid::new_v4(),
    );
    Message::agent_message(AgentResponse::from(text), metadata, None)
}

pub fn user_with_plan_step(title: &str) -> UserState {
    let mut user = UserState::new(Uuid::new_v4());
    let step = PlanStep::new(
        1,
        title.into(),
        StepType::Review {
            review_step_ids: vec![],
        },
        "Review".into(),
    );
    let dialect = user.active_branch_dialect();
    let plan = LanguagePlan::new("Test plan".into(), dialect, None, vec![step]);
    Arc::make_mut(&mut user.branches)[0].active_plan_id = Some(plan.id);
    Arc::make_mut(&mut user.language_plans).push(plan);
    user
}

pub fn quest(item_id: Uuid, dialect: Dialect, description: &str) -> Quest {
    Quest {
        id: format!("quest_{item_id}"),
        description: description.into(),
        completed: false,
        dialect,
    }
}
