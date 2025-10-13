use dialect_coach_shared::Message;
use gloo_net::websocket::{Message as WsMessage, futures::WebSocket};
use std::rc::Rc;
use std::cell::RefCell;
use wasm_bindgen_futures::spawn_local;
use yew::Callback;
use futures_util::{SinkExt, StreamExt};
use futures_channel::mpsc;
use log::{info, error, warn};

/// WebSocket service for managing real-time communication with backend
pub struct WebSocketService {
    sender: Rc<RefCell<Option<mpsc::UnboundedSender<String>>>>,
    url: String,
    on_message: Callback<Message>,
    on_error: Callback<String>,
    on_close: Callback<()>,
    on_open: Callback<()>,
    is_connected: Rc<RefCell<bool>>,
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
            is_connected: Rc::new(RefCell::new(false)),
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

    /// Check if WebSocket is currently connected
    pub fn is_connected(&self) -> bool {
        *self.is_connected.borrow()
    }

    /// Connect to the WebSocket server
    pub fn connect(&mut self) {
        info!("Connecting to WebSocket at: {}", self.url);

        let ws = match WebSocket::open(&self.url) {
            Ok(ws) => ws,
            Err(e) => {
                error!("Failed to open WebSocket: {:?}", e);
                self.on_error.emit(format!("Failed to connect: {:?}", e));
                return;
            }
        };

        let (mut write, mut read) = ws.split();

        // Create channel for sending messages
        let (tx, mut rx) = mpsc::unbounded::<String>();
        *self.sender.borrow_mut() = Some(tx);

        // Clone callbacks for async context
        let on_message = self.on_message.clone();
        let on_error = self.on_error.clone();
        let on_close = self.on_close.clone();
        let on_open = self.on_open.clone();
        let is_connected = self.is_connected.clone();

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
        let is_connected_clone = is_connected.clone();
        spawn_local(async move {
            info!("WebSocket connection established");
            on_open.emit(());
            *is_connected_clone.borrow_mut() = true;

            while let Some(msg) = read.next().await {
                match msg {
                    Ok(WsMessage::Text(text)) => {
                        info!("Received WebSocket message: {} bytes", text.len());
                        match serde_json::from_str::<Message>(&text) {
                            Ok(parsed_msg) => {
                                info!("Parsed message from: {}", parsed_msg.participant_id);
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
            *is_connected.borrow_mut() = false;
            on_close.emit(());
        });
    }

    /// Send a message through the WebSocket
    pub fn send_message(&self, message: &Message) -> Result<(), String> {
        if !self.is_connected() {
            return Err("WebSocket is not connected".to_string());
        }

        let json = serde_json::to_string(message)
            .map_err(|e| format!("Failed to serialize message: {}", e))?;

        info!("Sending message: {} bytes", json.len());

        if let Some(sender) = self.sender.borrow().as_ref() {
            sender.unbounded_send(json)
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
        *self.is_connected.borrow_mut() = false;
    }
}

impl Drop for WebSocketService {
    fn drop(&mut self) {
        self.disconnect();
    }
}
