use axum::{
    extract::{
        State,
        ws::{Message as WsMessage, WebSocket, WebSocketUpgrade},
    },
    response::Response,
};
use dialect_coach_shared::models::dialect::dialect_features;
use dialect_coach_shared::{
    AIActionRequest, AgentResponse, AgentUsageStats, Dialect, Message, MessageContent,
    MessageMetadata, UserMessage, UserMessageWithContext, UserState, UserStateMessage, WsEvent,
};
use futures_util::{SinkExt, StreamExt};
use rig::completion::{
    Message as RigMessage, message::AssistantContent, message::Text, message::UserContent,
};
use rig::one_or_many::OneOrMany;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::AppState;
use crate::agent_service::{AgentService, response::GenerateResponseParams};
use crate::rag_config::RAGConfig;
use crate::rate_limiter::service::RateLimiterService;

fn auth_error_to_message(error: crate::auth_service::AuthError) -> String {
    use crate::auth_service::{AuthError, UnauthorizedReason};
    match error {
        AuthError::UserExists => "Username already taken".to_string(),
        AuthError::InvalidCredentials => "Invalid credentials".to_string(),
        AuthError::UserNotFound => "User not found".to_string(),
        AuthError::Unauthorized(UnauthorizedReason::InviteCodeExpired) => {
            "Invite code has expired".to_string()
        }
        AuthError::Unauthorized(UnauthorizedReason::InviteCodeInvalid) => {
            "Invalid invite code".to_string()
        }
        AuthError::Unauthorized(UnauthorizedReason::InviteCodeUsed) => {
            "Invite code already used".to_string()
        }
        AuthError::Persistence(msg) => format!("Authentication error: {}", msg),
    }
}

fn error_to_agent_response(error_message: String) -> AgentResponse {
    AgentResponse::from(error_message)
}

