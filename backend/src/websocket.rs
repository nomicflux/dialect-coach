use axum::{
    extract::{
        ws::{Message as WsMessage, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use dialect_coach_shared::{Message, Dialect, Formality, TeachingMode};
use futures_util::{SinkExt, StreamExt};
use std::{
    collections::HashMap,
    sync::Arc,
};
use tokio::sync::{Mutex, mpsc};
use uuid::Uuid;

use crate::AppState;

/// Connection state manager
#[derive(Clone)]
pub struct ConnectionState {
    /// Active connections: session_id -> sender channel
    connections: Arc<Mutex<HashMap<Uuid, mpsc::UnboundedSender<String>>>>,
}

impl ConnectionState {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Add a new connection
    pub async fn add_connection(&self, session_id: Uuid, tx: mpsc::UnboundedSender<String>) {
        let mut connections = self.connections.lock().await;
        connections.insert(session_id, tx);
        tracing::info!("New connection added: {} (total: {})", session_id, connections.len());
    }

    /// Remove a connection
    pub async fn remove_connection(&self, session_id: &Uuid) {
        let mut connections = self.connections.lock().await;
        connections.remove(session_id);
        tracing::info!("Connection removed: {} (total: {})", session_id, connections.len());
    }

    /// Broadcast message to all connections except sender
    pub async fn broadcast(&self, sender_id: &Uuid, message: &str) {
        let connections = self.connections.lock().await;
        for (id, tx) in connections.iter() {
            if id != sender_id && let Err(e) = tx.send(message.to_string()) {
                tracing::warn!("Failed to send message to {}: {}", id, e);
            }
        }
    }

    /// Get connection count
    pub async fn connection_count(&self) -> usize {
        self.connections.lock().await.len()
    }
}

/// WebSocket handler
pub async fn websocket_handler(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> Response {
    eprintln!("[WEBSOCKET] Upgrade request received!");
    tracing::info!("WebSocket upgrade request received");
    let response = ws.on_upgrade(move |socket| handle_socket(socket, state));
    eprintln!("[WEBSOCKET] Returning upgrade response");
    response
}

/// Handle individual WebSocket connection
async fn handle_socket(socket: WebSocket, state: AppState) {
    let connection_id = Uuid::new_v4();
    eprintln!("[WEBSOCKET] Connection established: {}", connection_id);
    tracing::info!("WebSocket connection established: {}", connection_id);

    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    // Spawn task to send messages to client
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            eprintln!("[WEBSOCKET] send_task: Sending message to WebSocket");
            if let Err(e) = sender.send(WsMessage::Text(msg.into())).await {
                eprintln!("[WEBSOCKET] send_task: Failed to send: {}", e);
                tracing::error!("Failed to send message to client: {}", e);
                break;
            }
            eprintln!("[WEBSOCKET] send_task: Message sent successfully");
        }
        eprintln!("[WEBSOCKET] send_task: Channel closed, exiting");
    });

    // Clone state, tx, and connection_id for the receive task
    let state_clone = state.clone();
    let tx_clone = tx.clone();
    let conn_id = connection_id;

    // Handle incoming messages
    let mut recv_task = tokio::spawn(async move {
        let state = state_clone;
        let tx = tx_clone;
        while let Some(Ok(msg)) = receiver.next().await {
            if let WsMessage::Text(text) = msg {
                eprintln!("[WEBSOCKET] Received text message: {}", text);
                tracing::debug!("Received message: {}", text);

                // Parse and validate message
                match serde_json::from_str::<Message>(&text) {
                    Ok(parsed_msg) => {
                        eprintln!("[WEBSOCKET] Parsed message from {} in session {}: '{}'",
                            parsed_msg.participant_id, parsed_msg.session_id, parsed_msg.content);
                        tracing::info!(
                            "Valid message from {} in session {}: '{}'",
                            parsed_msg.participant_id,
                            parsed_msg.session_id,
                            parsed_msg.content
                        );

                        // Parse dialect from BCP-47 language tag
                        let dialect = match Dialect::from_bcp47(&parsed_msg.language) {
                            Some(d) => {
                                eprintln!("[WEBSOCKET] Parsed dialect: {} from language tag: {}", d.name(), parsed_msg.language);
                                tracing::info!("Parsed dialect: {} from language tag: {}", d.name(), parsed_msg.language);
                                d
                            }
                            None => {
                                eprintln!("[WEBSOCKET] ERROR: Unsupported language tag: {}", parsed_msg.language);
                                tracing::error!("Unsupported language tag: {}", parsed_msg.language);
                                let error_msg = Message::new(
                                    parsed_msg.session_id,
                                    "system".to_string(),
                                    format!("Unsupported language/dialect: {}", parsed_msg.language),
                                    parsed_msg.language.clone(),
                                );
                                if let Ok(error_json) = serde_json::to_string(&error_msg) && let Err(e) = tx.send(error_json) {
                                    tracing::error!("Failed to send error message: {}", e);
                                }
                                continue;
                            }
                        };

                        // Get or create session history
                        let mut histories = state.session_histories.lock().await;
                        let history = histories.entry(parsed_msg.session_id)
                            .or_insert_with(Vec::new);

                        // Add user message to history
                        history.push(format!("User: {}", parsed_msg.content));

                        // Keep only last 20 messages to avoid unbounded growth
                        if history.len() > 20 {
                            history.drain(0..history.len()-20);
                        }

                        // Clone history for agent call
                        let history_vec = history.clone();
                        drop(histories); // Release lock before agent call

                        // Extract formality and teaching mode from metadata (with defaults)
                        let formality = parsed_msg.metadata.formality.unwrap_or(Formality::Casual);
                        let teaching_mode = parsed_msg.metadata.teaching_mode.unwrap_or(TeachingMode::Immersive);

                        // Call agent with RAG
                        eprintln!("[WEBSOCKET] Calling agent for dialect {} ({:?}, {:?}) with {} history messages",
                            dialect.name(), formality, teaching_mode, history_vec.len());
                        tracing::info!("Calling agent for dialect {} ({:?}, {:?}) with {} history messages",
                            dialect.name(), formality, teaching_mode, history_vec.len());
                        match state.agent.generate_response(
                            &parsed_msg.content,
                            dialect,
                            formality,
                            teaching_mode,
                            &history_vec,
                        ).await {
                            Ok(agent_response) => {
                                eprintln!("[WEBSOCKET] Agent generated response ({} chars)", agent_response.len());
                                tracing::info!("Agent generated response ({} chars)", agent_response.len());

                                // Add agent response to history
                                let mut histories = state.session_histories.lock().await;
                                if let Some(history) = histories.get_mut(&parsed_msg.session_id) {
                                    history.push(format!("Agent: {}", agent_response));
                                }
                                drop(histories);

                                // Create response Message
                                let response_msg = Message::new(
                                    parsed_msg.session_id,
                                    "agent".to_string(),
                                    agent_response,
                                    parsed_msg.language.clone(),
                                );

                                // Send back to client
                                match serde_json::to_string(&response_msg) {
                                    Ok(response_json) => {
                                        eprintln!("[WEBSOCKET] Sending response to client");
                                        tracing::debug!("Sending response: {}", response_json);
                                        if let Err(e) = tx.send(response_json) {
                                            tracing::error!("Failed to send response to client: {}", e);
                                            break;
                                        }
                                        eprintln!("[WEBSOCKET] Response queued successfully");
                                    }
                                    Err(e) => {
                                        tracing::error!("Failed to serialize response message: {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("[WEBSOCKET] ERROR: Agent error: {}", e);
                                tracing::error!("Agent error: {}", e);
                                let error_msg = Message::new(
                                    parsed_msg.session_id,
                                    "system".to_string(),
                                    format!("Error generating response: {}", e),
                                    parsed_msg.language.clone(),
                                );
                                if let Ok(error_json) = serde_json::to_string(&error_msg) && let Err(e) = tx.send(error_json) {
                                    tracing::error!("Failed to send error message: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("[WEBSOCKET] ERROR: Failed to parse message JSON: {}. Raw message: {}", e, text);
                        tracing::error!("Failed to parse message JSON: {}. Raw message: {}", e, text);
                    }
                }
            } else if let WsMessage::Close(_) = msg {
                eprintln!("[WEBSOCKET] Client closed connection: {}", conn_id);
                tracing::info!("Client closed connection: {}", conn_id);
                break;
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = (&mut send_task) => {
            eprintln!("[WEBSOCKET] send_task finished first, aborting recv_task");
            recv_task.abort();
        }
        _ = (&mut recv_task) => {
            eprintln!("[WEBSOCKET] recv_task finished, dropping tx to signal send_task to finish");
            // Drop the tx channel so send_task can finish sending queued messages and exit gracefully
            drop(tx);
            // Wait for send_task to finish sending queued messages
            let _ = send_task.await;
        }
    }

    eprintln!("[WEBSOCKET] Connection closed: {}", connection_id);
    tracing::info!("WebSocket connection closed: {}", connection_id);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connection_state_add_remove() {
        let state = ConnectionState::new();
        let session_id = Uuid::new_v4();
        let (tx, _rx) = mpsc::unbounded_channel();

        assert_eq!(state.connection_count().await, 0);

        state.add_connection(session_id, tx).await;
        assert_eq!(state.connection_count().await, 1);

        state.remove_connection(&session_id).await;
        assert_eq!(state.connection_count().await, 0);
    }

    #[tokio::test]
    async fn test_message_parsing() {
        let message = Message::new(
            Uuid::new_v4(),
            "user1".to_string(),
            "Hello".to_string(),
            "es-MX".to_string(),
        );

        let json = serde_json::to_string(&message).unwrap();
        let parsed: Message = serde_json::from_str(&json).unwrap();

        assert_eq!(message.content, parsed.content);
        assert_eq!(message.participant_id, parsed.participant_id);
    }
}
