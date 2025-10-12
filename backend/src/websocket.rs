use axum::{
    extract::{
        ws::{Message as WsMessage, WebSocket, WebSocketUpgrade},
    },
    response::Response,
};
use dialect_coach_shared::Message;
use futures_util::{SinkExt, StreamExt};
use std::{
    collections::HashMap,
    sync::Arc,
};
use tokio::sync::{Mutex, mpsc};
use uuid::Uuid;

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
            if id != sender_id {
                if let Err(e) = tx.send(message.to_string()) {
                    tracing::warn!("Failed to send message to {}: {}", id, e);
                }
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
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(handle_socket)
}

/// Handle individual WebSocket connection
async fn handle_socket(socket: WebSocket) {
    let session_id = Uuid::new_v4();
    tracing::info!("WebSocket connection established: {}", session_id);

    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    // Spawn task to send messages to client
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(WsMessage::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    // Handle incoming messages
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let WsMessage::Text(text) = msg {
                tracing::debug!("Received message: {}", text);

                // Parse and validate message
                match serde_json::from_str::<Message>(&text) {
                    Ok(parsed_msg) => {
                        tracing::info!(
                            "Valid message from {} in session {}",
                            parsed_msg.participant_id,
                            parsed_msg.session_id
                        );

                        // TODO: Process message through RAG + agent
                        // For now, just echo back
                        if let Ok(response_json) = serde_json::to_string(&parsed_msg) {
                            if tx.send(response_json).is_err() {
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Invalid message format: {}", e);
                    }
                }
            } else if let WsMessage::Close(_) = msg {
                tracing::info!("Client closed connection: {}", session_id);
                break;
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = (&mut send_task) => {
            recv_task.abort();
        }
        _ = (&mut recv_task) => {
            send_task.abort();
        }
    }

    tracing::info!("WebSocket connection closed: {}", session_id);
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
