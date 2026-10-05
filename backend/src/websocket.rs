mod agents;
mod errors;
pub(crate) mod registry;
mod send;
mod user;
mod user_state;

use std::future::Future;

use axum::{
    extract::{
        State,
        ws::{Message as WsMessage, WebSocket, WebSocketUpgrade},
    },
    response::Response,
};
use dialect_coach_shared::tts::TtsRequest;
use dialect_coach_shared::{
    AccountRequest, ClientEnvelope, ClientMessage, Reply, ServerMessage, SignInResult, SpeechAudio,
    StudyRequest, UsageStats,
};
use futures_util::{Sink, SinkExt, StreamExt, stream::SplitStream};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use uuid::Uuid;

use crate::{
    AppState, enrichment_handler, grammar_handler, planning_handler, translation_handler,
    tts_handler,
};

type Tx = UnboundedSender<String>;

pub async fn websocket_handler(State(state): State<AppState>, ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let connection_id = Uuid::new_v4();
    tracing::info!("WebSocket connection established: {}", connection_id);
    let (sink, stream) = socket.split();
    let (tx, rx) = mpsc::unbounded_channel();
    let chat = spawn_chat_worker(state.clone(), tx.clone());
    tokio::select! {
        _ = forward(sink, rx) => {}
        _ = receive(stream, &state, &tx, &chat) => {}
    }
    registry::release(&mut *state.connections.lock().await, &tx);
    tracing::info!("WebSocket connection closed: {}", connection_id);
}

/// Write every outgoing message to the socket until a write fails.
async fn forward<S>(mut sink: S, mut rx: UnboundedReceiver<String>)
where
    S: Sink<WsMessage> + Unpin,
{
    while let Some(text) = rx.recv().await {
        if sink.send(WsMessage::Text(text.into())).await.is_err() {
            tracing::info!("WebSocket write failed; closing connection");
            break;
        }
    }
}

async fn receive(
    mut stream: SplitStream<WebSocket>,
    state: &AppState,
    tx: &Tx,
    chat: &UnboundedSender<ClientEnvelope>,
) {
    while let Some(Ok(message)) = stream.next().await {
        if let WsMessage::Text(text) = message {
            match serde_json::from_str::<ClientEnvelope>(&text) {
                Ok(envelope) => dispatch(envelope, state, tx, chat).await,
                Err(e) => tracing::error!("Unparseable client message: {}", e),
            }
        }
    }
}

/// Coach requests go to the sequential chat worker, which keeps their replies in order.
/// Study aids each run in their own task. Everything else is answered here, one at a
/// time, so saves apply in the order sent.
async fn dispatch(
    envelope: ClientEnvelope,
    state: &AppState,
    tx: &Tx,
    chat: &UnboundedSender<ClientEnvelope>,
) {
    match envelope.body {
        ClientMessage::Chat(_) => chat
            .send(envelope)
            .expect("chat worker runs while its connection is open"),
        ClientMessage::Study(_) => {
            tokio::spawn(respond(envelope, state.clone(), tx.clone()));
        }
        _ => respond(envelope, state.clone(), tx.clone()).await,
    }
}

fn spawn_chat_worker(state: AppState, tx: Tx) -> UnboundedSender<ClientEnvelope> {
    let (chat, mut queue) = mpsc::unbounded_channel::<ClientEnvelope>();
    tokio::spawn(async move {
        while let Some(envelope) = queue.recv().await {
            respond(envelope, state.clone(), tx.clone()).await;
        }
    });
    chat
}

async fn respond(ClientEnvelope { id, body }: ClientEnvelope, state: AppState, tx: Tx) {
    let reply = run_supervised(answer(body, state, tx.clone())).await;
    send::send(&tx, &ServerMessage::Reply { id, body: reply });
}

/// Run a request in its own task so a panic becomes a `Failed` reply instead of no reply.
async fn run_supervised(request: impl Future<Output = Reply> + Send + 'static) -> Reply {
    tokio::spawn(request).await.unwrap_or_else(|e| {
        tracing::error!("Request task failed: {}", e);
        Reply::Failed("internal error".to_string())
    })
}

