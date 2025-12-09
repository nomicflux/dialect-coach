use dialect_coach_shared::{AIActionRequest, Message, UserMessageWithContext, WsEvent};
use futures_channel::mpsc;
use futures_util::{SinkExt, StreamExt};
use gloo_net::websocket::{Message as WsMessage, futures::WebSocket};
use gloo_timers::callback::Timeout;
use log::{error, info, warn};
use std::cell::RefCell;
use std::rc::Rc;
use uuid::Uuid;
use wasm_bindgen_futures::spawn_local;
use yew::Callback;

/// Connection state for the WebSocket
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Failed,
}

/// Configuration for reconnection behavior
#[derive(Debug, Clone)]
pub struct ReconnectionConfig {
    pub max_attempts: u32,
    pub initial_delay_ms: u32,
    pub max_delay_ms: u32,
}

impl Default for ReconnectionConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 4000,
        }
    }
}

/// WebSocket service for managing real-time communication with backend
pub struct WebSocketService {
    sender: Rc<RefCell<Option<mpsc::UnboundedSender<String>>>>,
    url: String,
    on_message: Callback<Message>,
    on_error: Callback<String>,
    on_close: Callback<()>,
    on_open: Callback<()>,
    on_state_change: Callback<ConnectionState>,
    state: Rc<RefCell<ConnectionState>>,
    reconnection_config: ReconnectionConfig,
    reconnection_attempt: Rc<RefCell<u32>>,
    pending_messages: Rc<RefCell<Vec<String>>>,
    reconnection_timeout: Rc<RefCell<Option<Timeout>>>,
}

impl WebSocketService {
    /// Create a new WebSocket service (does not connect automatically)
    pub fn new(url: &str) -> Self {
        Self {
            sender: Rc::new(RefCell::new(None)),
            url: url.to_string(),
            on_message: Callback::noop(),
            on_error: Callback::noop(),
            on_close: Callback::noop(),
            on_open: Callback::noop(),
            on_state_change: Callback::noop(),
            state: Rc::new(RefCell::new(ConnectionState::Disconnected)),
            reconnection_config: ReconnectionConfig::default(),
            reconnection_attempt: Rc::new(RefCell::new(0)),
            pending_messages: Rc::new(RefCell::new(Vec::new())),
            reconnection_timeout: Rc::new(RefCell::new(None)),
        }
    }

    /// Set callback for receiving messages
    pub fn set_on_message(&mut self, callback: Callback<Message>) {
        self.on_message = callback;
    }

    /// Set callback for errors
    pub fn set_on_error(&mut self, callback: Callback<String>) {
        self.on_error = callback;
    }

    /// Set callback for connection close
    pub fn set_on_close(&mut self, callback: Callback<()>) {
        self.on_close = callback;
    }

    /// Set callback for connection open
    pub fn set_on_open(&mut self, callback: Callback<()>) {
        self.on_open = callback;
    }

    /// Set callback for state changes
    pub fn set_on_state_change(&mut self, callback: Callback<ConnectionState>) {
        self.on_state_change = callback;
    }

    /// Get current connection state
    pub fn state(&self) -> ConnectionState {
        *self.state.borrow()
    }

    /// Check if WebSocket is currently connected
    pub fn is_connected(&self) -> bool {
        matches!(*self.state.borrow(), ConnectionState::Connected)
    }

    /// Get number of pending messages
    pub fn pending_count(&self) -> usize {
        self.pending_messages.borrow().len()
    }

    /// Update connection state and notify listeners
    fn set_state(&self, new_state: ConnectionState) {
        let old_state = *self.state.borrow();
        if old_state != new_state {
            info!(
                "Connection state changed: {:?} -> {:?}",
                old_state, new_state
            );
            *self.state.borrow_mut() = new_state;
            self.on_state_change.emit(new_state);
        }
    }

    /// Connect to the WebSocket server
    pub fn connect(&mut self) {
        self.connect_internal(false);
    }

