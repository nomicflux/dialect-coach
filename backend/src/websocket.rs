use axum::{
    extract::{
        State,
        ws::{Message as WsMessage, WebSocket, WebSocketUpgrade},
    },
    response::Response,
};
use dialect_coach_shared::{
    AgentResponse, Dialect, Message, MessageContent, MessageMetadata, User, UserMessage,
    UserMessageWithContext, UserState, UserStateMessage,
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

async fn validate_and_parse_dialect(
    parsed_msg: &Message,
    tx: &mpsc::UnboundedSender<String>,
) -> Result<Dialect, ()> {
    match Dialect::from_bcp47(parsed_msg.metadata.dialect.bcp47_tag()) {
        Some(d) => {
            tracing::info!(
                "Parsed dialect: {} from language tag: {}",
                d.name(),
                parsed_msg.metadata.dialect.bcp47_tag()
            );
            Ok(d)
        }
        None => {
            tracing::error!(
                "Unsupported language tag: {}",
                parsed_msg.metadata.dialect.bcp47_tag()
            );
            let error_msg = create_error_message(
                format!(
                    "Unsupported language/dialect: {}",
                    parsed_msg.metadata.dialect.bcp47_tag()
                ),
                parsed_msg.metadata.clone(),
            );
            if let Err(e) = serialize_and_send(&error_msg, tx) {
                tracing::error!("{}", e);
            }
            Err(())
        }
    }
}

async fn update_and_save_usage(
    state: &AppState,
    user_id: Uuid,
    response_usage: Vec<dialect_coach_shared::models::usage_stats::AgentUsage>,
    analysis_usage: Vec<dialect_coach_shared::models::usage_stats::AgentUsage>,
    now: i64,
) {
    let mut user_state = match state.user_persistence.load(user_id).await {
        Ok(Some(s)) => s,
        Ok(None) | Err(_) => return,
    };

    crate::usage_tracker::add_response_usage(&mut user_state.usage_stats, response_usage, now, 24);
    if !analysis_usage.is_empty() {
        crate::usage_tracker::add_analysis_usage(
            &mut user_state.usage_stats,
            analysis_usage,
            now,
            24,
        );
    }

    let _ = state.user_persistence.save(&user_state).await;
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

async fn run_agents_parallel(
    state: &AppState,
    msg_with_context: &UserMessageWithContext,
    dialect: Dialect,
    history_vec: &[RigMessage],
) -> Result<AgentResponse, anyhow::Error> {
    let formality = msg_with_context.message.metadata.formality;
    let teaching_mode = msg_with_context.message.metadata.teaching_mode;
    let user_text = &msg_with_context.message.get_content();

    if AgentService::contains_illegal_characters(user_text) {
        tracing::error!("User message contains illegal characters (null bytes or control chars)");
        return Err(anyhow::anyhow!("User message contains illegal characters"));
    }

    let has_learning_items = !msg_with_context.past_mistakes.is_empty()
        || !msg_with_context.past_explained.is_empty()
        || !msg_with_context.past_translated.is_empty()
        || !msg_with_context.past_exploratory.is_empty();

    check_rate_limits(
        state,
        msg_with_context.user_id,
        teaching_mode,
        has_learning_items,
    )
    .await?;

    let rag_config = RAGConfig::new(20, 5);
    let now = chrono::Utc::now().timestamp();

    if !has_learning_items {
        let params = GenerateResponseParams {
            user_message: user_text,
            dialect,
            formality,
            teaching_mode,
            conversation_history: history_vec,
            learning_goals: &msg_with_context.learning_goals,
            rag_config: &rag_config,
        };
        let (agent_response, response_usage) = state.agent.generate_response(&params).await?;

        update_and_save_usage(state, msg_with_context.user_id, response_usage, vec![], now).await;

        return Ok(agent_response);
    }

    tracing::info!(
        "Running agents in parallel: {} mistakes, {} explained, {} translated, {} exploratory items",
        msg_with_context.past_mistakes.len(),
        msg_with_context.past_explained.len(),
        msg_with_context.past_translated.len(),
        msg_with_context.past_exploratory.len()
    );

    let params = GenerateResponseParams {
        user_message: user_text,
        dialect,
        formality,
        teaching_mode,
        conversation_history: history_vec,
        learning_goals: &msg_with_context.learning_goals,
        rag_config: &rag_config,
    };

    let (response_result, analysis_result) = tokio::join!(
        state.agent.generate_response(&params),
        state.agent.generate_analysis(
            dialect,
            user_text,
            &msg_with_context.past_mistakes,
            &msg_with_context.past_explained,
            &msg_with_context.past_translated,
            &msg_with_context.past_exploratory,
        )
    );

    let (agent_response, response_usage) = response_result?;

    match analysis_result {
        Ok((analysis, analysis_usage)) => {
            let mut agent_response = agent_response;
            agent_response.analysis = Some(analysis);

            update_and_save_usage(
                state,
                msg_with_context.user_id,
                response_usage,
                analysis_usage,
                now,
            )
            .await;

            Ok(agent_response)
        }
        Err(e) => {
            tracing::error!("Analysis agent failed: {}", e);
            Err(e)
        }
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

async fn process_user_message(
    state: &AppState,
    msg_with_context: UserMessageWithContext,
    tx: &mpsc::UnboundedSender<String>,
) -> Result<(), ()> {
    let dialect = validate_and_parse_dialect(&msg_with_context.message, tx).await?;

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

                match serde_json::from_str::<UserMessageWithContext>(&text) {
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

                        if process_user_message(&state, msg_with_context, &tx)
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to parse message JSON: {}. Raw message: {}",
                            e,
                            text
                        );
                    }
                }
            } else if let WsMessage::Close(_) = msg {
                tracing::info!("Client closed connection: {}", conn_id);
                break;
            }
        }
    })
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
    tracing::info!("Loading user state for user: {}", user_id);

    let response = match state.user_persistence.load(user_id).await {
        Ok(user_state) => UserStateMessage::LoadResponse(user_state),
        Err(e) => {
            tracing::error!("Failed to load user state: {}", e);
            UserStateMessage::LoadResponse(None)
        }
    };

    send_user_state_message(&response, tx)
}