fn convert_to_rig_message(message: &Message) -> RigMessage {
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

fn build_context_from_messages(messages: &[Message]) -> Vec<RigMessage> {
    messages.iter().map(convert_to_rig_message).collect()
}

fn create_error_message(error_text: String, metadata: MessageMetadata) -> Message {
    let error_response = error_to_agent_response(error_text);
    Message::agent_message(error_response, metadata, None)
}

fn serialize_and_send(msg: &Message, tx: &mpsc::UnboundedSender<String>) -> Result<(), String> {
    let json =
        serde_json::to_string(msg).map_err(|e| format!("Failed to serialize message: {}", e))?;

    tx.send(json)
        .map_err(|e| format!("Failed to send message: {}", e))?;

    Ok(())
}

async fn add_agent_to_history(state: &AppState, session_id: Uuid, agent_text: &str) {
    let mut histories = state.session_histories.lock().await;
    if let Some(history) = histories.get_mut(&session_id) {
        history.push(format!("Agent: {}", agent_text));
    }
}

fn create_agent_response_message(
    agent_response: AgentResponse,
    metadata: MessageMetadata,
    parent_id: Uuid,
) -> Message {
    Message::agent_message(agent_response, metadata, Some(parent_id))
}

async fn handle_agent_success(
    state: &AppState,
    parsed_msg: &Message,
    agent_response: AgentResponse,
    tx: &mpsc::UnboundedSender<String>,
) -> Result<(), String> {
    add_agent_to_history(
        state,
        parsed_msg.metadata.session_id,
        &agent_response.response,
    )
    .await;

    let response_msg =
        create_agent_response_message(agent_response, parsed_msg.metadata.clone(), parsed_msg.id);

    serialize_and_send(&response_msg, tx)?;
    Ok(())
}

async fn handle_agent_error(
    parsed_msg: &Message,
    error: anyhow::Error,
    tx: &mpsc::UnboundedSender<String>,
) -> Result<(), String> {
    let error_msg = create_error_message(
        format!("Error generating response: {}", error),
        parsed_msg.metadata.clone(),
    );

    serialize_and_send(&error_msg, tx)?;
    Ok(())
}

async fn update_and_save_usage(
    state: &AppState,
    mut user_state: dialect_coach_shared::UserState,
    usage_stats: &AgentUsageStats,
    now: i64,
) {
    let user_id = user_state.user_id;
    log_response_usage(&user_id, &usage_stats.response_usage);
    crate::usage_tracker::add_response_usage(
        &mut user_state.usage_stats,
        usage_stats.response_usage.clone(),
        now,
        24,
    );

    if !usage_stats.learning_usage.is_empty() {
        log_learning_usage(&user_id, &usage_stats.learning_usage);
        crate::usage_tracker::add_learning_usage(
            &mut user_state.usage_stats,
            usage_stats.learning_usage.clone(),
            now,
            24,
        );
    }

    if !usage_stats.analysis_usage.is_empty() {
        log_analysis_usage(&user_id, &usage_stats.analysis_usage);
        crate::usage_tracker::add_analysis_usage(
            &mut user_state.usage_stats,
            usage_stats.analysis_usage.clone(),
            now,
            24,
        );
    }

    log_total_usage(&user_id, &user_state.usage_stats);
    let usage_stats = user_state.usage_stats.clone();
    save_usage_and_notify(state, user_state, usage_stats, user_id).await;
}

fn log_response_usage(
    user_id: &Uuid,
    usage: &[dialect_coach_shared::models::usage_stats::AgentUsage],
) {
    let input_tokens =
        dialect_coach_shared::models::usage_stats::AgentUsage::input_tokens_total(usage);
    let output_tokens =
        dialect_coach_shared::models::usage_stats::AgentUsage::output_tokens_total(usage);
    let retry_count = dialect_coach_shared::models::usage_stats::AgentUsage::retry_count(usage);
    let estimate_count =
        dialect_coach_shared::models::usage_stats::AgentUsage::estimate_count(usage);

    tracing::info!(
        user_id = %user_id,
        "Updating usage stats: {} response events ({} input, {} output tokens, {} retries, {} estimates)",
        usage.len(),
        input_tokens,
        output_tokens,
        retry_count,
        estimate_count
    );
}

fn log_analysis_usage(
    user_id: &Uuid,
    usage: &[dialect_coach_shared::models::usage_stats::AgentUsage],
) {
    let input_tokens =
        dialect_coach_shared::models::usage_stats::AgentUsage::input_tokens_total(usage);
    let output_tokens =
        dialect_coach_shared::models::usage_stats::AgentUsage::output_tokens_total(usage);
    let retry_count = dialect_coach_shared::models::usage_stats::AgentUsage::retry_count(usage);
    let estimate_count =
        dialect_coach_shared::models::usage_stats::AgentUsage::estimate_count(usage);

    tracing::info!(
        user_id = %user_id,
        "Updating usage stats: {} analysis events ({} input, {} output tokens, {} retries, {} estimates)",
        usage.len(),
        input_tokens,
        output_tokens,
        retry_count,
        estimate_count
    );
}

fn log_total_usage(user_id: &Uuid, stats: &dialect_coach_shared::UsageStats) {
    tracing::info!(
        user_id = %user_id,
        "Usage stats updated: {} total response events, {} total analysis events, {} total learning events",
        stats.response_count(),
        stats.analysis_count(),
        stats.learning_count()
    );
}

fn log_learning_usage(
    user_id: &Uuid,
    usage: &[dialect_coach_shared::models::usage_stats::AgentUsage],
) {
    let input_tokens =
        dialect_coach_shared::models::usage_stats::AgentUsage::input_tokens_total(usage);
    let output_tokens =
        dialect_coach_shared::models::usage_stats::AgentUsage::output_tokens_total(usage);
    let retry_count = dialect_coach_shared::models::usage_stats::AgentUsage::retry_count(usage);
    let estimate_count =
        dialect_coach_shared::models::usage_stats::AgentUsage::estimate_count(usage);

    tracing::info!(
        user_id = %user_id,
        "Updating usage stats: {} learning events ({} input, {} output tokens, {} retries, {} estimates)",
        usage.len(),
        input_tokens,
        output_tokens,
        retry_count,
        estimate_count
    );
}

fn log_branch_metadata(event: &str, state: &dialect_coach_shared::UserState) {
    let branch_count = state.branches.len();
    let active_branch_id = state.active_branch_id;
    let has_active_branch = state.branches.iter().any(|b| b.id == active_branch_id);
    let missing_leaf_count = state
        .branches
        .iter()
        .filter(|branch| branch.leaf_message_id.is_none())
        .count();

    tracing::info!(user_id = %state.user_id, event, branch_count, conversation_len = state.conversation_history.len(), has_active_branch, missing_leaf_count, "Branch metadata snapshot");

    if branch_count == 0 || !has_active_branch {
        tracing::error!(user_id = %state.user_id, event, branch_count, has_active_branch, "Branch metadata invalid");
    }
}

async fn save_usage_and_notify(
    state: &AppState,
    _user_state: dialect_coach_shared::UserState,
    usage_stats: dialect_coach_shared::UsageStats,
    user_id: Uuid,
) {
    match state
        .user_persistence
        .save_usage_stats(user_id, &usage_stats)
        .await
    {
        Ok(_) => {
            tracing::info!(user_id = %user_id, "Successfully saved usage stats");
            send_usage_stats_update(state, user_id, usage_stats).await;
        }
        Err(e) => tracing::error!(user_id = %user_id, "Failed to save usage stats: {}", e),
    }
}

async fn send_usage_stats_update(
    state: &AppState,
    user_id: Uuid,
    usage_stats: dialect_coach_shared::UsageStats,
) {
    let connections = state.user_state_connections.lock().await;
    if let Some(tx) = connections.get(&user_id) {
        let msg = UserStateMessage::UsageStatsUpdate(usage_stats);
        match send_user_state_message(&msg, tx) {
            Ok(_) => tracing::info!(user_id = %user_id, "Sent usage stats update to frontend"),
            Err(e) => {
                tracing::warn!(user_id = %user_id, "Failed to send usage stats update: {:?}", e)
            }
        }
    }
}

async fn register_user_state_connection(
    state: &AppState,
    user_id: Uuid,
    tx: &mpsc::UnboundedSender<String>,
) {
    let mut connections = state.user_state_connections.lock().await;
    connections.insert(user_id, tx.clone());
    tracing::info!(user_id = %user_id, "Registered user state WebSocket connection");
}

async fn load_user_state_response(state: &AppState, user_id: Uuid) -> UserStateMessage {
    match state.user_persistence.load(user_id).await {
        Ok(user_state) => create_load_response(user_id, user_state),
        Err(e) => {
            tracing::error!(user_id = %user_id, "Failed to load user state: {}", e);
            UserStateMessage::LoadResponse(None)
        }
    }
}

fn create_load_response(user_id: Uuid, user_state: Option<UserState>) -> UserStateMessage {
    match user_state {
        Some(mut state) => {
            if state.rebuild_branches_from_history() {
                tracing::warn!(user_id = %user_id, "Rebuilt branch metadata from conversation history");
            }
            log_branch_metadata("load_user_state", &state);
            log_usage_stats_snapshot(user_id, &state);
            UserStateMessage::LoadResponse(Some(state))
        }
        None => UserStateMessage::LoadResponse(None),
    }
}

fn log_usage_stats_snapshot(user_id: Uuid, state: &UserState) {
    tracing::info!(
        user_id = %user_id,
        "Loaded user state with usage stats: {} response events ({} input, {} output tokens), {} analysis events ({} input, {} output tokens), {} TTS events ({} characters)",
        state.usage_stats.response_count(),
        state.usage_stats.response_input_tokens(),
        state.usage_stats.response_output_tokens(),
        state.usage_stats.analysis_count(),
        state.usage_stats.analysis_input_tokens(),
        state.usage_stats.analysis_output_tokens(),
        state.usage_stats.tts_count(),
        state.usage_stats.tts_characters()
    );
}

fn prepare_user_state_for_save(user_state: &mut UserState) {
    if user_state.rebuild_branches_from_history() {
        tracing::warn!(user_id = %user_state.user_id, "Rebuilt branch metadata before save");
    }
    log_branch_metadata("save_user_state", user_state);
}

fn should_check_analysis_limit(
    teaching_mode: dialect_coach_shared::TeachingMode,
    needs_analysis: bool,
) -> bool {
    if !needs_analysis {
        return false;
    }
    teaching_mode != dialect_coach_shared::TeachingMode::Immersive
        && teaching_mode != dialect_coach_shared::TeachingMode::Debug
}

async fn check_rate_limits(
    state: &AppState,
    user_id: Uuid,
    teaching_mode: dialect_coach_shared::TeachingMode,
    needs_analysis: bool,
) -> Result<dialect_coach_shared::UserState, anyhow::Error> {
    let user_state = state
        .user_persistence
        .load(user_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("User state not found"))?;

    if !state.rate_limiter.anthropic_has_quota().await {
        return Err(anyhow::anyhow!("Anthropic quota exceeded"));
    }

    if !state
        .rate_limiter
        .can_make_response_call(&user_state.usage_stats, &state.rate_limit_config)
    {
        return Err(anyhow::anyhow!("Response agent rate limit exceeded"));
    }

    if should_check_analysis_limit(teaching_mode, needs_analysis)
        && !state
            .rate_limiter
            .can_make_analysis_call(&user_state.usage_stats, &state.rate_limit_config)
    {
        return Err(anyhow::anyhow!("Analysis agent rate limit exceeded"));
    }

    Ok(user_state)
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
            update_and_save_usage(state, user_state, usage_stats, now).await;
            Ok(agent_response)
        }
        (Ok(agent_response), Err(e)) => {
            tracing::error!("Analysis agent failed: {}", e);
            update_and_save_usage(state, user_state, usage_stats, now).await;
            Ok(agent_response)
        }
        (Err(e), _) => {
            update_and_save_usage(state, user_state, usage_stats, now).await;
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
    }
}

