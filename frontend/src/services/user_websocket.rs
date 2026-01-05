use dialect_coach_shared::{AuthCredentials, InitialUserSettings, User, UserMessage, UserState};
use futures_util::{SinkExt, StreamExt};
use gloo_net::websocket::{Message as WsMessage, futures::WebSocket};
use log::{error, info};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen_futures::spawn_local;
use yew::Callback;

/// WebSocket service for user management
pub struct UserWebSocketService {
    sender: Rc<RefCell<Option<futures_channel::mpsc::UnboundedSender<String>>>>,
    url: String,
    on_signin: Callback<Result<(User, UserState, String), String>>,
    on_open: Callback<()>,
}

impl UserWebSocketService {
    /// Create a new user WebSocket service
    pub fn new(url: &str) -> Self {
        Self {
            sender: Rc::new(RefCell::new(None)),
            url: url.to_string(),
            on_signin: Callback::noop(),
            on_open: Callback::noop(),
        }
    }

    /// Set callback for sign in responses (all auth flows)
    pub fn set_on_signin(&mut self, callback: Callback<Result<(User, UserState, String), String>>) {
        self.on_signin = callback;
    }

    /// Set callback for connection open
    pub fn set_on_open(&mut self, callback: Callback<()>) {
        self.on_open = callback;
    }

    /// Connect to the WebSocket server
    pub fn connect(&mut self) {
        info!("Connecting to user WebSocket at: {}", self.url);

        match WebSocket::open(&self.url) {
            Ok(ws) => start_connection_tasks(
                ws,
                self.sender.clone(),
                self.on_signin.clone(),
                self.on_open.clone(),
            ),
            Err(e) => error!("Failed to open user WebSocket: {:?}", e),
        }
    }

    /// Create a new user
    pub fn create_user(
        &self,
        username: String,
        email: String,
        credentials: AuthCredentials,
        password: String,
        initial_settings: Option<InitialUserSettings>,
    ) -> Result<(), String> {
        let msg = UserMessage::CreateUser {
            username,
            email,
            credentials,
            password,
            initial_settings,
        };
        self.send_message(&msg)
    }

    /// Sign in with username and password
    pub fn sign_in(&self, username: String, password: String) -> Result<(), String> {
        let msg = UserMessage::SignIn { username, password };
        self.send_message(&msg)
    }

    /// Send a UserMessage
    fn send_message(&self, msg: &UserMessage) -> Result<(), String> {
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

/// Process incoming UserMessage
fn process_message(
    text: &str,
    on_signin: &Callback<Result<(User, UserState, String), String>>,
) {
    match serde_json::from_str::<UserMessage>(text) {
        Ok(UserMessage::SignInResponse(result)) => {
            info!("Received SignInResponse: {:?}", result.is_ok());
            on_signin.emit(*result);
        }
        Ok(_) => {
            error!("Received unexpected UserMessage variant");
        }
        Err(e) => {
            error!("Failed to parse UserMessage: {}", e);
        }
    }
}

fn start_connection_tasks(
    ws: WebSocket,
    sender_ref: Rc<RefCell<Option<futures_channel::mpsc::UnboundedSender<String>>>>,
    on_signin: Callback<Result<(User, UserState, String), String>>,
    on_open: Callback<()>,
) {
    spawn_local(async move {
        use crate::services::websocket::wait_for_connection;
        if !wait_for_connection(&ws).await {
            error!("Failed to connect to user WebSocket");
            return;
        }

        let (mut write, mut read) = ws.split();
        let (tx, mut rx) = futures_channel::mpsc::unbounded::<String>();
        *sender_ref.borrow_mut() = Some(tx);

        spawn_local(async move {
            while let Some(text) = rx.next().await {
                if let Err(e) = write.send(WsMessage::Text(text)).await {
                    error!("Failed to send user message: {:?}", e);
                    break;
                }
            }
        });

        info!("User WebSocket connection established");
        on_open.emit(());

        while let Some(msg) = read.next().await {
            if let Ok(WsMessage::Text(text)) = msg {
                process_message(&text, &on_signin);
            }
        }
        info!("User WebSocket connection closed");
    });
}