/// Send UserStateMessage through WebSocket
fn send_user_state_message(
    msg: &UserStateMessage,
    tx: &mpsc::UnboundedSender<String>,
) -> Result<(), ()> {
    let json = serde_json::to_string(msg)
        .map_err(|e| tracing::error!("Failed to serialize UserStateMessage: {}", e))?;

    tx.send(json)
        .map_err(|e| tracing::error!("Failed to send UserStateMessage: {}", e))?;

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

/// Handle create user request
async fn handle_create_user(
    state: &AppState,
    user_id: Uuid,
    username: String,
    tx: &mpsc::UnboundedSender<String>,
) -> Result<(), ()> {
    tracing::info!("Creating user: {} with id {}", username, user_id);

    let user = User::new(user_id, username.clone());
    let response = match state.user_persistence.create_user(&user).await {
        Ok(_) => UserMessage::CreateUserResponse(Ok(user)),
        Err(e) => {
            tracing::error!("Failed to create user: {}", e);
            UserMessage::CreateUserResponse(Err(e.to_string()))
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

    let response = match state
        .user_persistence
        .load_user_by_username(&username)
        .await
    {
        Ok(Some(user)) => UserMessage::SignInResponse(Ok(user)),
        Ok(None) => {
            tracing::warn!("User not found: {}", username);
            UserMessage::SignInResponse(Err("User not found".to_string()))
        }
        Err(e) => {
            tracing::error!("Failed to load user: {}", e);
            UserMessage::SignInResponse(Err(e.to_string()))
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
        Ok(UserMessage::CreateUser { user_id, username }) => {
            let _ = handle_create_user(state, user_id, username, tx).await;
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
    use dialect_coach_shared::{Dialect, Formality, Language, TeachingMode};

    fn test_metadata(session_id: Uuid) -> MessageMetadata {
        MessageMetadata::at_now(
            Formality::Casual,
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
        assert_eq!(error_msg.metadata.formality, Formality::Casual);
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
        assert_eq!(msg.metadata.formality, Formality::Casual);
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

    #[tokio::test]
    async fn test_validate_and_parse_dialect_success() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let msg = Message::agent_message(
            AgentResponse::from("Hola"),
            test_metadata(Uuid::new_v4()),
            None,
        );

        let result = validate_and_parse_dialect(&msg, &tx).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Dialect::SpanishArgentinian);
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
        let user = User::new(Uuid::new_v4(), "testuser".to_string());
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
        let user = User::new(Uuid::new_v4(), "bob".to_string());
        let response = UserMessage::SignInResponse(Ok(user.clone()));

        let result = send_user_message(&response, &tx);
        assert!(result.is_ok());

        let json = rx.try_recv().unwrap();
        let parsed: UserMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, response);
    }
}
