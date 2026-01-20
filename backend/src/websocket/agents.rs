use rig::completion::{
    Message as RigMessage, message::AssistantContent, message::Text, message::UserContent,
};
use rig::one_or_many::OneOrMany;
use std::collections::HashSet;
use tokio::sync::mpsc;
use uuid::Uuid;

use dialect_coach_shared::models::dialect::dialect_features;
use dialect_coach_shared::{
    AIActionRequest, AgentResponse, AgentUsageStats, Dialect, Gender, LanguagePlan, LearningItem,
    LearningItemType, Message, MessageContent, MessageMetadata, UserGender, UserMessageWithContext,
    UserState,
};

use crate::rag_config::RAGConfig;

use super::{errors, send, user, user_state};
use crate::AppState;
use crate::agent_service::response::GenerateResponseParams;

pub fn convert_to_rig_message(message: &Message) -> RigMessage {
    match message.content.clone() {
        MessageContent::UserMessage { content } => RigMessage::User {
            content: OneOrMany::one(UserContent::Text(Text {
                text: content.clone(),
            })),
        },
        MessageContent::AgentMessage { content } => RigMessage::Assistant {
            id: None,
            content: OneOrMany::one(AssistantContent::Text(Text {
                text: content.response.clone(),
            })),
        },
    }
}

pub fn build_context_from_messages(messages: &[Message]) -> Vec<RigMessage> {
    messages.iter().map(convert_to_rig_message).collect()
}

pub fn create_agent_response_message(
    agent_response: AgentResponse,
    metadata: MessageMetadata,
    parent_id: Uuid,
) -> Message {
    Message::agent_message(agent_response, metadata, Some(parent_id))
}

pub async fn handle_agent_success(
    _state: &AppState,
    parsed_msg: &Message,
    agent_response: AgentResponse,
    tx: &mpsc::UnboundedSender<String>,
) -> Result<(), String> {
    let response_msg =
        create_agent_response_message(agent_response, parsed_msg.metadata.clone(), parsed_msg.id);

    send::serialize_and_send(&response_msg, tx)?;
    Ok(())
}

pub async fn handle_agent_error(
    parsed_msg: &Message,
    error: anyhow::Error,
    tx: &mpsc::UnboundedSender<String>,
) -> Result<(), String> {
    let error_msg = errors::create_error_message(
        format!("Error generating response: {}", error),
        parsed_msg.metadata.clone(),
    );

    send::serialize_and_send(&error_msg, tx)?;
    Ok(())
}

async fn handle_parallel_agents_success(
    state: &AppState,
    user_state: dialect_coach_shared::UserState,
    response_result: Result<AgentResponse, anyhow::Error>,
    analysis_result: Result<dialect_coach_shared::AgentAnalysis, anyhow::Error>,
    usage_stats: &AgentUsageStats,
    now: i64,
) -> Result<AgentResponse, anyhow::Error> {
    match (response_result, analysis_result) {
        (Ok(mut agent_response), Ok(analysis)) => {
            agent_response.analysis = Some(analysis);
            user_state::update_and_save_usage(state, user_state, usage_stats, now).await;
            Ok(agent_response)
        }
        (Ok(agent_response), Err(e)) => {
            tracing::error!("Analysis agent failed: {}", e);
            user_state::update_and_save_usage(state, user_state, usage_stats, now).await;
            Ok(agent_response)
        }
        (Err(e), _) => {
            user_state::update_and_save_usage(state, user_state, usage_stats, now).await;
            Err(e)
        }
    }
}

fn has_learning_items(msg_with_context: &UserMessageWithContext) -> bool {
    !msg_with_context.past_mistakes.is_empty()
        || !msg_with_context.past_explained.is_empty()
        || !msg_with_context.past_translated.is_empty()
        || !msg_with_context.past_exploratory.is_empty()
}

