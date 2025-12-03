mod agents;
mod errors;
mod send;
mod user;
pub mod user_state;

use axum::{
    extract::{
        State,
        ws::{Message as WsMessage, WebSocket, WebSocketUpgrade},
    },
    response::Response,
};
use dialect_coach_shared::{UserMessage, WsEvent};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::AppState;

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

                // TODO: send last conversation message if not present.
                if try_parse_ws_event(&text, &state, &tx).await.is_err() {
                    tracing::error!("Error parsing WsEvent");
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
            user_id,
            action,
            session_id,
        }) => {
            tracing::info!(user_id = %user_id, "Received AI action request: {:?}", action);
            agents::process_ai_action_request(state, user_id, session_id, action, tx).await
        }
        Ok(WsEvent::UserMessage { user_message }) => {
            if agents::process_user_message(state, *user_message, tx)
                .await
                .is_err()
            {
                tracing::error!("Error processing user message");
                Err(())
            } else {
                Ok(())
            }
        }
        Err(_) => Err(()),
    }
}

pub async fn websocket_handler(State(state): State<AppState>, ws: WebSocketUpgrade) -> Response {
    tracing::info!("WebSocket upgrade request received");
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

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

async fn process_user_message_ws(state: &AppState, text: &str, tx: &mpsc::UnboundedSender<String>) {
    match serde_json::from_str::<UserMessage>(text) {
        Ok(UserMessage::CreateUser {
            username,
            email,
            credentials,
            password,
        }) => {
            let _ =
                user::handle_create_user(state, username, email, credentials, password, tx).await;
        }
        Ok(UserMessage::SignIn { username, password }) => {
            let _ = user::handle_sign_in(state, username, password, tx).await;
        }
        Ok(UserMessage::ValidateSession { token }) => {
            let _ = user::handle_validate_session(state, token, tx).await;
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

pub async fn user_websocket_handler(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> Response {
    tracing::info!("User WebSocket upgrade request received");
    ws.on_upgrade(move |socket| handle_user_socket(socket, state))
}