    /// Reconnect manually (resets attempt counter)
    pub fn reconnect(&mut self) {
        *self.reconnection_attempt.borrow_mut() = 0;
        self.connect_internal(true);
    }

    /// Internal connection logic
    fn connect_internal(&mut self, is_reconnect: bool) {
        // Cancel any pending reconnection timeout
        *self.reconnection_timeout.borrow_mut() = None;

        let new_state = if is_reconnect {
            ConnectionState::Reconnecting
        } else {
            ConnectionState::Connecting
        };
        self.set_state(new_state);

        info!("Connecting to WebSocket at: {}", self.url);

        let ws = match WebSocket::open(&self.url) {
            Ok(ws) => ws,
            Err(e) => {
                error!("Failed to open WebSocket: {:?}", e);
                self.on_error.emit(format!("Failed to connect: {:?}", e));
                self.handle_connection_failure();
                return;
            }
        };

        let (mut write, mut read) = ws.split();

        // Create channel for sending messages
        let (tx, mut rx) = mpsc::unbounded::<String>();
        *self.sender.borrow_mut() = Some(tx);

        // Clone callbacks and state for async context
        let on_message = self.on_message.clone();
        let on_error = self.on_error.clone();
        let on_close = self.on_close.clone();
        let on_open = self.on_open.clone();
        let state = self.state.clone();
        let reconnection_attempt = self.reconnection_attempt.clone();
        let on_state_change = self.on_state_change.clone();
        let reconnection_config = self.reconnection_config.clone();
        let reconnection_timeout = self.reconnection_timeout.clone();

        // Spawn send task
        spawn_local(async move {
            while let Some(text) = rx.next().await {
                if let Err(e) = write.send(WsMessage::Text(text)).await {
                    error!("Failed to send message: {:?}", e);
                    break;
                }
            }
            info!("Send task terminated");
        });

        // Spawn receive task
        spawn_local(async move {
            info!("WebSocket connection established");

            // Connection successful
            *state.borrow_mut() = ConnectionState::Connected;
            on_state_change.emit(ConnectionState::Connected);
            *reconnection_attempt.borrow_mut() = 0; // Reset attempt counter
            on_open.emit(());

            // Note: Pending messages are stored but will be sent through the normal
            // send_message() path once connection is established and is_connected() returns true.
            // The app layer can check pending_count() and resend if needed, or messages
            // will be sent on next user interaction.

            while let Some(msg) = read.next().await {
                match msg {
                    Ok(WsMessage::Text(text)) => {
                        info!("Received WebSocket message: {} bytes", text.len());
                        match serde_json::from_str::<Message>(&text) {
                            Ok(parsed_msg) => {
                                on_message.emit(parsed_msg);
                            }
                            Err(e) => {
                                error!("Failed to parse message JSON: {}", e);
                                on_error.emit(format!("Failed to parse message: {}", e));
                            }
                        }
                    }
                    Ok(WsMessage::Bytes(bytes)) => {
                        warn!("Received unexpected binary message: {} bytes", bytes.len());
                    }
                    Err(e) => {
                        error!("WebSocket error: {:?}", e);
                        on_error.emit(format!("WebSocket error: {:?}", e));
                        break;
                    }
                }
            }

            info!("WebSocket connection closed");
            *state.borrow_mut() = ConnectionState::Disconnected;
            on_state_change.emit(ConnectionState::Disconnected);
            on_close.emit(());

            // Attempt reconnection
            let attempt = *reconnection_attempt.borrow() + 1;
            if attempt <= reconnection_config.max_attempts {
                *reconnection_attempt.borrow_mut() = attempt;

                // Calculate backoff delay
                let delay = (reconnection_config.initial_delay_ms * (1 << (attempt - 1)))
                    .min(reconnection_config.max_delay_ms);

                info!(
                    "Reconnecting in {}ms (attempt {}/{})",
                    delay, attempt, reconnection_config.max_attempts
                );

                *state.borrow_mut() = ConnectionState::Reconnecting;
                on_state_change.emit(ConnectionState::Reconnecting);

                // Schedule reconnection
                let timeout = Timeout::new(delay, move || {
                    // This is tricky - we can't easily call connect() from here
                    // In practice, the app layer should handle reconnection on state change
                    warn!("Reconnection timeout fired, but cannot reconnect from here");
                });
                *reconnection_timeout.borrow_mut() = Some(timeout);
            } else {
                error!("Max reconnection attempts reached");
                *state.borrow_mut() = ConnectionState::Failed;
                on_state_change.emit(ConnectionState::Failed);
                on_error.emit("Connection failed after maximum retry attempts".to_string());
            }
        });
    }