fn build_response_params<'a>(
    user_text: &'a str,
    msg_with_context: &'a UserMessageWithContext,
    dialect: Dialect,
    history_vec: &'a [RigMessage],
    rag_config: &'a RAGConfig,
) -> GenerateResponseParams<'a> {
    let formality = msg_with_context.message.metadata.formality;
    let teaching_mode = msg_with_context.message.metadata.teaching_mode;

    GenerateResponseParams {
        user_message: user_text,
        dialect: dialect_features(dialect),
        formality,
        teaching_mode,
        conversation_history: history_vec,
        learning_goals: &msg_with_context.learning_goals,
        rag_config,
        past_mistakes: &msg_with_context.past_mistakes,
        past_explained: &msg_with_context.past_explained,
        past_translated: &msg_with_context.past_translated,
        past_exploratory: &msg_with_context.past_exploratory,
        user_gender: msg_with_context.user_gender,
        language_option: &msg_with_context.language_option,
        active_plan: msg_with_context.active_plan.as_ref(),
        language_level: msg_with_context.language_level,
    }
}

async fn run_agents_with_analysis(
    state: &AppState,
    params: &GenerateResponseParams<'_>,
    msg_with_context: &UserMessageWithContext,
    dialect: Dialect,
    user_state: dialect_coach_shared::UserState,
    now: i64,
) -> Result<AgentResponse, anyhow::Error> {
    // Use learning items from msg_with_context - frontend has already filtered by dialect
    tracing::info!(
        "Running agents in parallel: {} mistakes, {} explained, {} translated, {} exploratory items",
        msg_with_context.past_mistakes.len(),
        msg_with_context.past_explained.len(),
        msg_with_context.past_translated.len(),
        msg_with_context.past_exploratory.len()
    );

    let user_text = msg_with_context.message.get_content();
    let active_plan = user_state.active_plan();
    let all_items = combine_plan_and_user_items(&user_state.learning_items, active_plan.as_ref());
    let (m, e, t, x) = extract_learning_items(&all_items, dialect);

    let ((response_result, response_usage, learning_usage), (analysis_result, analysis_usage)) = tokio::join!(
        state.agent.generate_response(params),
        state
            .agent
            .generate_analysis(crate::agent_service::analysis::AnalysisRequestParams {
                dialect,
                msg: &user_text,
                mistakes: &m,
                explained: &e,
                translated: &t,
                exploratory: &x,
                language_option: &msg_with_context.language_option,
            })
    );

    let usage_stats = AgentUsageStats {
        response_usage,
        learning_usage,
        analysis_usage,
    };

    handle_parallel_agents_success(
        state,
        user_state,
        response_result,
        analysis_result,
        &usage_stats,
        now,
    )
    .await
}

async fn run_agents_parallel(
    state: &AppState,
    msg_with_context: &UserMessageWithContext,
    dialect: Dialect,
    history_vec: &[RigMessage],
) -> Result<AgentResponse, anyhow::Error> {
    let user_text = msg_with_context.message.get_content();
    user::validate_user_message(&user_text)?;

    let has_learning_items = has_learning_items(msg_with_context);
    let teaching_mode = msg_with_context.message.metadata.teaching_mode;
    tracing::info!(
        "run_agents_parallel: has_learning_items={}, teaching_mode={:?}, past_mistakes={}, past_explained={}, past_translated={}, past_exploratory={}",
        has_learning_items,
        teaching_mode,
        msg_with_context.past_mistakes.len(),
        msg_with_context.past_explained.len(),
        msg_with_context.past_translated.len(),
        msg_with_context.past_exploratory.len()
    );
    let user_state = user_state::check_rate_limits(
        state,
        msg_with_context.user_id,
        teaching_mode,
        has_learning_items,
    )
    .await?;

    let rag_config = RAGConfig::default_config();
    let now = chrono::Utc::now().timestamp();
    let params = build_response_params(
        &user_text,
        msg_with_context,
        dialect,
        history_vec,
        &rag_config,
    );

    if !has_learning_items {
        tracing::info!("Taking run_response_only path (no learning items)");
        run_response_only(state, &params, user_state, now).await
    } else {
        tracing::info!("Taking run_agents_with_analysis path (has learning items)");
        run_agents_with_analysis(state, &params, msg_with_context, dialect, user_state, now).await
    }
}

