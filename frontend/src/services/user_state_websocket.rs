use dialect_coach_shared::{UserState, UserStateMessage};
use futures_util::{SinkExt, StreamExt};
use gloo_net::websocket::{Message as WsMessage, futures::WebSocket};
use log::{error, info};
use std::cell::RefCell;
use std::rc::Rc;
use uuid::Uuid;
use wasm_bindgen_futures::spawn_local;
use yew::Callback;

/// Simple WebSocket service for user state persistence
pub struct UserStateWebSocketService {
    sender: Rc<RefCell<Option<futures_channel::mpsc::UnboundedSender<String>>>>,
    url: String,
    on_load_response: Callback<Option<UserState>>,
    on_save_response: Callback<Result<(), String>>,
    on_usage_stats_update: Callback<dialect_coach_shared::UsageStats>,
    on_open: Callback<()>,
}

impl UserStateWebSocketService {
    /// Create a new user state WebSocket service
    pub fn new(url: &str) -> Self {
        Self {
            sender: Rc::new(RefCell::new(None)),
            url: url.to_string(),
            on_load_response: Callback::noop(),
            on_save_response: Callback::noop(),
            on_usage_stats_update: Callback::noop(),
            on_open: Callback::noop(),
        }
    }

    /// Set callback for load responses
    pub fn set_on_load_response(&mut self, callback: Callback<Option<UserState>>) {
        self.on_load_response = callback;
    }

    /// Set callback for save responses
    pub fn set_on_save_response(&mut self, callback: Callback<Result<(), String>>) {
        self.on_save_response = callback;
    }

    /// Set callback for usage stats updates
    pub fn set_on_usage_stats_update(
        &mut self,
        callback: Callback<dialect_coach_shared::UsageStats>,
    ) {
        self.on_usage_stats_update = callback;
    }

    /// Set callback for connection open
    pub fn set_on_open(&mut self, callback: Callback<()>) {
        self.on_open = callback;
    }

    /// Connect to the WebSocket server
    pub fn connect(&mut self) {
        info!("Connecting to user state WebSocket at: {}", self.url);

        let ws = match WebSocket::open(&self.url) {
            Ok(ws) => ws,
            Err(e) => {
                error!("Failed to open user state WebSocket: {:?}", e);
                return;
            }
        };

        let (mut write, mut read) = ws.split();
        let (tx, mut rx) = futures_channel::mpsc::unbounded::<String>();
        *self.sender.borrow_mut() = Some(tx);

        let on_load = self.on_load_response.clone();
        let on_save = self.on_save_response.clone();
        let on_usage_stats_update = self.on_usage_stats_update.clone();
        let on_open = self.on_open.clone();

        // Spawn send task
        spawn_local(async move {
            while let Some(text) = rx.next().await {
                if let Err(e) = write.send(WsMessage::Text(text)).await {
                    error!("Failed to send user state message: {:?}", e);
                    break;
                }
            }
        });

        // Spawn receive task
        spawn_local(async move {
            info!("User state WebSocket connection established");
            on_open.emit(());

            while let Some(msg) = read.next().await {
                if let Ok(WsMessage::Text(text)) = msg {
                    process_message(&text, &on_load, &on_save, &on_usage_stats_update);
                }
            }

            info!("User state WebSocket connection closed");
        });
    }

    /// Save user state
    pub fn save_user_state(&self, user_state: &UserState) -> Result<(), String> {
        let msg = UserStateMessage::Save(user_state.clone());
        self.send_message(&msg)
    }

    /// Load user state
    pub fn load_user_state(&self, user_id: Uuid) -> Result<(), String> {
        let msg = UserStateMessage::Load(user_id);
        self.send_message(&msg)
    }

    /// Send a UserStateMessage
    fn send_message(&self, msg: &UserStateMessage) -> Result<(), String> {
        let json = serde_json::to_string(msg).map_err(|e| format!("Failed to serialize: {}", e))?;

        if let Some(sender) = self.sender.borrow().as_ref() {
            sender
                .unbounded_send(json)
                .map_err(|e| format!("Failed to send: {}", e))?;
            Ok(())
        } else {
            Err("WebSocket not connected".to_string())
        }
    }
}

/// Process incoming UserStateMessage
fn process_message(
    text: &str,
    on_load: &Callback<Option<UserState>>,
    on_save: &Callback<Result<(), String>>,
    on_usage_stats_update: &Callback<dialect_coach_shared::UsageStats>,
) {
    match serde_json::from_str::<UserStateMessage>(text) {
        Ok(msg) => handle_user_state_message(msg, on_load, on_save, on_usage_stats_update),
        Err(e) => error!("Failed to parse UserStateMessage: {}", e),
    }
}

fn handle_user_state_message(
    msg: UserStateMessage,
    on_load: &Callback<Option<UserState>>,
    on_save: &Callback<Result<(), String>>,
    on_usage_stats_update: &Callback<dialect_coach_shared::UsageStats>,
) {
    match msg {
        UserStateMessage::LoadResponse(user_state) => handle_load_response(user_state, on_load),
        UserStateMessage::SaveResponse(result) => handle_save_response(result, on_save),
        UserStateMessage::UsageStatsUpdate(usage_stats) => {
            handle_usage_stats_update(usage_stats, on_usage_stats_update)
        }
        _ => error!("Received unexpected UserStateMessage variant"),
    }
}

fn handle_save_response(result: Result<(), String>, on_save: &Callback<Result<(), String>>) {
    info!("Received SaveResponse: {:?}", result.is_ok());
    on_save.emit(result);
}

fn handle_load_response(user_state: Option<UserState>, on_load: &Callback<Option<UserState>>) {
    if let Some(ref state) = user_state {
        log_load_response(state);
    } else {
        info!("Received LoadResponse with no user state");
    }
    on_load.emit(user_state);
}

fn log_load_response(state: &UserState) {
    let user_id = state.user_id;
    info!(
        "Received LoadResponse for user {} with usage stats: {} response events ({} input, {} output tokens), {} analysis events ({} input, {} output tokens), {} TTS events ({} characters)",
        user_id,
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

fn handle_usage_stats_update(
    usage_stats: dialect_coach_shared::UsageStats,
    on_usage_stats_update: &Callback<dialect_coach_shared::UsageStats>,
) {
    log_usage_stats_update(&usage_stats);
    on_usage_stats_update.emit(usage_stats);
}

fn log_usage_stats_update(usage_stats: &dialect_coach_shared::UsageStats) {
    info!(
        "Received UsageStatsUpdate: {} response events ({} input, {} output tokens), {} analysis events ({} input, {} output tokens), {} TTS events ({} characters)",
        usage_stats.response_count(),
        usage_stats.response_input_tokens(),
        usage_stats.response_output_tokens(),
        usage_stats.analysis_count(),
        usage_stats.analysis_input_tokens(),
        usage_stats.analysis_output_tokens(),
        usage_stats.tts_count(),
        usage_stats.tts_characters()
    );
}