fn validate_user_message(user_text: &str) -> Result<(), anyhow::Error> {
    if AgentService::contains_illegal_characters(user_text) {
        tracing::error!("User message contains illegal characters (null bytes or control chars)");
        return Err(anyhow::anyhow!("User message contains illegal characters"));
    }
    Ok(())
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
    update_and_save_usage(state, user_state, &usage_stats, now).await;
    result
}

async fn run_agents_with_analysis(
    state: &AppState,
    params: &GenerateResponseParams<'_>,
    msg_with_context: &UserMessageWithContext,
    dialect: Dialect,
    user_state: dialect_coach_shared::UserState,
    now: i64,
) -> Result<AgentResponse, anyhow::Error> {
    tracing::info!(
        "Running agents in parallel: {} mistakes, {} explained, {} translated, {} exploratory items",
        msg_with_context.past_mistakes.len(),
        msg_with_context.past_explained.len(),
        msg_with_context.past_translated.len(),
        msg_with_context.past_exploratory.len()
    );

    let user_text = msg_with_context.message.get_content();
    let ((response_result, response_usage, learning_usage), (analysis_result, analysis_usage)) = tokio::join!(
        state.agent.generate_response(params),
        state.agent.generate_analysis(
            dialect,
            &user_text,
            &msg_with_context.past_mistakes,
            &msg_with_context.past_explained,
            &msg_with_context.past_translated,
            &msg_with_context.past_exploratory,
        )
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
    validate_user_message(&user_text)?;

    let has_learning_items = has_learning_items(msg_with_context);
    let teaching_mode = msg_with_context.message.metadata.teaching_mode;
    let user_state = check_rate_limits(
        state,
        msg_with_context.user_id,
        teaching_mode,
        has_learning_items,
    )
    .await?;

    let rag_config = RAGConfig::new(20, 5);
    let now = chrono::Utc::now().timestamp();
    let params = build_response_params(
        &user_text,
        msg_with_context,
        dialect,
        history_vec,
        &rag_config,
    );

    if !has_learning_items {
        run_response_only(state, &params, user_state, now).await
    } else {
        run_agents_with_analysis(state, &params, msg_with_context, dialect, user_state, now).await
    }
}

async fn call_agent_and_respond(
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

fn build_action_context(action: &AIActionRequest, user_state: &UserState) -> String {
    let dialect_name = user_state.current_dialect().name();
    let formality_name = user_state.formality.name();

    match action {
        AIActionRequest::StartConversation => {
            format!(
                "[System: Please greet the user in {} with {} formality level]",
                dialect_name, formality_name
            )
        }
        AIActionRequest::ContinueBranch { .. } => "[System: Continue the conversation]".to_string(),
        AIActionRequest::ExplainMessage { .. } => {
            "[System: Explain your previous response in simpler terms]".to_string()
        }
        AIActionRequest::TranslateMessage { .. } => {
            "[System: Provide word-by-word translation of your previous response]".to_string()
        }
    }
}

async fn process_ai_action_request(
    state: &AppState,
    user_id: Uuid,
    action: AIActionRequest,
    tx: &mpsc::UnboundedSender<String>,
) -> Result<(), ()> {
    tracing::info!(user_id = %user_id, "Processing AI action request: {:?}", action);

    let user_state = match load_user_state_for_action(state, user_id).await {
        Some(us) => us,
        None => {
            tracing::error!(user_id = %user_id, "User state not found for AI action");
            return Err(());
        }
    };

    let context = build_action_context(&action, &user_state);
    let session_id = Uuid::new_v4();
    let metadata = create_metadata_from_user_state(&user_state, session_id);

    let user_message = Message::user_message(context, metadata.clone(), None);
    let msg_with_context = create_ai_action_context(user_state.clone(), user_message.clone());

    let dialect = metadata.dialect;
    let context_vec = build_context_from_messages(&msg_with_context.context_messages);

    call_agent_and_respond(state, &msg_with_context, dialect, &context_vec, tx).await
}

fn create_metadata_from_user_state(user_state: &UserState, session_id: Uuid) -> MessageMetadata {
    MessageMetadata::at_now(
        user_state.formality,
        user_state.teaching_mode,
        user_state.current_dialect().language(),
        user_state.current_dialect(),
        session_id,
    )
}

async fn load_user_state_for_action(state: &AppState, user_id: Uuid) -> Option<UserState> {
    match state.user_persistence.load(user_id).await {
        Ok(Some(us)) => Some(us),
        Ok(None) => None,
        Err(e) => {
            tracing::error!(user_id = %user_id, "Failed to load user state: {}", e);
            None
        }
    }
}

fn create_ai_action_context(
    user_state: UserState,
    user_message: Message,
) -> UserMessageWithContext {
    use dialect_coach_shared::PastLearningItems;

    UserMessageWithContext::new(
        user_state.user_id,
        user_message,
        PastLearningItems {
            mistakes: vec![],
            explained: vec![],
            translated: vec![],
            exploratory: vec![],
        },
        user_state.active_branch_id,
        user_state.get_active_branch_messages().into_iter().cloned().collect(),
        user_state.learning_goals.clone(),
    )
}

async fn process_user_message(
    state: &AppState,
    msg_with_context: UserMessageWithContext,
    tx: &mpsc::UnboundedSender<String>,
) -> Result<(), ()> {
    let dialect = msg_with_context.message.metadata.dialect;
    tracing::info!("Processing message for dialect: {}", dialect.name());

    let context_vec = build_context_from_messages(&msg_with_context.context_messages);

    call_agent_and_respond(state, &msg_with_context, dialect, &context_vec, tx).await
}

fn create_send_task(
    mut sender: futures_util::stream::SplitSink<WebSocket, WsMessage>,
    mut rx: mpsc::UnboundedReceiver<String>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Err(e) = sender.send(WsMessage::Text(msg.into())).await {
                tracing::error!("Failed to send message to client: {}", e);
                break;
            }
        }
    })
}

