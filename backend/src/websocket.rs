use axum::{
    extract::{
        State,
        ws::{Message as WsMessage, WebSocket, WebSocketUpgrade},
    },
    response::Response,
};
use dialect_coach_shared::{AgentResponse, Dialect, Formality, Message, TeachingMode, UserMessageWithContext, UserState, UserStateMessage};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::AppState;

fn error_to_agent_response(error_message: String) -> AgentResponse {
    AgentResponse::from(error_message)
}

fn trim_history(history: &mut Vec<String>, max_size: usize) {
    if history.len() > max_size {
        history.drain(0..history.len() - max_size);
    }
}

fn create_error_message(
    session_id: Uuid,
    error_text: String,
    language: String,
    formality: Formality,
    teaching_mode: TeachingMode,
) -> Message {
    let error_response = error_to_agent_response(error_text);
    Message::new(
        session_id,
        "system".to_string(),
        error_response,
        language,
        formality,
        teaching_mode,
    )
}

fn serialize_and_send(
    msg: &Message,
    tx: &mpsc::UnboundedSender<String>,
) -> Result<(), String> {
    let json = serde_json::to_string(msg)
        .map_err(|e| format!("Failed to serialize message: {}", e))?;

    tx.send(json)
        .map_err(|e| format!("Failed to send message: {}", e))?;

    Ok(())
}

async fn update_user_history(
    state: &AppState,
    session_id: Uuid,
    user_text: &str,
) -> Vec<String> {
    let mut histories = state.session_histories.lock().await;
    let history = histories.entry(session_id).or_insert_with(Vec::new);

    history.push(format!("User: {}", user_text));
    trim_history(history, 20);

    history.clone()
}

async fn add_agent_to_history(state: &AppState, session_id: Uuid, agent_text: &str) {
    let mut histories = state.session_histories.lock().await;
    if let Some(history) = histories.get_mut(&session_id) {
        history.push(format!("Agent: {}", agent_text));
    }
}

fn create_agent_response_message(
    session_id: Uuid,
    agent_response: AgentResponse,
    language: String,
    formality: Formality,
    teaching_mode: TeachingMode,
) -> Message {
    Message::new(
        session_id,
        "agent".to_string(),
        agent_response,
        language,
        formality,
        teaching_mode,
    )
}

async fn handle_agent_success(
    state: &AppState,
    parsed_msg: &Message,
    agent_response: AgentResponse,
    tx: &mpsc::UnboundedSender<String>,
) -> Result<(), String> {
    add_agent_to_history(state, parsed_msg.session_id, &agent_response.response).await;

    let response_msg = create_agent_response_message(
        parsed_msg.session_id,
        agent_response,
        parsed_msg.language.clone(),
        parsed_msg.metadata.formality,
        parsed_msg.metadata.teaching_mode,
    );

    serialize_and_send(&response_msg, tx)?;
    Ok(())
}

async fn handle_agent_error(
    parsed_msg: &Message,
    error: anyhow::Error,
    tx: &mpsc::UnboundedSender<String>,
) -> Result<(), String> {
    let error_msg = create_error_message(
        parsed_msg.session_id,
        format!("Error generating response: {}", error),
        parsed_msg.language.clone(),
        parsed_msg.metadata.formality,
        parsed_msg.metadata.teaching_mode,
    );

    serialize_and_send(&error_msg, tx)?;
    Ok(())
}

async fn validate_and_parse_dialect(
    parsed_msg: &Message,
    tx: &mpsc::UnboundedSender<String>,
) -> Result<Dialect, ()> {
    match Dialect::from_bcp47(&parsed_msg.language) {
        Some(d) => {
            tracing::info!(
                "Parsed dialect: {} from language tag: {}",
                d.name(),
                parsed_msg.language
            );
            Ok(d)
        }
        None => {
            tracing::error!("Unsupported language tag: {}", parsed_msg.language);
            let error_msg = create_error_message(
                parsed_msg.session_id,
                format!("Unsupported language/dialect: {}", parsed_msg.language),
                parsed_msg.language.clone(),
                parsed_msg.metadata.formality,
                parsed_msg.metadata.teaching_mode,
            );
            if let Err(e) = serialize_and_send(&error_msg, tx) {
                tracing::error!("{}", e);
            }
            Err(())
        }
    }
}

