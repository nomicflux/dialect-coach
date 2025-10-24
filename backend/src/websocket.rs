use axum::{
    extract::{
        State,
        ws::{Message as WsMessage, WebSocket, WebSocketUpgrade},
    },
    response::Response,
};
use dialect_coach_shared::{AgentResponse, Dialect, Formality, Message, TeachingMode};
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

async fn call_agent_and_respond(
    state: &AppState,
    parsed_msg: &Message,
    dialect: Dialect,
    history_vec: &[String],
    tx: &mpsc::UnboundedSender<String>,
) -> Result<(), ()> {
    let formality = parsed_msg.metadata.formality;
    let teaching_mode = parsed_msg.metadata.teaching_mode;

    tracing::info!(
        "Calling agent for dialect {} ({:?}, {:?}) with {} history messages",
        dialect.name(),
        formality,
        teaching_mode,
        history_vec.len()
    );

    match state
        .agent
        .generate_response(
            &parsed_msg.content.response,
            dialect,
            formality,
            teaching_mode,
            history_vec,
        )
        .await
    {
        Ok(agent_response) => {
            tracing::info!(
                "Agent generated response ({} chars)",
                agent_response.response.len()
            );
            handle_agent_success(state, parsed_msg, agent_response, tx)
                .await
                .map_err(|e| {
                    tracing::error!("{}", e);
                })
        }
        Err(e) => {
            tracing::error!("Agent error: {}", e);
            let _ = handle_agent_error(parsed_msg, e, tx).await;
            Ok(())
        }
    }
}

async fn process_user_message(
    state: &AppState,
    parsed_msg: Message,
    tx: &mpsc::UnboundedSender<String>,
) -> Result<(), ()> {
    let dialect = validate_and_parse_dialect(&parsed_msg, tx).await?;

    let history_vec = update_user_history(state, parsed_msg.session_id, &parsed_msg.content.response).await;

    call_agent_and_respond(state, &parsed_msg, dialect, &history_vec, tx).await
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

                match serde_json::from_str::<Message>(&text) {
                    Ok(parsed_msg) => {
                        tracing::info!(
                            "Valid message from {} in session {}: '{}'",
                            parsed_msg.participant_id,
                            parsed_msg.session_id,
                            parsed_msg.content.response
                        );

                        if process_user_message(&state, parsed_msg, &tx).await.is_err() {
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
}