async fn simple_call_and_respond(
    state: &AppState,
    prompt: &Message,
    tx: &mpsc::UnboundedSender<String>,
) -> Result<(), ()> {
    let response = state
        .agent
        .generate_simple_response("", prompt.as_str())
        .await;

    match response {
        Ok(agent_response) => {
            tracing::info!("Agent generated simple response",);
            handle_agent_success(state, prompt, agent_response, tx)
                .await
                .map_err(|e| {
                    tracing::error!("{}", e);
                })
        }
        Err(e) => {
            tracing::error!("Agent error: {}", e);
            let _ = handle_agent_error(prompt, e, tx).await;
            Ok(())
        }
    }
}

pub async fn call_agent_and_respond(
    state: &AppState,
    msg_with_context: &UserMessageWithContext,
    dialect: Dialect,
    history_vec: &[RigMessage],
    tx: &mpsc::UnboundedSender<String>,
) -> Result<(), ()> {
    let formality = msg_with_context.message.metadata.formality;
    let teaching_mode = msg_with_context.message.metadata.teaching_mode;

    tracing::info!(
        "Calling agent for dialect {} ({:?}, {:?}) with {} history messages",
        dialect.name(),
        formality,
        teaching_mode,
        history_vec.len()
    );

    match run_agents_parallel(state, msg_with_context, dialect, history_vec).await {
        Ok(agent_response) => {
            tracing::info!(
                "Agent generated response ({} chars)",
                agent_response.response.len()
            );
            handle_agent_success(state, &msg_with_context.message, agent_response, tx)
                .await
                .map_err(|e| {
                    tracing::error!("{}", e);
                })
        }
        Err(e) => {
            tracing::error!("Agent error: {}", e);
            let _ = handle_agent_error(&msg_with_context.message, e, tx).await;
            Ok(())
        }
    }
}

fn extract_agent_gender(dialect: Dialect) -> Gender {
    use dialect_coach_shared::TTSProviderType;

    let dialect_features = dialect_features(dialect);

    dialect_features
        .tts_voices
        .get(&TTSProviderType::ElevenLabs)
        .and_then(|voice| voice.as_ref().map(|v| v.gender))
        .unwrap_or(Gender::FemalePresenting)
}

fn format_gender_context(user_gender: UserGender, agent_gender: Gender) -> String {
    let user_gender_str = match user_gender {
        UserGender::Male => "male",
        UserGender::Female => "female",
        UserGender::NonBinary => "non-binary",
    };

    let agent_gender_str = match agent_gender {
        Gender::MalePresenting => "male",
        Gender::FemalePresenting => "female",
    };

    format!(
        "You are {} and the user is {}. Use appropriate gendered language.",
        agent_gender_str, user_gender_str
    )
}