async fn answer(body: ClientMessage, state: AppState, tx: Tx) -> Reply {
    match body {
        ClientMessage::Account(request) => account(request, &state, &tx).await,
        ClientMessage::SaveUserState(user_state) => {
            Reply::UserStateSaved(user_state::save(&state, *user_state).await)
        }
        ClientMessage::Chat(request) => {
            Reply::Chat(Box::new(agents::answer(&state, request).await))
        }
        ClientMessage::Study(request) => study(request, &state).await,
    }
}

async fn study(request: StudyRequest, state: &AppState) -> Reply {
    match request {
        StudyRequest::Translate(r) => {
            Reply::Translated(translation_handler::translate(state, r).await)
        }
        StudyRequest::Grammar(r) => Reply::Grammar(grammar_handler::explain(state, r).await),
        StudyRequest::Enrich(r) => Reply::Enriched(enrichment_handler::enrich(state, r).await),
        StudyRequest::GeneratePlan(r) => {
            Reply::Plan(planning_handler::generate_plan(&state.planning_generator, r).await)
        }
        StudyRequest::Synthesize(r) => Reply::Speech(synthesize(state, r).await),
    }
}

async fn synthesize(state: &AppState, request: TtsRequest) -> Result<SpeechAudio, String> {
    match &state.tts {
        Some(tts) => tts_handler::synthesize(tts, request).await,
        None => Err("TTS service unavailable".to_string()),
    }
}

async fn account(request: AccountRequest, state: &AppState, tx: &Tx) -> Reply {
    match request {
        AccountRequest::SignIn { username, password } => {
            signed_in(state, tx, user::sign_in(state, username, password).await).await
        }
        AccountRequest::CreateUser(new_user) => {
            signed_in(state, tx, user::create_user(state, *new_user).await).await
        }
        AccountRequest::ValidateSession { token } => {
            signed_in(state, tx, user::validate_session(state, &token).await).await
        }
        AccountRequest::Reattach { token } => {
            reattached(state, tx, user::reattach(state, &token).await).await
        }
    }
}

/// A successful sign-in registers this connection for the user's usage pushes.
async fn signed_in(state: &AppState, tx: &Tx, result: SignInResult) -> Reply {
    if let Ok((user, _, _)) = &result {
        registry::bind(&mut *state.connections.lock().await, tx, user.id);
    }
    Reply::SignedIn(Box::new(result))
}

async fn reattached(
    state: &AppState,
    tx: &Tx,
    result: Result<(Uuid, String, UsageStats), String>,
) -> Reply {
    let renewed = match result {
        Ok((user_id, token, usage_stats)) => {
            registry::bind(&mut *state.connections.lock().await, tx, user_id);
            Ok((token, usage_stats))
        }
        Err(e) => Err(e),
    };
    Reply::Reattached(renewed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_forward_writes_messages_in_order_until_channel_closes() {
        let (tx, rx) = mpsc::unbounded_channel();
        tx.send("first".to_string()).unwrap();
        tx.send("second".to_string()).unwrap();
        drop(tx);
        let mut written: Vec<WsMessage> = Vec::new();

        forward(&mut written, rx).await;

        let expected = [
            WsMessage::Text("first".into()),
            WsMessage::Text("second".into()),
        ];
        assert_eq!(written, expected);
    }

    #[tokio::test]
    async fn test_run_supervised_returns_the_reply() {
        let reply = run_supervised(async { Reply::UserStateSaved(Ok(())) }).await;
        assert!(matches!(reply, Reply::UserStateSaved(Ok(()))));
    }

    #[tokio::test]
    async fn test_run_supervised_turns_panic_into_failed_reply() {
        let reply = run_supervised(async { panic!("request blew up") }).await;
        assert!(matches!(reply, Reply::Failed(e) if e == "internal error"));
    }
}