fn create_recv_task(
    mut receiver: futures_util::stream::SplitStream<WebSocket>,
    state: AppState,
    tx: mpsc::UnboundedSender<String>,
    conn_id: Uuid,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let WsMessage::Text(text) = msg {
                tracing::debug!("Received message: {}", text);

                if try_parse_ws_event(&text, &state, &tx).await.is_err() {
                    try_parse_user_message(&text, &state, &tx).await;
                }
            } else if let WsMessage::Close(_) = msg {
                tracing::info!("Client closed connection: {}", conn_id);
                break;
            }
        }
    })
}

async fn try_parse_ws_event(
    text: &str,
    state: &AppState,
    tx: &mpsc::UnboundedSender<String>,
) -> Result<(), ()> {
    match serde_json::from_str::<WsEvent>(text) {
        Ok(WsEvent::RequestAIAction {
            user_id, action, ..
        }) => {
            tracing::info!(user_id = %user_id, "Received AI action request: {:?}", action);
            process_ai_action_request(state, user_id, action, tx).await
        }
        Ok(_) => {
            tracing::warn!("Received unsupported WsEvent type");
            Err(())
        }
        Err(_) => Err(()),
    }
}

async fn try_parse_user_message(text: &str, state: &AppState, tx: &mpsc::UnboundedSender<String>) {
    match serde_json::from_str::<UserMessageWithContext>(text) {
        Ok(msg_with_context) => {
            tracing::info!(
                "Valid message in session {}: '{}' with {} mistakes, {} explained, {} translated, {} exploratory",
                msg_with_context.message.metadata.session_id,
                msg_with_context.message.get_content(),
                msg_with_context.past_mistakes.len(),
                msg_with_context.past_explained.len(),
                msg_with_context.past_translated.len(),
                msg_with_context.past_exploratory.len()
            );

            let _ = process_user_message(state, msg_with_context, tx).await;
        }
        Err(e) => {
            tracing::error!("Failed to parse message JSON: {}. Raw message: {}", e, text);
        }
    }
}