fn build_action_context(
    action: &AIActionRequest,
    user_state: &UserState,
    dialect: Dialect,
    formality: dialect_coach_shared::Formality,
    user_gender: UserGender,
) -> String {
    let dialect_name = dialect.name();
    let formality_name = formality.name();
    let agent_gender = extract_agent_gender(dialect);
    let gender_context = format_gender_context(user_gender, agent_gender);

    match action {
        AIActionRequest::StartConversation => {
            format!(
                "[System: Please greet the user in {} with {} formality level. {}]",
                dialect_name, formality_name, gender_context
            )
        }
        AIActionRequest::ContinueBranch {
            parent_message_id, ..
        } => {
            let message: Option<Message> = user_state.msg_by_id(*parent_message_id);
            format!(
                "[System: Continue the conversation in {} with {} formality level. {}] {}",
                dialect_name,
                formality_name,
                gender_context,
                message
                    .iter()
                    .fold("[Start with a simple greeting]", |_, msg| msg.as_str())
            )
            .to_string()
        }
        AIActionRequest::ExplainMessage { message_id } => {
            let message: Option<Message> = user_state.msg_by_id(*message_id);
            format!(
                r#"[System: You are rewording the following prompt in {} for a beginner.
Reword this response in simpler terms, using fewer and more basic words.
Focus on ease of understanding for a beginning learner; do not change words if they are already basic enough.]
{}"#,
                dialect_name,
                message.iter().fold("[Ignore, no message given]", |_, msg| msg.as_str())
            ).to_string()
        }
        AIActionRequest::TranslateMessage { message_id } => {
            let message: Option<Message> = user_state.msg_by_id(*message_id);
            format!(
                r#"[System: Provide phrase-by-phrase translation of your previous response.
Format with newlines between phrases, like such:\n\
- <target phrase>: <English translation>\n\
- <next target phrase>: <next English transaction>\n
]
{}"#,
                message
                    .iter()
                    .fold("[Ignore, no message given]", |_, msg| msg.as_str())
            )
            .to_string()
        }
    }
}

pub async fn process_ai_action_request(
    state: &AppState,
    user_id: Uuid,
    session_id: Uuid,
    action: AIActionRequest,
    tx: &mpsc::UnboundedSender<String>,
) -> Result<(), ()> {
    tracing::info!(user_id = %user_id, "Processing AI action request: {:?}", action);

    let user_state = match user_state::load_user_state_for_action(state, user_id).await {
        Some(us) => us,
        None => {
            tracing::error!(user_id = %user_id, "User state not found for AI action");
            return Err(());
        }
    };

    let (dialect, formality, _teaching_mode, user_gender) = match &action {
        AIActionRequest::ContinueBranch { context, .. } => (
            context.dialect,
            context.formality,
            context.teaching_mode,
            context.user_gender,
        ),
        _ => {
            let settings = user_state.active_branch_settings();
            (
                user_state.current_dialect(),
                settings.formality,
                settings.teaching_mode,
                user_state.user_gender,
            )
        }
    };

    let context = build_action_context(&action, &user_state, dialect, formality, user_gender);
    let metadata = user_state::create_metadata_from_user_state(&user_state, session_id);
    let prompt_message = Message::user_message(context, metadata.clone(), None);

    match action {
        AIActionRequest::StartConversation | AIActionRequest::ContinueBranch { .. } => {
            match call_agent_for_conversation_action(state, &action, &user_state, user_id).await {
                Ok(agent_response) => {
                    tracing::info!("Agent generated conversation action response");
                    handle_agent_success(state, &prompt_message, agent_response, tx)
                        .await
                        .map_err(|e| {
                            tracing::error!("{}", e);
                        })
                }
                Err(e) => {
                    tracing::error!("Agent error: {}", e);
                    let _ = handle_agent_error(&prompt_message, e, tx).await;
                    Ok(())
                }
            }
        }
        AIActionRequest::ExplainMessage { .. } | AIActionRequest::TranslateMessage { .. } => {
            simple_call_and_respond(state, &prompt_message, tx).await
        }
    }
}

async fn run_response_only(
    state: &AppState,
    params: &GenerateResponseParams<'_>,
    user_state: dialect_coach_shared::UserState,
    now: i64,
) -> Result<AgentResponse, anyhow::Error> {
    let (result, response_usage, learning_usage) = state.agent.generate_response(params).await;
    let usage_stats = AgentUsageStats {
        response_usage,
        learning_usage,
        analysis_usage: Vec::new(),
    };
    user_state::update_and_save_usage(state, user_state, &usage_stats, now).await;
    result
}

fn clone_messages_up_to_index(messages: &[&Message], end_idx: usize) -> Vec<Message> {
    messages[..=end_idx]
        .iter()
        .map(|&msg| msg.clone())
        .collect()
}