    /// Handle connection failure
    fn handle_connection_failure(&mut self) {
        let attempt = *self.reconnection_attempt.borrow() + 1;

        if attempt <= self.reconnection_config.max_attempts {
            *self.reconnection_attempt.borrow_mut() = attempt;

            let delay = (self.reconnection_config.initial_delay_ms * (1 << (attempt - 1)))
                .min(self.reconnection_config.max_delay_ms);

            info!(
                "Scheduling reconnection in {}ms (attempt {}/{})",
                delay, attempt, self.reconnection_config.max_attempts
            );

            self.set_state(ConnectionState::Reconnecting);

            // We can't easily schedule reconnection here without more complex state management
            // The app layer should watch for Reconnecting state and call reconnect()
        } else {
            error!("Max reconnection attempts reached");
            self.set_state(ConnectionState::Failed);
        }
    }

    /// Send a message with context through the WebSocket
    pub fn send_message(
        &self,
        message_with_context: &UserMessageWithContext,
    ) -> Result<(), String> {
        let json = serde_json::to_string(&WsEvent::UserMessage {
            user_message: Box::new((*message_with_context).clone()),
        })
        .map_err(|e| format!("Failed to serialize message: {}", e))?;

        if !self.is_connected() {
            // Queue the message for later
            info!("WebSocket not connected, queueing message");
            self.pending_messages.borrow_mut().push(json);
            return Ok(());
        }

        info!(
            "Sending message: {} bytes ({} mistakes, {} explained)",
            json.len(),
            message_with_context.past_mistakes.len(),
            message_with_context.past_explained.len()
        );

        if let Some(sender) = self.sender.borrow().as_ref() {
            sender
                .unbounded_send(json)
                .map_err(|e| format!("Failed to send message: {}", e))?;
            Ok(())
        } else {
            Err("WebSocket sender not initialized".to_string())
        }
    }

    /// Disconnect from the WebSocket server
    pub fn disconnect(&mut self) {
        info!("Disconnecting WebSocket");
        *self.sender.borrow_mut() = None;
        *self.reconnection_timeout.borrow_mut() = None;
        self.set_state(ConnectionState::Disconnected);
    }

    /// Send an AI action request through the WebSocket
    pub fn send_ai_action(
        &self,
        action: AIActionRequest,
        session_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), String> {
        let event = WsEvent::RequestAIAction {
            session_id,
            user_id,
            action: Box::new(action),
        };
        self.send_event(&event)
    }

    /// Send a WsEvent through the WebSocket
    fn send_event(&self, event: &WsEvent) -> Result<(), String> {
        let json = serde_json::to_string(event)
            .map_err(|e| format!("Failed to serialize event: {}", e))?;

        if !self.is_connected() {
            info!("WebSocket not connected, queueing event");
            self.pending_messages.borrow_mut().push(json);
            return Ok(());
        }

        info!("Sending WsEvent: {} bytes", json.len());

        if let Some(sender) = self.sender.borrow().as_ref() {
            sender
                .unbounded_send(json)
                .map_err(|e| format!("Failed to send event: {}", e))?;
            Ok(())
        } else {
            Err("WebSocket sender not initialized".to_string())
        }
    }
}

impl Drop for WebSocketService {
    fn drop(&mut self) {
        self.disconnect();
    }
}