/// WebSocket handler
pub async fn websocket_handler(State(state): State<AppState>, ws: WebSocketUpgrade) -> Response {
    tracing::info!("WebSocket upgrade request received");
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Handle individual WebSocket connection
async fn handle_socket(socket: WebSocket, state: AppState) {
    let connection_id = Uuid::new_v4();
    tracing::info!("WebSocket connection established: {}", connection_id);

    let (sender, receiver) = socket.split();
    let (tx, rx) = mpsc::unbounded_channel::<String>();

    let mut send_task = create_send_task(sender, rx);
    let mut recv_task = create_recv_task(receiver, state, tx.clone(), connection_id);

    tokio::select! {
        _ = (&mut send_task) => {
            recv_task.abort();
        }
        _ = (&mut recv_task) => {
            drop(tx);
            let _ = send_task.await;
        }
    }

    tracing::info!("WebSocket connection closed: {}", connection_id);
}

/// Handle save user state request
async fn handle_save_user_state(
    state: &AppState,
    user_state: UserState,
    tx: &mpsc::UnboundedSender<String>,
) -> Result<(), ()> {
    let mut user_state = user_state;
    prepare_user_state_for_save(&mut user_state);
    tracing::info!("Saving user state for user: {}", user_state.user_id);

    let response = match state.user_persistence.save(&user_state).await {
        Ok(_) => UserStateMessage::SaveResponse(Ok(())),
        Err(e) => {
            tracing::error!("Failed to save user state: {}", e);
            UserStateMessage::SaveResponse(Err(e.to_string()))
        }
    };

    send_user_state_message(&response, tx)
}

/// Handle load user state request
async fn handle_load_user_state(
    state: &AppState,
    user_id: Uuid,
    tx: &mpsc::UnboundedSender<String>,
) -> Result<(), ()> {
    tracing::info!(user_id = %user_id, "Loading user state");

    register_user_state_connection(state, user_id, tx).await;
    let response = load_user_state_response(state, user_id).await;
    send_user_state_message(&response, tx)
}

/// Send UserStateMessage through WebSocket
fn send_user_state_message(
    msg: &UserStateMessage,
    tx: &mpsc::UnboundedSender<String>,
) -> Result<(), ()> {
    if let UserStateMessage::LoadResponse(Some(user_state)) = msg {
        let user_id = user_state.user_id;
        let json = serde_json::to_string(msg)
            .map_err(|e| tracing::error!("Failed to serialize UserStateMessage: {}", e))?;
        let message_size = json.len();

        tracing::info!(
            user_id = %user_id,
            "Sending LoadResponse with usage stats: {} response events ({} input, {} output tokens), {} analysis events ({} input, {} output tokens), {} TTS events ({} characters), message size: {} bytes",
            user_state.usage_stats.response_count(),
            user_state.usage_stats.response_input_tokens(),
            user_state.usage_stats.response_output_tokens(),
            user_state.usage_stats.analysis_count(),
            user_state.usage_stats.analysis_input_tokens(),
            user_state.usage_stats.analysis_output_tokens(),
            user_state.usage_stats.tts_count(),
            user_state.usage_stats.tts_characters(),
            message_size
        );

        tx.send(json)
            .map_err(|e| tracing::error!("Failed to send UserStateMessage: {}", e))?;
    } else {
        let json = serde_json::to_string(msg)
            .map_err(|e| tracing::error!("Failed to serialize UserStateMessage: {}", e))?;

        tx.send(json)
            .map_err(|e| tracing::error!("Failed to send UserStateMessage: {}", e))?;
    }

    Ok(())
}

/// Process incoming user state message
async fn process_user_state_message(
    state: &AppState,
    text: &str,
    tx: &mpsc::UnboundedSender<String>,
) {
    match serde_json::from_str::<UserStateMessage>(text) {
        Ok(UserStateMessage::Save(user_state)) => {
            let _ = handle_save_user_state(state, user_state, tx).await;
        }
        Ok(UserStateMessage::Load(user_id)) => {
            let _ = handle_load_user_state(state, user_id, tx).await;
        }
        Ok(_) => {
            tracing::warn!("Received unexpected UserStateMessage variant from client");
        }
        Err(e) => {
            tracing::error!("Failed to parse UserStateMessage: {}", e);
        }
    }
}

/// Create send task for user state WebSocket
fn create_user_state_send_task(
    mut sender: futures_util::stream::SplitSink<WebSocket, WsMessage>,
    mut rx: mpsc::UnboundedReceiver<String>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(WsMessage::Text(msg.into())).await.is_err() {
                break;
            }
        }
    })
}