fn get_branch_messages_up_to(user_state: &UserState, parent_message_id: Uuid) -> Vec<Message> {
    let all_messages = user_state.get_active_branch_messages();
    let parent_idx = all_messages
        .iter()
        .position(|msg| msg.id == parent_message_id);

    match parent_idx {
        Some(idx) => clone_messages_up_to_index(&all_messages, idx),
        None => {
            tracing::warn!(
                "Parent message {} not found in active branch",
                parent_message_id
            );
            Vec::new()
        }
    }
}

fn extract_learning_items(
    items: &[dialect_coach_shared::LearningItem],
    dialect: Dialect,
) -> (
    Vec<dialect_coach_shared::models::agent::Mistake>,
    Vec<dialect_coach_shared::models::agent::Explained>,
    Vec<dialect_coach_shared::models::agent::Translated>,
    Vec<dialect_coach_shared::models::agent::Exploratory>,
) {
    use dialect_coach_shared::models::learning_item::LearningItemType;

    let mut mistakes = Vec::new();
    let mut explained = Vec::new();
    let mut translated = Vec::new();
    let mut exploratory = Vec::new();

    for item in items.iter().filter(|i| i.dialect == dialect) {
        match &item.item {
            LearningItemType::Mistake(m) => mistakes.push(m.clone()),
            LearningItemType::Explanation(e) => explained.push(e.clone()),
            LearningItemType::Translation(t) => translated.push(t.clone()),
            LearningItemType::Exploration(e) => exploratory.push(e.clone()),
        }
    }

    (mistakes, explained, translated, exploratory)
}

fn get_item_id(item: &LearningItem) -> Uuid {
    match &item.item {
        LearningItemType::Mistake(m) => m.id,
        LearningItemType::Explanation(e) => e.id,
        LearningItemType::Translation(t) => t.id,
        LearningItemType::Exploration(e) => e.id,
    }
}

#[allow(clippy::collapsible_if)]
fn combine_plan_and_user_items(
    user_items: &[LearningItem],
    plan: Option<&LanguagePlan>,
) -> Vec<LearningItem> {
    let mut all_items = user_items.to_vec();
    if let Some(plan) = plan {
        if let Some(step) = plan.steps.get(plan.current_step_index) {
            if let dialect_coach_shared::StepType::Learning { content } = &step.step_type {
                let existing_ids: HashSet<Uuid> = all_items.iter().map(get_item_id).collect();
                for item in &content.items {
                    if !existing_ids.contains(&get_item_id(item)) {
                        all_items.push(item.clone());
                    }
                }
            }
        }
    }
    all_items
}