async fn run_agents_parallel(
    state: &AppState,
    msg_with_context: &UserMessageWithContext,
    dialect: Dialect,
    history_vec: &[String],
) -> Result<AgentResponse, anyhow::Error> {
    let formality = msg_with_context.message.metadata.formality;
    let teaching_mode = msg_with_context.message.metadata.teaching_mode;
    let user_text = &msg_with_context.message.content.response;

    let has_learning_items = !msg_with_context.past_mistakes.is_empty()
        || !msg_with_context.past_explained.is_empty()
        || !msg_with_context.past_translated.is_empty()
        || !msg_with_context.past_exploratory.is_empty();

    if !has_learning_items {
        return state
            .agent
            .generate_response(user_text, dialect, formality, teaching_mode, history_vec)
            .await;
    }

    tracing::info!(
        "Running agents in parallel: {} mistakes, {} explained, {} translated, {} exploratory items",
        msg_with_context.past_mistakes.len(),
        msg_with_context.past_explained.len(),
        msg_with_context.past_translated.len(),
        msg_with_context.past_exploratory.len()
    );

    let (response_result, analysis_result) = tokio::join!(
        state.agent.generate_response(user_text, dialect, formality, teaching_mode, history_vec),
        state.agent.generate_analysis(
            dialect,
            history_vec,
            &msg_with_context.past_mistakes,
            &msg_with_context.past_explained,
            &msg_with_context.past_translated,
            &msg_with_context.past_exploratory,
        )
    );

    let mut agent_response = response_result?;

    match analysis_result {
        Ok(analysis) => {
            agent_response.analysis = Some(analysis);
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
    history_vec: &[String],
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

    let history_vec = update_user_history(
        state,
        msg_with_context.message.session_id,
        &msg_with_context.message.content.response
    ).await;

    call_agent_and_respond(state, &msg_with_context, dialect, &history_vec, tx).await
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
                            "Valid message from {} in session {}: '{}' with {} mistakes, {} explained, {} translated, {} exploratory",
                            msg_with_context.message.participant_id,
                            msg_with_context.message.session_id,
                            msg_with_context.message.content.response,
                            msg_with_context.past_mistakes.len(),
                            msg_with_context.past_explained.len(),
                            msg_with_context.past_translated.len(),
                            msg_with_context.past_exploratory.len()
                        );

                        if process_user_message(&state, msg_with_context, &tx).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to parse message JSON: {}. Raw message: {}", e, text);
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
    tracing::info!("User state WebSocket connection established: {}", connection_id);

    let (sender, receiver) = socket.split();
    let (tx, rx) = mpsc::unbounded_channel::<String>();

    let send_task = create_user_state_send_task(sender, rx);
    run_user_state_receive_task(receiver, state, tx).await;

    let _ = send_task.await;
    tracing::info!("User state WebSocket connection closed: {}", connection_id);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trim_history() {
        let mut history = vec![];

        // Test: no trimming when under max
        for i in 0..10 {
            history.push(format!("msg {}", i));
        }
        trim_history(&mut history, 20);
        assert_eq!(history.len(), 10);

        // Test: trimming when over max
        for i in 10..25 {
            history.push(format!("msg {}", i));
        }
        trim_history(&mut history, 20);
        assert_eq!(history.len(), 20);
        assert_eq!(history[0], "msg 5"); // Oldest 5 removed
        assert_eq!(history[19], "msg 24"); // Newest kept
    }

    #[test]
    fn test_create_error_message() {
        let session_id = Uuid::new_v4();
        let error_msg = create_error_message(
            session_id,
            "Test error".to_string(),
            "es-MX".to_string(),
            Formality::Casual,
            TeachingMode::Immersive,
        );

        assert_eq!(error_msg.session_id, session_id);
        assert_eq!(error_msg.participant_id, "system");
        assert_eq!(error_msg.content.response, "Test error");
        assert_eq!(error_msg.language, "es-MX");
        assert_eq!(error_msg.metadata.formality, Formality::Casual);
        assert_eq!(error_msg.metadata.teaching_mode, TeachingMode::Immersive);
    }

    #[test]
    fn test_serialize_and_send() {
        let (tx, mut rx) = mpsc::unbounded_channel();

        let msg = Message::new(
            Uuid::new_v4(),
            "user1".to_string(),
            AgentResponse::from("Hello"),
            "es-MX".to_string(),
            Formality::Casual,
            TeachingMode::Immersive,
        );

        let result = serialize_and_send(&msg, &tx);
        assert!(result.is_ok());

        let received = rx.try_recv().unwrap();
        let parsed: Message = serde_json::from_str(&received).unwrap();
        assert_eq!(parsed.content.response, "Hello");
    }

    #[test]
    fn test_create_agent_response_message() {
        let session_id = Uuid::new_v4();
        let agent_response = AgentResponse::from("Test response");

        let msg = create_agent_response_message(
            session_id,
            agent_response.clone(),
            "es-MX".to_string(),
            Formality::DialectRich,
            TeachingMode::Corrective,
        );

        assert_eq!(msg.session_id, session_id);
        assert_eq!(msg.participant_id, "agent");
        assert_eq!(msg.content, agent_response);
        assert_eq!(msg.language, "es-MX");
        assert_eq!(msg.metadata.formality, Formality::DialectRich);
        assert_eq!(msg.metadata.teaching_mode, TeachingMode::Corrective);
    }

    #[tokio::test]
    async fn test_message_parsing() {
        let content = AgentResponse::from("Hello");
        let message = Message::new(
            Uuid::new_v4(),
            "user1".to_string(),
            content,
            "es-MX".to_string(),
            Formality::Casual,
            TeachingMode::Immersive,
        );

        let json = serde_json::to_string(&message).unwrap();
        let parsed: Message = serde_json::from_str(&json).unwrap();

        assert_eq!(message.content, parsed.content);
        assert_eq!(message.participant_id, parsed.participant_id);
    }

    #[tokio::test]
    async fn test_validate_and_parse_dialect_success() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let msg = Message::new(
            Uuid::new_v4(),
            "user1".to_string(),
            AgentResponse::from("Hola"),
            "es-MX".to_string(),
            Formality::Casual,
            TeachingMode::Immersive,
        );

        let result = validate_and_parse_dialect(&msg, &tx).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Dialect::SpanishMexican);
    }

    #[tokio::test]
    async fn test_validate_and_parse_dialect_failure() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let msg = Message::new(
            Uuid::new_v4(),
            "user1".to_string(),
            AgentResponse::from("Hello"),
            "invalid-tag".to_string(),
            Formality::Casual,
            TeachingMode::Immersive,
        );

        let result = validate_and_parse_dialect(&msg, &tx).await;
        assert!(result.is_err());

        let error_json = rx.try_recv().unwrap();
        let error_msg: Message = serde_json::from_str(&error_json).unwrap();
        assert_eq!(error_msg.participant_id, "system");
        assert!(error_msg.content.response.contains("Unsupported language/dialect"));
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
}