/// Create receive task for user state WebSocket
async fn run_user_state_receive_task(
    mut receiver: futures_util::stream::SplitStream<WebSocket>,
    state: AppState,
    tx: mpsc::UnboundedSender<String>,
) {
    while let Some(msg) = receiver.next().await {
        if let Ok(WsMessage::Text(text)) = msg {
            process_user_state_message(&state, &text, &tx).await;
        }
    }
    unregister_user_state_connection(&state, &tx).await;
}

async fn unregister_user_state_connection(state: &AppState, tx: &mpsc::UnboundedSender<String>) {
    let mut connections = state.user_state_connections.lock().await;
    let user_id_to_remove: Option<Uuid> = connections
        .iter()
        .find(|(_, sender)| sender.same_channel(tx))
        .map(|(user_id, _)| *user_id);
    if let Some(user_id) = user_id_to_remove {
        connections.remove(&user_id);
        tracing::info!(user_id = %user_id, "Unregistered user state WebSocket connection");
    }
}

/// Handle user state WebSocket connection
pub async fn user_state_websocket_handler(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> Response {
    tracing::info!("User state WebSocket upgrade request received");
    ws.on_upgrade(move |socket| handle_user_state_socket(socket, state))
}

/// Handle individual user state WebSocket connection
async fn handle_user_state_socket(socket: WebSocket, state: AppState) {
    let connection_id = Uuid::new_v4();
    tracing::info!(
        "User state WebSocket connection established: {}",
        connection_id
    );

    let (sender, receiver) = socket.split();
    let (tx, rx) = mpsc::unbounded_channel::<String>();

    let send_task = create_user_state_send_task(sender, rx);
    run_user_state_receive_task(receiver, state, tx).await;

    let _ = send_task.await;
    tracing::info!("User state WebSocket connection closed: {}", connection_id);
}

fn convert_auth_credentials(
    creds: dialect_coach_shared::AuthCredentials,
) -> crate::auth_service::AuthCredentials {
    match creds {
        dialect_coach_shared::AuthCredentials::InviteCode(code) => {
            tracing::info!(
                "Converting invite code, length: {}, content: '{}'",
                code.len(),
                code
            );
            crate::auth_service::AuthCredentials::Token(code)
        }
    }
}

/// Handle create user request
async fn handle_create_user(
    state: &AppState,
    username: String,
    email: String,
    credentials: dialect_coach_shared::AuthCredentials,
    tx: &mpsc::UnboundedSender<String>,
) -> Result<(), ()> {
    tracing::info!("Creating user: {} with email {}", username, email);

    let auth_creds = convert_auth_credentials(credentials);
    let response = match state
        .auth_service
        .create_user(username, email, auth_creds)
        .await
    {
        Ok(user) => UserMessage::CreateUserResponse(Ok(user)),
        Err(e) => {
            let error_msg =
                if let Some(auth_error) = e.downcast_ref::<crate::auth_service::AuthError>() {
                    auth_error_to_message(auth_error.clone())
                } else {
                    format!("Failed to create user: {}", e)
                };
            tracing::error!("{}", error_msg);
            UserMessage::CreateUserResponse(Err(error_msg))
        }
    };

    send_user_message(&response, tx)
}

/// Handle sign in request
async fn handle_sign_in(
    state: &AppState,
    username: String,
    tx: &mpsc::UnboundedSender<String>,
) -> Result<(), ()> {
    tracing::info!("Sign in request for user: {}", username);

    let response = match state.auth_service.authenticate(&username).await {
        Ok(user) => UserMessage::SignInResponse(Ok(user)),
        Err(e) => {
            let error_msg =
                if let Some(auth_error) = e.downcast_ref::<crate::auth_service::AuthError>() {
                    auth_error_to_message(auth_error.clone())
                } else {
                    format!("Authentication failed: {}", e)
                };
            tracing::warn!("{}", error_msg);
            UserMessage::SignInResponse(Err(error_msg))
        }
    };

    send_user_message(&response, tx)
}

/// Send UserMessage through WebSocket
fn send_user_message(msg: &UserMessage, tx: &mpsc::UnboundedSender<String>) -> Result<(), ()> {
    let json = serde_json::to_string(msg)
        .map_err(|e| tracing::error!("Failed to serialize UserMessage: {}", e))?;

    tx.send(json)
        .map_err(|e| tracing::error!("Failed to send UserMessage: {}", e))?;

    Ok(())
}

/// Process incoming user message
async fn process_user_message_ws(state: &AppState, text: &str, tx: &mpsc::UnboundedSender<String>) {
    match serde_json::from_str::<UserMessage>(text) {
        Ok(UserMessage::CreateUser {
            username,
            email,
            credentials,
        }) => {
            let _ = handle_create_user(state, username, email, credentials, tx).await;
        }
        Ok(UserMessage::SignIn { username }) => {
            let _ = handle_sign_in(state, username, tx).await;
        }
        Ok(_) => {
            tracing::warn!("Received unexpected UserMessage variant from client");
        }
        Err(e) => {
            tracing::error!("Failed to parse UserMessage: {}", e);
        }
    }
}

/// Create send task for user WebSocket
fn create_user_send_task(
    mut sender: futures_util::stream::SplitSink<WebSocket, WsMessage>,
    mut rx: mpsc::UnboundedReceiver<String>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(WsMessage::Text(msg.into())).await.is_err() {
                break;
            }
        }
    })
}