async fn call_agent_for_conversation_action(
    state: &AppState,
    action: &AIActionRequest,
    user_state: &UserState,
    user_id: Uuid,
) -> Result<AgentResponse, anyhow::Error> {
    let (dialect, formality, teaching_mode, user_gender) = match action {
        AIActionRequest::ContinueBranch { context, .. } => (
            context.dialect,
            context.formality,
            context.teaching_mode,
            context.user_gender,
        ),
        _ => {
            let settings = user_state.active_branch_settings();
            (
                user_state.current_dialect(),
                settings.formality,
                settings.teaching_mode,
                user_state.user_gender,
            )
        }
    };

    user_state::check_rate_limits(state, user_id, teaching_mode, false).await?;

    let instruction = build_action_context(action, user_state, dialect, formality, user_gender);
    let context_messages = match action {
        AIActionRequest::StartConversation => Vec::new(),
        AIActionRequest::ContinueBranch {
            parent_message_id, ..
        } => get_branch_messages_up_to(user_state, *parent_message_id),
        _ => unreachable!("Only Start/Continue handled"),
    };

    let history_vec = build_context_from_messages(&context_messages);
    let rag_config = RAGConfig::default_config();
    // Extract context: use injected context for ContinueBranch, otherwise derive from user_state
    let (
        mistakes,
        explained,
        translated,
        exploratory,
        learning_goals,
        active_plan,
        language_option,
        language_level,
    ) = match action {
        AIActionRequest::ContinueBranch { context, .. } => (
            context.past_mistakes.clone(),
            context.past_explained.clone(),
            context.past_translated.clone(),
            context.past_exploratory.clone(),
            context.learning_goals.clone(),
            context.active_plan.clone(),
            context.language_option,
            context.language_level,
        ),
        _ => {
            let active_plan = user_state.active_plan();
            let all_items =
                combine_plan_and_user_items(&user_state.learning_items, active_plan.as_ref());

            let (m, e, t, x) = extract_learning_items(&all_items, dialect);
            let goals = user_state
                .learning_goals
                .iter()
                .filter(|g| g.dialect == dialect)
                .cloned()
                .collect();
            (
                m,
                e,
                t,
                x,
                goals,
                user_state.active_plan(),
                user_state.current_language_option(),
                user_state.current_language_level(),
            )
        }
    };

    let params = GenerateResponseParams {
        user_message: &instruction,
        dialect: dialect_features(dialect),
        formality,
        teaching_mode,
        conversation_history: &history_vec,
        learning_goals: &learning_goals,
        rag_config: &rag_config,
        past_mistakes: &mistakes,
        past_explained: &explained,
        past_translated: &translated,
        past_exploratory: &exploratory,
        user_gender,
        language_option: &language_option,
        active_plan: active_plan.as_ref(),
        language_level,
    };

    let (result, response_usage) = state.agent.generate_response_for_action(&params).await;

    let usage_stats = AgentUsageStats {
        response_usage,
        learning_usage: Vec::new(),
        analysis_usage: Vec::new(),
    };
    let now = chrono::Utc::now().timestamp();
    user_state::update_and_save_usage(state, user_state.clone(), &usage_stats, now).await;

    result
}

