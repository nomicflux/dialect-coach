use axum::{
    extract::{
        State,
        ws::{Message as WsMessage, WebSocket, WebSocketUpgrade},
    },
    response::Response,
};
use dialect_coach_shared::{AgentUsageStats, MessageMetadata, UserState, UserStateMessage};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::AppState;
use crate::rate_limiter::service::RateLimiterService;

async fn register_user_state_connection(
    state: &AppState,
    user_id: Uuid,
    tx: &mpsc::UnboundedSender<String>,
) {
    let mut connections = state.user_state_connections.lock().await;
    connections.insert(user_id, tx.clone());
    tracing::info!(user_id = %user_id, "Registered user state WebSocket connection");
}

pub async fn send_usage_stats_update(
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

async fn handle_save_user_state(
    state: &AppState,
    user_state: UserState,
    tx: &mpsc::UnboundedSender<String>,
) -> Result<(), ()> {
    // DEBUG: Log plans received for save
    for plan in user_state.language_plans.iter() {
        tracing::info!(
            "Backend received plan to save: {} (id: {})",
            plan.title,
            plan.id
        );
    }

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

pub async fn user_state_websocket_handler(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> Response {
    tracing::info!("User state WebSocket upgrade request received");
    ws.on_upgrade(move |socket| handle_user_state_socket(socket, state))
}

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

pub async fn load_user_state_for_action(state: &AppState, user_id: Uuid) -> Option<UserState> {
    match state.user_persistence.load(user_id).await {
        Ok(Some(us)) => Some(us),
        Ok(None) => None,
        Err(e) => {
            tracing::error!(user_id = %user_id, "Failed to load user state: {}", e);
            None
        }
    }
}

pub fn create_metadata_from_user_state(
    user_state: &UserState,
    session_id: Uuid,
) -> MessageMetadata {
    MessageMetadata::at_now(
        user_state.formality,
        user_state.teaching_mode,
        user_state.current_dialect().language(),
        user_state.current_dialect(),
        session_id,
    )
}

pub async fn update_and_save_usage(
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

pub fn log_branch_metadata(event: &str, state: &dialect_coach_shared::UserState) {
    let branch_count = state.branches.len();
    let active_branch_id = state.active_branch_id;
    let has_active_branch = state.branches.iter().any(|b| b.id == active_branch_id);
    let missing_leaf_count = state
        .branches
        .iter()
        .filter(|branch| branch.leaf_message_id.is_none())
        .count();

    tracing::info!(user_id = %state.user_id, event, branch_count, conversation_len = state.conversation_history.len(), has_active_branch, missing_leaf_count, "Branch metadata snapshot");
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

pub fn log_usage_stats_snapshot(user_id: Uuid, state: &UserState) {
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

fn should_check_analysis_limit(
    teaching_mode: dialect_coach_shared::TeachingMode,
    needs_analysis: bool,
) -> bool {
    if !needs_analysis {
        return false;
    }
    teaching_mode != dialect_coach_shared::TeachingMode::Immersive
        && teaching_mode != dialect_coach_shared::TeachingMode::Debug
        && teaching_mode != dialect_coach_shared::TeachingMode::ErrorFinding
}

pub async fn check_rate_limits(
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

pub fn create_load_response(user_id: Uuid, user_state: Option<UserState>) -> UserStateMessage {
    match user_state {
        Some(state) => {
            // Removed perpetual migrations - no version gating meant they ran forever
            // If old data needs migration, frontend ReplaceUserState handles rebuild once
            log_branch_metadata("load_user_state", &state);
            log_usage_stats_snapshot(user_id, &state);
            UserStateMessage::LoadResponse(Some(state))
        }
        None => UserStateMessage::LoadResponse(None),
    }
}

pub fn prepare_user_state_for_save(user_state: &mut UserState) {
    // Removed all migrations - no version gating meant they ran forever
    // - migrate_branch_message_ids: Old tree format → message_ids list
    // - rebuild_branches_from_history: Prevented legitimate deletion of all branches
    log_branch_metadata("save_user_state", user_state);
}

pub async fn load_user_state_response(state: &AppState, user_id: Uuid) -> UserStateMessage {
    match state.user_persistence.load(user_id).await {
        Ok(Some(mut user_state)) => {
            // Populate is_admin from user record
            if let Ok(Some(user)) = state.user_persistence.load_user_by_id(user_id).await {
                user_state.is_admin = user.is_admin;
            }
            create_load_response(user_id, Some(user_state))
        }
        Ok(None) => create_load_response(user_id, None),
        Err(e) => {
            tracing::error!(user_id = %user_id, "Failed to load user state: {}", e);
            UserStateMessage::LoadResponse(None)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

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
}