/// Run receive task for user WebSocket
async fn run_user_receive_task(
    mut receiver: futures_util::stream::SplitStream<WebSocket>,
    state: AppState,
    tx: mpsc::UnboundedSender<String>,
) {
    while let Some(msg) = receiver.next().await {
        if let Ok(WsMessage::Text(text)) = msg {
            process_user_message_ws(&state, &text, &tx).await;
        }
    }
}

/// Handle user WebSocket connection
async fn handle_user_socket(socket: WebSocket, state: AppState) {
    let connection_id = Uuid::new_v4();
    tracing::info!("User WebSocket connection established: {}", connection_id);

    let (sender, receiver) = socket.split();
    let (tx, rx) = mpsc::unbounded_channel::<String>();

    let send_task = create_user_send_task(sender, rx);
    run_user_receive_task(receiver, state, tx).await;

    let _ = send_task.await;
    tracing::info!("User WebSocket connection closed: {}", connection_id);
}

/// User WebSocket handler
pub async fn user_websocket_handler(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> Response {
    tracing::info!("User WebSocket upgrade request received");
    ws.on_upgrade(move |socket| handle_user_socket(socket, state))
}

#[cfg(test)]
mod tests {
    use super::*;
    use dialect_coach_shared::{Dialect, Formality, Language, TeachingMode, User};

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
    fn test_create_error_message() {
        let session_id = Uuid::new_v4();
        let error_msg = create_error_message("Test error".to_string(), test_metadata(session_id));

        assert_eq!(error_msg.get_content(), "Test error");
        assert_eq!(error_msg.metadata.session_id, session_id);
        assert_eq!(error_msg.metadata.formality, Formality::Informal);
        assert_eq!(error_msg.metadata.teaching_mode, TeachingMode::Immersive);
    }

    #[test]
    fn test_serialize_and_send() {
        let (tx, mut rx) = mpsc::unbounded_channel();

        let msg = Message::agent_message(
            AgentResponse::from("Hello"),
            test_metadata(Uuid::new_v4()),
            None,
        );

        let result = serialize_and_send(&msg, &tx);
        assert!(result.is_ok());

        let received = rx.try_recv().unwrap();
        let parsed: Message = serde_json::from_str(&received).unwrap();
        assert_eq!(parsed.get_content(), "Hello");
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
    fn test_send_user_state_message_ok() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let response = UserStateMessage::SaveResponse(Ok(()));

        let result = send_user_state_message(&response, &tx);
        assert!(result.is_ok());

        let json = rx.try_recv().unwrap();
        let parsed: UserStateMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, response);
    }

    #[test]
    fn test_send_user_state_message_err() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let response = UserStateMessage::SaveResponse(Err("Test error".to_string()));

        let result = send_user_state_message(&response, &tx);
        assert!(result.is_ok());

        let json = rx.try_recv().unwrap();
        let parsed: UserStateMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, response);
    }

    #[test]
    fn test_send_user_state_message_load_response() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let user_id = Uuid::new_v4();
        let user_state = UserState::new(user_id);
        let response = UserStateMessage::LoadResponse(Some(user_state.clone()));

        let result = send_user_state_message(&response, &tx);
        assert!(result.is_ok());

        let json = rx.try_recv().unwrap();
        let parsed: UserStateMessage = serde_json::from_str(&json).unwrap();
        if let UserStateMessage::LoadResponse(Some(state)) = parsed {
            assert_eq!(state.user_id, user_id);
        } else {
            panic!("Expected LoadResponse(Some)");
        }
    }

    #[test]
    fn test_send_user_message_create_user_response_ok() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let user = User::new(
            Uuid::new_v4(),
            "testuser".to_string(),
            "test@example.com".to_string(),
        );
        let response = UserMessage::CreateUserResponse(Ok(user.clone()));

        let result = send_user_message(&response, &tx);
        assert!(result.is_ok());

        let json = rx.try_recv().unwrap();
        let parsed: UserMessage = serde_json::from_str(&json).unwrap();
        if let UserMessage::CreateUserResponse(Ok(parsed_user)) = parsed {
            assert_eq!(parsed_user.username, "testuser");
        } else {
            panic!("Expected CreateUserResponse(Ok)");
        }
    }

    #[test]
    fn test_send_user_message_sign_in_response_err() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let response = UserMessage::SignInResponse(Err("User not found".to_string()));

        let result = send_user_message(&response, &tx);
        assert!(result.is_ok());

        let json = rx.try_recv().unwrap();
        let parsed: UserMessage = serde_json::from_str(&json).unwrap();
        if let UserMessage::SignInResponse(Err(err_msg)) = parsed {
            assert_eq!(err_msg, "User not found");
        } else {
            panic!("Expected SignInResponse(Err)");
        }
    }

    #[test]
    fn test_send_user_message_serialization() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let user = User::new(
            Uuid::new_v4(),
            "bob".to_string(),
            "bob@example.com".to_string(),
        );
        let response = UserMessage::SignInResponse(Ok(user.clone()));

        let result = send_user_message(&response, &tx);
        assert!(result.is_ok());

        let json = rx.try_recv().unwrap();
        let parsed: UserMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, response);
    }
}