pub async fn process_user_message(
    state: &AppState,
    msg_with_context: UserMessageWithContext,
    tx: &mpsc::UnboundedSender<String>,
) -> Result<(), ()> {
    let dialect = msg_with_context.message.metadata.dialect;
    tracing::info!("Processing message for dialect: {}", dialect.name());

    let context_vec = build_context_from_messages(&msg_with_context.context_messages);

    call_agent_and_respond(state, &msg_with_context, dialect, &context_vec, tx).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use dialect_coach_shared::MessageMetadata;
    use dialect_coach_shared::models::{Dialect, Formality, Language, TeachingMode};
    use uuid::Uuid;

    fn test_metadata(session_id: Uuid) -> MessageMetadata {
        MessageMetadata::at_now(
            Formality::Informal,
            TeachingMode::Immersive,
            Language::Spanish,
            Dialect::SpanishArgentinian,
            session_id,
        )
    }

    #[test]
    fn test_build_context_from_messages() {
        let session_id = Uuid::new_v4();

        let msg1 = Message::user_message("Hello".to_string(), test_metadata(session_id), None);

        let msg2 = Message::agent_message(
            AgentResponse::from("Hola"),
            test_metadata(session_id),
            Some(msg1.id),
        );

        let messages = vec![msg1.clone(), msg2.clone()];
        let context = build_context_from_messages(&messages);

        assert_eq!(context.len(), 2);
        assert_eq!(context[0], convert_to_rig_message(&msg1));
        assert_eq!(context[1], convert_to_rig_message(&msg2));
    }

    #[test]
    fn test_create_agent_response_message() {
        let session_id = Uuid::new_v4();
        let parent_id = Uuid::new_v4();
        let agent_response = AgentResponse::from("Test response");

        let msg = create_agent_response_message(
            agent_response.clone(),
            test_metadata(session_id),
            parent_id,
        );

        assert_eq!(msg.get_content(), "Test response");
        assert_eq!(msg.metadata.session_id, session_id);
        assert_eq!(msg.metadata.formality, Formality::Informal);
        assert_eq!(msg.metadata.teaching_mode, TeachingMode::Immersive);
        assert_eq!(msg.parent_id, Some(parent_id));
    }

    #[tokio::test]
    async fn test_message_parsing() {
        let content = AgentResponse::from("Hello");
        let message = Message::agent_message(content, test_metadata(Uuid::new_v4()), None);

        let json = serde_json::to_string(&message).unwrap();
        let parsed: Message = serde_json::from_str(&json).unwrap();

        assert_eq!(message.content, parsed.content);
    }

    #[test]
    fn test_dialect_from_message_metadata() {
        let msg = Message::agent_message(
            AgentResponse::from("Hola"),
            test_metadata(Uuid::new_v4()),
            None,
        );

        let dialect = msg.metadata.dialect;
        assert_eq!(dialect, Dialect::SpanishArgentinian);
    }
    #[test]
    fn test_extract_learning_items() {
        use dialect_coach_shared::models::learning_item::{LearningItem, LearningItemType};
        use dialect_coach_shared::models::{Explained, Mistake, MistakeCategory};

        let dialect = Dialect::SpanishMexican;
        let other_dialect = Dialect::SpanishArgentinian;

        let mistake = Mistake::new(
            "bad".to_string(),
            "good".to_string(),
            MistakeCategory::Other {
                context: "ctx".to_string(),
            },
        );
        let explained = Explained::new("phrase".to_string(), "trans".to_string());

        let items = vec![
            // Match dialect, Mistake
            LearningItem::new(LearningItemType::Mistake(mistake.clone()), dialect),
            // Match dialect, Explanation
            LearningItem::new(LearningItemType::Explanation(explained.clone()), dialect),
            // Wrong dialect
            LearningItem::new(LearningItemType::Mistake(mistake.clone()), other_dialect),
        ];

        let (m, e, t, x) = extract_learning_items(&items, dialect);

        assert_eq!(m.len(), 1);
        assert_eq!(e.len(), 1);
        assert_eq!(t.len(), 0);
        assert_eq!(x.len(), 0);

        assert_eq!(m[0].specific_mistake, "bad");
        assert_eq!(e[0].new_phrase, "phrase");
    }

    #[test]
    fn test_combine_plan_and_user_items() {
        use dialect_coach_shared::models::{
            Mistake, MistakeCategory, PlanContent, PlanStep, StepType,
        };

        let dialect = Dialect::SpanishMexican;
        let mistake1 = Mistake::new(
            "m1".to_string(),
            "c1".to_string(),
            MistakeCategory::Other {
                context: "ctx".to_string(),
            },
        );
        let item1 = LearningItem::new(LearningItemType::Mistake(mistake1.clone()), dialect);

        let mistake2 = Mistake::new(
            "m2".to_string(),
            "c2".to_string(),
            MistakeCategory::Other {
                context: "ctx".to_string(),
            },
        );
        let item2 = LearningItem::new(LearningItemType::Mistake(mistake2.clone()), dialect);

        // User has item1
        let user_items = vec![item1.clone()];

        // Plan has item1 AND item2
        let step = PlanStep::new(
            1,
            "Step 1".to_string(),
            StepType::Learning {
                content: PlanContent {
                    items: vec![item1.clone(), item2.clone()],
                },
            },
            "Inst".to_string(),
        );
        let plan = LanguagePlan::new("Plan".to_string(), dialect, None, vec![step]);

        let combined = combine_plan_and_user_items(&user_items, Some(&plan));

        assert_eq!(combined.len(), 2);
        // Should contain item1 and item2, without duplicating item1
        let ids: HashSet<Uuid> = combined.iter().map(get_item_id).collect();
        assert!(ids.contains(&mistake1.id));
        assert!(ids.contains(&mistake2.id));
    }
}
